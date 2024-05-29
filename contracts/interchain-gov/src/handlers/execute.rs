use abstract_adapter::objects::chain_name::ChainName;
use abstract_adapter::objects::module::ModuleInfo;
use abstract_adapter::sdk::{AbstractSdkResult, IbcInterface};
use abstract_adapter::sdk::base::{ModuleIbcEndpoint};
use abstract_adapter::std::AbstractResult;
use abstract_adapter::std::ibc::CallbackInfo;
use abstract_adapter::std::ibc_client::state::IBC_INFRA;
use abstract_adapter::traits::AbstractResponse;
use abstract_adapter::traits::ModuleIdentification;
use cosmwasm_std::{CosmosMsg, DepsMut, ensure_eq, Env, MessageInfo, Order, StdResult, Storage, to_json_binary, to_json_string, WasmQuery};

use crate::{
    contract::{AdapterResult, InterchainGov},
    InterchainGovError,
    msg::InterchainGovExecuteMsg,
};
use crate::handlers::instantiate::{get_item_state, propose_item_state};
use crate::ibc_callbacks::{REGISTER_VOTE_ID, PROPOSE_CALLBACK_ID};
use crate::msg::{InterchainGovIbcMsg, InterchainGovQueryMsg};
use crate::state::{DataState, VOTE, MEMBERS, Proposal, REMOTE_PROPOSAL_STATE, ProposalId, ProposalMsg, PROPOSALS, Vote, Members, VOTES, Governance, TEMP_REMOTE_GOV_MODULE_ADDRS, GovernanceVote, VOTE_RESULTS};

pub fn execute_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    adapter: InterchainGov,
    msg: InterchainGovExecuteMsg,
) -> AdapterResult {
    match msg {
        InterchainGovExecuteMsg::Propose { proposal } => propose(deps, env, info, adapter, proposal),
        InterchainGovExecuteMsg::Finalize { prop_id } => finalize(deps, env, info, adapter, prop_id),
        InterchainGovExecuteMsg::InviteMember { member } => invite_member(deps, adapter, member),
        InterchainGovExecuteMsg::VoteProposal { prop_id, vote, governance } => do_vote(deps, env, adapter, prop_id, vote, governance),
        InterchainGovExecuteMsg::RequestVoteResults { prop_id } => request_vote_results(deps, env, adapter, prop_id),
        InterchainGovExecuteMsg::TestAddMembers { members } => test_add_members(deps, adapter, members),
        _ => todo!()
    }
}

fn request_vote_results(deps: DepsMut, env: Env, app: InterchainGov, prop_id: String) -> AdapterResult {
    let (prop, state) = load_proposal(deps.storage, &prop_id)?;

    // We can only tally upon expiration
    if !prop.expiration.is_expired(&env.block) {
        return Err(InterchainGovError::ProposalStillOpen(prop_id.clone()));
    }

    // Check whether the result is already finalized
    if VOTES.may_load(deps.storage, prop_id.clone())?.is_some() {
        return Err(InterchainGovError::VotesAlreadyFinalized(prop_id.clone()));
    }

    // First, make sure that all votes are in by checking whether we have vote results
    let existing_vote_results = VOTE_RESULTS.prefix(prop_id.clone()).range(deps.storage, None, None, Order::Ascending).collect::<StdResult<Vec<(ChainName, Option<GovernanceVote>)>>>()?;

    // If we don't have any vote results, we need to query them
    if !existing_vote_results.is_empty() {
        // if we have pending votes, check they're all resolved
        return if existing_vote_results.iter().any(|(_, vote)| vote.is_none()) {
            Err(InterchainGovError::VotesStillPending {
                prop_id: prop_id.clone(),
                chains: existing_vote_results.into_iter().map(|(chain, _)| chain.clone()).collect(),
            })
        } else {
            // happy path error
            Err(InterchainGovError::VotesAlreadyFinalized(prop_id.clone()))
        }
    };

    // Ask everyone to give us their votes
    let external_members = load_external_members(deps.storage, &env)?;
    let vote_queries = external_members.iter().map(|host| -> AbstractSdkResult<CosmosMsg> {
        let ibc_client = app.ibc_client(deps.as_ref());
        let module_addr = TEMP_REMOTE_GOV_MODULE_ADDRS.load(deps.storage, host)?;
        let query = ibc_client.ibc_query(host.to_string(), WasmQuery::Smart {
            contract_addr: module_addr,
            msg: to_json_binary(&InterchainGovQueryMsg::Vote {
                prop_id: prop_id.clone(),
            })?,
        }, CallbackInfo::new(REGISTER_VOTE_ID, None))?;


        // Mark the vote result as pending
        VOTE_RESULTS.save(deps.storage, (prop_id.clone(), host), &None)?;

        Ok(query)
    }).collect::<AbstractSdkResult<Vec<CosmosMsg>>>()?;

    // let external_members = load_external_members(deps.storage, &env)?;
    Ok(app.response("query_vote_results").add_attribute("prop_id", prop_id).add_messages(vote_queries))
}

fn query_gov_vote_results() {
    // // All queries must be resolved
    // if let Some(pending_votes) = GOV_VOTE_QUERIES.may_load(deps.storage, prop_id.clone())? {
    //     return Err(InterchainGovError::VotesStillPending {
    //         prop_id: prop_id.clone(),
    //         chains: pending_votes.into_iter().map(|(chain, _)| chain).collect(),
    //     });
    // }

    // let icq = app.neutron_icq(deps.as_ref(), env.contract.address)?;

    // // loop through the external members and register interchain queries for their votes
    // let gov_queries = external_members.iter().map(|host| {
    //     let query = icq.register_interchain_query(host, QueryType::KV, create_gov_proposal_keys(vec![]), vec![], 0)?;
    //     Ok(query)
    //     VOTE_QUERIES.save(deps.storage, prop_id.clone(), &(host.clone(), query))?;
    //
    // }).collect::<AbstractSdkResult<Vec<_>>>()?;
    //
    // // we should do this all at once
    // let withdraw_msg = app.bank(deps.as_ref()).transfer(coins(100000u128, "untrn"), &env.contract.address)?;
    // let withdraw_msg = app.executor(deps.as_ref()).execute(vec![withdraw_msg])?.into();

}

fn load_proposal(storage: &mut dyn Storage, prop_id: &String) -> Result<(Proposal, DataState), InterchainGovError> {
    PROPOSALS.load(storage, prop_id.clone()).map_err(|_| InterchainGovError::ProposalNotFound(prop_id.clone()))
}

// This currently allows for overriding votes
fn do_vote(deps: DepsMut, env: Env, app: InterchainGov, prop_id: String, vote: bool, governance: Governance) -> AdapterResult {
    let (prop, state) = load_proposal(deps.storage, &prop_id)?;

    state.expect_or(DataState::Finalized, |actual| InterchainGovError::InvalidProposalState {
        prop_id: prop_id.clone(),
        chain: ChainName::new(&env),
        expected: Some(DataState::Finalized),
        actual: Some(actual),
    })?;

    if prop.expiration.is_expired(&env.block) {
        return Err(InterchainGovError::ProposalExpired(prop_id.clone()));
    }

    let vote = if vote { Vote::Yes } else { Vote::No };
    VOTE.save(deps.storage, prop_id.clone(), &GovernanceVote::new(governance, vote))?;

    Ok(app.response("vote_proposal").add_attribute("prop_id", prop_id))
}

fn test_add_members(deps: DepsMut, app: InterchainGov, members: Vec<ChainName>) -> AdapterResult {
    MEMBERS.update(deps.storage, |mut m| -> StdResult<Members> {
        m.members.extend(members);
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
    let hash = <sha2::Sha256 as sha2::Digest>::digest(to_json_string(&proposal)?);
    let prop_id: ProposalId = format!("{:?}", hash);
    let prop = Proposal::new(proposal, &info.sender, &env);
    // check that prop doesn't exist
    let existing_prop = PROPOSALS.may_load(deps.storage, prop_id.clone())?;
    if existing_prop.is_some() {
        return Err(InterchainGovError::ProposalAlreadyExists(prop_id));
    }

    let external_members = load_external_members(deps.storage, &env)?;
    if external_members.is_empty() {
        // We have initiate state for the proposal, it will be proposed when we send it to other chains and get all confirmations
        PROPOSALS.save(deps.storage, prop_id.clone(), &(prop.clone(), DataState::Finalized))?;
        return Ok(app.response("propose").add_attribute("prop_id", prop_id))
    };

    // We have initiate state for the proposal, it will be proposed when we send it to other chains and get all confirmations
    PROPOSALS.save(deps.storage, prop_id.clone(), &(prop.clone(), DataState::Initiated))?;

    // 2.
    // TODO: THIS HARD_CODES A COSMOS SDK VOTE RESULT HERE, PROBABLY SHOULD INCLUDE IN PROPOSE?
    VOTE.save(deps.storage, prop_id.clone(), &GovernanceVote::new(Governance::CosmosSDK {
        proposal_id: 0,
    }, Vote::Yes))?;

    // 3.
    let target_module = this_module(&app)?;
    let callback = CallbackInfo::new(PROPOSE_CALLBACK_ID, None);
    // Loop through members and propose to them the proposal (TODO: do we actually need ourselves stored)?

    let propose_msgs = external_members.iter().map(|host| {
        let ibc_client = app.ibc_client(deps.as_ref());
        let msg = ibc_client.module_ibc_action(host.to_string(), target_module.clone(), &InterchainGovIbcMsg::ProposeProposal {
            prop: prop.clone(),
            prop_hash: prop_id.clone(),
            chain: host.clone(),
        }, Some(callback.clone()))?;

        REMOTE_PROPOSAL_STATE.save(deps.storage, (prop_id.clone(), host), &DataState::Initiated)?;
        Ok(msg)
    }).collect::<AbstractSdkResult<Vec<CosmosMsg>>>()?;

    Ok(app.response("propose").add_attribute("prop_id", prop_id).add_messages(propose_msgs))
}

/// Helper to load external members
fn load_external_members(storage: &mut dyn Storage, env: &Env) -> AdapterResult<Vec<ChainName>> {
    Ok(MEMBERS.load(storage)?.members.into_iter().filter(|m| m != &ChainName::new(env)).collect::<Vec<_>>())
}

/// Send finalization message over IBC
fn finalize(deps: DepsMut, env: Env, info: MessageInfo, app: InterchainGov, prop_id: ProposalId) -> AdapterResult {
    // check whether it exists
    let (prop, local_prop_state) = load_proposal(deps.storage, &prop_id)?;

    // The state must be initiated locally (OR we could make it proposed on the final callback?)
    if !local_prop_state.is_proposed() {
        return Err(InterchainGovError::InvalidProposalState {
            prop_id: prop_id.clone(),
            chain: ChainName::new(&env),
            expected: Some(DataState::Proposed),
            actual: Some(local_prop_state),
        });
    }

    // ensure that all remote chains have no state
    let mut prop_states = REMOTE_PROPOSAL_STATE.prefix(prop_id.clone()).range(deps.storage, None, None, Order::Ascending).take(1).collect::<StdResult<Vec<(ChainName, DataState)>>>()?;
    if let Some((chain, state)) = prop_states.pop() {
        return Err(InterchainGovError::InvalidProposalState {
            prop_id: prop_id.clone(),
            chain,
            expected: Some(DataState::Proposed),
            actual: Some(state),
        });
    }

    let target_module = this_module(&app)?;
    let callback = CallbackInfo::new(REGISTER_VOTE_ID, None);

    let external_members = load_external_members(deps.storage, &env)?;
    let finalize_messages = external_members.iter().map(|host| {
        let ibc_client = app.ibc_client(deps.as_ref());
        let msg = ibc_client.module_ibc_action(host.to_string(), target_module.clone(), &InterchainGovIbcMsg::FinalizeProposal {
            prop_hash: prop_id.clone(),
            chain: host.clone(),
        }, Some(callback.clone()))?;

        REMOTE_PROPOSAL_STATE.save(deps.storage, (prop_id.clone(), host), &DataState::Proposed)?;

        Ok(msg)
    }).collect::<AbstractSdkResult<Vec<CosmosMsg>>>()?;


    Ok(app.response("finalize").add_attribute("prop_id", prop_id).add_messages(finalize_messages))
}

/// TODO
fn invite_member(
    deps: DepsMut,
    app: InterchainGov,
    new_member: ChainName
) -> AdapterResult {

    todo!();
    // TODO: auth

    // check whether member is in the list
    let mut members= MEMBERS.load(deps.storage)?;
    let members_state = get_item_state(deps.storage, &MEMBERS)?;

    ensure_eq!(members_state, DataState::Finalized, InterchainGovError::DataNotFinalized {
        key: "members".to_string(),
        state: members_state
    });

    // check that all our other chains have been finalized?



    // Now that there's a new member, check that they're in the list of proxies
    let ibc_host_addr = app.ibc_host(deps.as_ref())?;

    if IBC_INFRA.query(&deps.querier, ibc_host_addr.clone(), &new_member)?.is_none() {
        return Err(InterchainGovError::UnknownRemoteHost {
            host: new_member.to_string(),
        })
    };
    // Add the new member, updating status to Proposed
    members.members.push(new_member.clone());

    propose_item_state(deps.storage, &MEMBERS, members.clone())?;

    let ibc_client = app.ibc_client(deps.as_ref());
    let target_module = this_module(&app)?;

    // let ibc_register_msg = ibc_client.module_ibc_action(new_members.to_string(), target_module, &InterchainGovIbcMsg::SyncState {
    //     key: "members".to_string(),
    //     value: to_json_binary(members.members)?
    // }, None)?;

    let ibc_register_msg = ibc_client.module_ibc_action(new_member.to_string(), target_module, &InterchainGovIbcMsg::UpdateMembers {
        members: members.members.clone(),
    }, None)?;

    Ok(app.response("update_members").add_message(ibc_register_msg))
}

fn this_module(app: &InterchainGov) -> AbstractResult<ModuleInfo> {
    ModuleInfo::from_id(app.module_id(), app.version().into())
}


