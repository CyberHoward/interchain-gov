use abstract_adapter::objects::chain_name::ChainName;
use abstract_adapter::objects::module::ModuleInfo;
use abstract_adapter::sdk::{AbstractSdkResult, Execution, IbcInterface, TransferInterface};
use abstract_adapter::std::ibc::CallbackInfo;
use abstract_adapter::std::AbstractResult;
use abstract_adapter::traits::AbstractResponse;
use abstract_adapter::traits::ModuleIdentification;
use base64::Engine;
use cosmwasm_std::{
    coins, to_json_binary, CosmosMsg, DepsMut, Env, MessageInfo, Order, StdResult, Storage, SubMsg,
    WasmQuery,
};

use ibc_sync_state::DataState;
use neutron_query::gov::create_gov_proposal_keys;
use neutron_query::icq::IcqInterface;
use neutron_query::QueryType;

use crate::ibc_callbacks::{FINALIZE_CALLBACK_ID, PROPOSE_CALLBACK_ID, REGISTER_VOTE_ID};
use crate::msg::InterchainGovQueryMsg;
use crate::msg::{InterchainGovIbcCallbackMsg, InterchainGovIbcMsg};
use crate::state::{
    Governance, GovernanceVote, Members, Proposal, ProposalAction, ProposalId, ProposalMsg,
    ProposalOutcome, TallyResult, Vote, FINALIZED_PROPOSALS, GOV_VOTE_QUERIES, MEMBERS,
    MEMBERS_STATE_SYNC, PENDING_REPLIES, PROPOSAL_STATE_SYNC, TEMP_REMOTE_GOV_MODULE_ADDRS, VOTE,
    VOTE_RESULTS,
};
use crate::{
    contract::{AdapterResult, InterchainGov},
    msg::InterchainGovExecuteMsg,
    InterchainGovError,
};

pub fn execute_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    adapter: InterchainGov,
    msg: InterchainGovExecuteMsg,
) -> AdapterResult {
    match msg {
        InterchainGovExecuteMsg::Propose { proposal } => {
            propose(deps, env, info, adapter, proposal)
        }
        InterchainGovExecuteMsg::Finalize { prop_id } => {
            finalize(deps, env, info, adapter, prop_id)
        }
        InterchainGovExecuteMsg::VoteProposal {
            prop_id,
            vote,
            governance,
        } => do_vote(deps, env, adapter, prop_id, vote, governance),
        InterchainGovExecuteMsg::RequestVoteResults { prop_id } => {
            request_vote_results(deps, env, adapter, prop_id)
        }
        InterchainGovExecuteMsg::RequestGovVoteDetails { prop_id } => {
            request_gov_vote_details(deps, env, adapter, prop_id)
        }
        InterchainGovExecuteMsg::TestAddMembers { members } => {
            test_add_members(deps, adapter, members)
        }
        InterchainGovExecuteMsg::TemporaryRegisterRemoteGovModuleAddrs { modules } => {
            temporary_register_remote_gov_module_addrs(deps, adapter, modules)
        }
        InterchainGovExecuteMsg::Execute { prop_id } => execute_prop(deps, env, adapter, prop_id),
        _ => todo!(),
    }
}

// Execute a proposal after we got all the votes
fn execute_prop(
    deps: DepsMut,
    env: Env,
    app: InterchainGov,
    prop_id: String,
) -> Result<cosmwasm_std::Response, InterchainGovError> {
    // check existing vote results
    let existing_vote_results = VOTE_RESULTS
        .prefix(prop_id.clone())
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<(ChainName, Option<GovernanceVote>)>>>()?;

    // If we don't have any vote results, we need to query them
    if existing_vote_results.is_empty() {
        return Err(InterchainGovError::MissingVoteResults {
            prop_id: prop_id.clone(),
        });
    } else {
        // if we have pending votes, check they're all resolved
        if existing_vote_results.iter().any(|(_, vote)| vote.is_none()) {
            return Err(InterchainGovError::VotesStillPending {
                prop_id: prop_id.clone(),
                chains: existing_vote_results
                    .into_iter()
                    .map(|(chain, _)| chain.clone())
                    .collect(),
            });
        }
    }

    let mut votes_for: u8 = 0;
    let mut votes_against: u8 = 0;

    let this_vote = VOTE.load(deps.storage, prop_id.clone())?;
    if this_vote.vote == Vote::Yes {
        votes_for += 1;
    } else {
        votes_against += 1;
    }

    // Then get prop and check if it passed
    existing_vote_results
        .iter()
        .for_each(|(_, vote)| match vote {
            Some(vote) => {
                if vote.vote == Vote::Yes {
                    votes_for += 1;
                } else {
                    votes_against += 1;
                }
            }
            None => panic!("Vote Null checked before."),
        });

    let outcome = if votes_for > votes_against {
        ProposalOutcome {
            passed: true,
            votes_for,
            votes_against,
        }
    } else {
        ProposalOutcome {
            passed: false,
            votes_for,
            votes_against,
        }
    };

    let prop = PROPOSAL_STATE_SYNC.load(deps.storage, prop_id.clone())?.0;
    // TODO: store each vote per chain
    FINALIZED_PROPOSALS.save(
        deps.storage,
        prop_id.clone(),
        &(prop.clone(), outcome.clone()),
    )?;
    let external_members = MEMBERS_STATE_SYNC.external_members(deps.storage, &env)?;

    // Execute the prop
    match prop.action {
        ProposalAction::UpdateMembers { members } => {
            // If new members exclude self, update members to only be self
            if !members.members.contains(&ChainName::new(&env)) {
                MEMBERS_STATE_SYNC.save_members(deps.storage, &Members::new(&env))?;
            }
            MEMBERS_STATE_SYNC.save_members(deps.storage, &members)?;
        }
        ProposalAction::Signal => {}
    }

    // Send mgs to other members to report vote outcome

    PROPOSAL_STATE_SYNC
        .set_outstanding_finalization_acks(deps.storage, external_members.members.clone())?;

    let ibc_client = app.ibc_client(deps.as_ref());
    let exec_msg = InterchainGovIbcMsg::ProposalResult {
        prop_hash: prop_id.clone(),
        outcome: outcome,
    };
    let mut msgs = vec![];
    let target_module = this_module(&app)?;
    for host in external_members.members.iter() {
        let callback = CallbackInfo::new(
            PROPOSE_CALLBACK_ID,
            Some(to_json_binary(
                &InterchainGovIbcCallbackMsg::ProposalResult {
                    proposed_to: host.clone(),
                    prop_hash: prop_id.clone(),
                },
            )?),
        );
        msgs.push(ibc_client.module_ibc_action(
            host.to_string(),
            target_module.clone(),
            &exec_msg,
            Some(callback.clone()),
        )?);
    }
    return Ok(app
        .response("propose_members")
        .add_messages(msgs)
        .add_attribute("prop_id", prop_id));
}

/// Reach out to external members and request their local vote results
fn request_vote_results(
    deps: DepsMut,
    env: Env,
    app: InterchainGov,
    prop_id: ProposalId,
) -> AdapterResult {
    let (prop, state) = load_proposal(deps.storage, &prop_id)?;

    // We can only tally upon expiration
    if !prop.expiration.is_expired(&env.block) {
        return Err(InterchainGovError::ProposalStillOpen(prop_id.clone()));
    }

    // // Check whether the result is already finalized
    // if VOTES.may_load(deps.storage, prop_id.clone())?.is_some() {
    //     return Err(InterchainGovError::VotesAlreadyFinalized(prop_id.clone()));
    // }

    // First, make sure that all votes are in by checking whether we have vote results
    let existing_vote_results = VOTE_RESULTS
        .prefix(prop_id.clone())
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<(ChainName, Option<GovernanceVote>)>>>()?;

    // If we don't have any vote results, we need to query them
    if !existing_vote_results.is_empty() {
        // if we have pending votes, check they're all resolved
        return if existing_vote_results.iter().any(|(_, vote)| vote.is_none()) {
            Err(InterchainGovError::VotesStillPending {
                prop_id: prop_id.clone(),
                chains: existing_vote_results
                    .into_iter()
                    .map(|(chain, _)| chain.clone())
                    .collect(),
            })
        } else {
            // happy path error
            Err(InterchainGovError::VotesAlreadyFinalized(prop_id.clone()))
        };
    };

    // Ask everyone to give us their votes
    let external_members = load_external_members(deps.storage, &env)?;
    let vote_queries = external_members
        .iter()
        .map(|host| -> AbstractSdkResult<CosmosMsg> {
            let ibc_client = app.ibc_client(deps.as_ref());
            let module_addr = TEMP_REMOTE_GOV_MODULE_ADDRS.load(deps.storage, host)?;
            let query = ibc_client.ibc_query(
                host.to_string(),
                WasmQuery::Smart {
                    contract_addr: module_addr,
                    msg: to_json_binary(&crate::msg::QueryMsg::Module(InterchainGovQueryMsg::Vote {
                        prop_id: prop_id.clone(),
                    }))?,
                },
                CallbackInfo::new(REGISTER_VOTE_ID, None),
            )?;

            // Mark the vote result as pending
            VOTE_RESULTS.save(deps.storage, (prop_id.clone(), host), &None)?;

            Ok(query)
        })
        .collect::<AbstractSdkResult<Vec<CosmosMsg>>>()?;

    // let external_members = load_external_members(deps.storage, &env)?;
    Ok(app
        .response("query_vote_results")
        .add_attribute("prop_id", prop_id)
        .add_messages(vote_queries))
}

/// REuest external members actual governance vote details
fn request_gov_vote_details(
    deps: DepsMut,
    env: Env,
    app: InterchainGov,
    prop_id: ProposalId,
) -> AdapterResult {
    // check existing vote results
    let existing_vote_results = VOTE_RESULTS
        .prefix(prop_id.clone())
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<(ChainName, Option<GovernanceVote>)>>>()?;

    // If we don't have any vote results, we need to query them
    if existing_vote_results.is_empty() {
        return Err(InterchainGovError::MissingVoteResults {
            prop_id: prop_id.clone(),
        });
    } else {
        // if we have pending votes, check they're all resolved
        if existing_vote_results.iter().any(|(_, vote)| vote.is_none()) {
            return Err(InterchainGovError::VotesStillPending {
                prop_id: prop_id.clone(),
                chains: existing_vote_results
                    .into_iter()
                    .map(|(chain, _)| chain.clone())
                    .collect(),
            });
        }
    }

    // First, make sure that all votes are in by checking whether we have vote results
    let exsting_query_results = GOV_VOTE_QUERIES
        .prefix(prop_id.clone())
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<(ChainName, Option<TallyResult>)>>>()?;

    // If we don't have any vote results, we need to query them
    if !exsting_query_results.is_empty() {
        // if we have pending votes, check they're all resolved
        return if exsting_query_results.iter().any(|(_, vote)| vote.is_none()) {
            Err(InterchainGovError::GovVotesStillPending {
                prop_id: prop_id.clone(),
                chains: exsting_query_results
                    .into_iter()
                    .map(|(chain, _)| chain.clone())
                    .collect(),
            })
        } else {
            // happy path error
            Err(InterchainGovError::GovVotesAlreadyQueried(prop_id.clone()))
        };
    };

    // TODO: check pending replies (could be overwritten)
    // TODO: Only do ICQ for cosmos-sdk govs

    // let external_members = load_external_members(deps.storage, &env)?;
    // let query_sender = env.contract.address.clone();

    // loop through the external members and register interchain queries for their votes
    // These will call the sudo endpoint on our contract
    // let gov_queries = external_members.iter().enumerate().map(|(index, host)| {
    //     let icq = app.neutron_icq(deps.as_ref())?;
    //     let query = icq.register_interchain_query(&query_sender, host.clone(), QueryType::KV, create_gov_proposal_keys(vec![])?, vec![], 0)?;
    //     // store the query as pending
    //     GOV_VOTE_QUERIES.save(deps.storage, (prop_id.clone(), host), &None)?;
    //     let reply_id = index as u64;

    //     PENDING_REPLIES.save(deps.storage, reply_id, &(host.clone(), prop_id.clone()))?;

    //     Ok(SubMsg::reply_always(query, reply_id))
    // }).collect::<AbstractSdkResult<Vec<_>>>()?;

    // we should do this all at once
    // let withdraw_msg = app.bank(deps.as_ref()).transfer(coins(100000u128, "untrn"), &env.contract.address)?;
    // let withdraw_msg: CosmosMsg = app.executor(deps.as_ref()).execute(vec![withdraw_msg])?.into();

    // Ok(app.response("request_gov_vote_details").add_attribute("prop_id", prop_id).add_message(withdraw_msg).add_submessages(gov_queries))

    Ok(app.response("action"))
}

fn check_existing_votes<E: FnOnce() -> AdapterResult<()>>(
    deps: &DepsMut,
    prop_id: &ProposalId,
    has_all_votes_handler: E,
) -> AdapterResult<()> {
    // First, make sure that all votes are in by checking whether we have vote results

    Ok(())
}

fn load_proposal(
    storage: &mut dyn Storage,
    prop_id: &String,
) -> Result<(Proposal, Option<DataState>), InterchainGovError> {
    let (prop, _vote) = PROPOSAL_STATE_SYNC
        .load(storage, prop_id.clone())
        .map_err(|_| InterchainGovError::ProposalNotFound(prop_id.clone()))?;
    let data_state = PROPOSAL_STATE_SYNC.data_state(storage, prop_id.clone());
    Ok((prop, data_state))
}

// This currently allows for overriding votes
fn do_vote(
    deps: DepsMut,
    env: Env,
    app: InterchainGov,
    prop_id: String,
    vote: Vote,
    governance: Governance,
) -> AdapterResult {
    println!("Voting on proposal: {:?} with vote: {:?}", prop_id, vote);
    PROPOSAL_STATE_SYNC.assert_finalized(deps.storage, prop_id.clone())?;

    let (prop, _) = PROPOSAL_STATE_SYNC.load(deps.storage, prop_id.clone())?;
    if prop.expiration.is_expired(&env.block) {
        return Err(InterchainGovError::ProposalExpired(prop_id.clone()));
    }

    /*

    // TODO: THIS HARD_CODES A COSMOS SDK VOTE RESULT HERE, PROBABLY SHOULD INCLUDE IN PROPOSE?
    VOTE.save(deps.storage, prop_id.clone(), &GovernanceVote::new(Governance::CosmosSDK {
        proposal_id: 0,
    }, Vote::Yes))?;
     */

    // TODO: check governance?
    VOTE.save(
        deps.storage,
        prop_id.clone(),
        &GovernanceVote::new(governance, vote.clone()),
    )?;
    PROPOSAL_STATE_SYNC.finalize_kv_state(deps.storage, prop_id.clone(), Some((prop, vote)))?;

    Ok(app
        .response("vote_proposal")
        .add_attribute("prop_id", prop_id))
}

fn test_add_members(deps: DepsMut, app: InterchainGov, members: Members) -> AdapterResult {
    MEMBERS.update(deps.storage, |mut m| -> StdResult<Members> {
        m = members;
        Ok(m)
    })?;

    Ok(app.response("update_members"))
}

/// Propose a new message to the interchain DAO
/// 1. Create a new proposal
/// 2. Vote on our own proposal
/// 3. Propose to other chains, adding a callback to update the proposal's proposed state (to ensure they received it)
///
fn propose(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    app: InterchainGov,
    proposal: ProposalMsg,
) -> AdapterResult {
    // 1.
    let hash = <sha2::Sha256 as sha2::Digest>::digest(proposal.to_string());
    let prop_id = base64::prelude::BASE64_STANDARD.encode(hash.as_slice());
    let prop = Proposal::new(proposal.clone(), &info.sender, &env);

    // check that prop doesn't exist
    if PROPOSAL_STATE_SYNC.has(deps.storage, prop_id.clone()) {
        return Err(InterchainGovError::ProposalAlreadyExists(prop_id));
    }

    let external_members = MEMBERS_STATE_SYNC.external_members(deps.storage, &env)?;

    if external_members.members.is_empty() {
        // We have initiate state for the proposal, it will be proposed when we send it to other chains and get all confirmations
        PROPOSAL_STATE_SYNC.finalize_kv_state(
            deps.storage,
            prop_id.clone(),
            Some((prop.clone(), Vote::Yes)),
        )?;
        match prop.action {
            // If goal is to update members, send an IBC packet to members to update their state
            ProposalAction::UpdateMembers { mut members } => {
                MEMBERS_STATE_SYNC.initiate_members(deps.storage, &env, members.clone())?;
                // send msgs to new members
                let ibc_client = app.ibc_client(deps.as_ref());
                let exec_msg = InterchainGovIbcMsg::JoinGov {
                    members: members.clone(),
                };

                // filter out self
                members.members.retain(|c| c != &ChainName::new(&env));

                let mut msgs = vec![];
                let target_module = this_module(&app)?;
                for host in members.members.iter() {
                    let callback = CallbackInfo::new(
                        PROPOSE_CALLBACK_ID,
                        Some(to_json_binary(&InterchainGovIbcCallbackMsg::JoinGov {
                            proposed_to: host.clone(),
                        })?),
                    );
                    msgs.push(ibc_client.module_ibc_action(
                        host.to_string(),
                        target_module.clone(),
                        &exec_msg,
                        Some(callback.clone()),
                    )?);
                }

                return Ok(app
                    .response("propose_members")
                    .add_messages(msgs)
                    .add_attribute("prop_id", prop_id));
            }
            ProposalAction::Signal => {
                return Ok(app
                    .response("propose_and_accept")
                    .add_attribute("prop_id", prop_id))
            }
        }
    };

    // Here we have to account for all other instances of the app.

    // 2.
    // We have initiate state for the proposal, it will be proposed when we send it to other chains and get all confirmations
    PROPOSAL_STATE_SYNC.initiate_kv_state(
        deps.storage,
        prop_id.clone(),
        (prop.clone(), Vote::Yes),
        external_members.members.clone(),
    )?;

    // 3.
    let target_module = this_module(&app)?;

    // Loop through members and propose to them the proposal (TODO: do we actually need ourselves stored)?
    let propose_msgs = external_members
        .members
        .iter()
        .map(|host| {
            let callback = CallbackInfo::new(
                PROPOSE_CALLBACK_ID,
                Some(to_json_binary(
                    &InterchainGovIbcCallbackMsg::ProposeProposal {
                        prop_hash: prop_id.clone(),
                        proposed_to: host.clone(),
                    },
                )?),
            );
            let ibc_client = app.ibc_client(deps.as_ref());
            let msg = ibc_client.module_ibc_action(
                host.to_string(),
                target_module.clone(),
                &InterchainGovIbcMsg::ProposeProposal {
                    prop: prop.clone(),
                    prop_hash: prop_id.clone(),
                    chain: host.clone(),
                },
                Some(callback.clone()),
            )?;
            Ok(msg)
        })
        .collect::<AbstractSdkResult<Vec<CosmosMsg>>>()?;

    Ok(app
        .response("propose")
        .add_attribute("prop_id", prop_id)
        .add_messages(propose_msgs))
}

/// Helper to load external members
fn load_external_members(storage: &mut dyn Storage, env: &Env) -> AdapterResult<Vec<ChainName>> {
    Ok(MEMBERS
        .load(storage)?
        .members
        .into_iter()
        .filter(|m| m != &ChainName::new(env))
        .collect::<Vec<_>>())
}

/// Send finalization message over IBC
pub fn finalize(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    app: InterchainGov,
    prop_id: ProposalId,
) -> AdapterResult {
    // State is already finalized after receiving the final proposal.

    let target_module = this_module(&app)?;

    let external_members = MEMBERS_STATE_SYNC.external_members(deps.storage, &env)?;

    // set outstanding acks
    PROPOSAL_STATE_SYNC
        .set_outstanding_finalization_acks(deps.storage, external_members.members.clone())?;

    let ibc_client = app.ibc_client(deps.as_ref());
    let finalize_messages = external_members
        .members
        .iter()
        .map(|host| {
            let callback = CallbackInfo::new(
                FINALIZE_CALLBACK_ID,
                Some(to_json_binary(
                    &InterchainGovIbcCallbackMsg::FinalizeProposal {
                        prop_hash: prop_id.clone(),
                        proposed_to: host.clone(),
                    },
                )?),
            );
            let msg = ibc_client.module_ibc_action(
                host.to_string(),
                target_module.clone(),
                &InterchainGovIbcMsg::FinalizeProposal {
                    prop_hash: prop_id.clone(),
                },
                Some(callback.clone()),
            )?;

            Ok(msg)
        })
        .collect::<AbstractSdkResult<Vec<CosmosMsg>>>()?;
    
    Ok(app
        .response("finalize")
        .add_attribute("prop_id", prop_id)
        .add_messages(finalize_messages))
}

fn this_module(app: &InterchainGov) -> AbstractResult<ModuleInfo> {
    ModuleInfo::from_id(app.module_id(), app.version().into())
}

fn temporary_register_remote_gov_module_addrs(
    deps: DepsMut,
    app: InterchainGov,
    modules: Vec<(ChainName, String)>,
) -> AdapterResult {
    for (chain, addr) in modules {
        TEMP_REMOTE_GOV_MODULE_ADDRS.save(deps.storage, &chain, &addr)?;
    }

    Ok(app.response("register_remote_gov_module_addrs"))
}
