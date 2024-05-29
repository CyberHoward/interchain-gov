use abstract_adapter::objects::chain_name::ChainName;
use abstract_adapter::objects::module::ModuleInfo;
use abstract_adapter::sdk::base::ModuleIbcEndpoint;
use abstract_adapter::sdk::{AbstractSdkResult, IbcInterface};
use abstract_adapter::std::ibc::CallbackInfo;
use abstract_adapter::std::ibc_client::state::IBC_INFRA;
use abstract_adapter::std::AbstractResult;
use abstract_adapter::traits::AbstractResponse;
use abstract_adapter::traits::ModuleIdentification;
use cosmwasm_std::{
    ensure_eq, to_json_binary, to_json_string, CosmosMsg, DepsMut, Env, MessageInfo, Order,
    StdResult, Storage,
};

use crate::ibc_callbacks::{FINALIZE_CALLBACK_ID, PROPOSE_CALLBACK_ID};
use crate::{
    contract::{AdapterResult, InterchainGov},
    msg::InterchainGovExecuteMsg,
    state::{Members, MEMBERS, MEMBERS_STATE_SYNC},
    InterchainGovError,
};

use crate::msg::{InterchainGovIbcCallbackMsg, InterchainGovIbcMsg};
use crate::state::{Proposal, ProposalAction, ProposalId, ProposalMsg, Vote, PROPOSAL_STATE_SYNC};
use base64::Engine;

fn tally(deps: DepsMut, env: Env, app: InterchainGov, prop_id: String) -> AdapterResult {
    let (prop, state) = PROPOSAL_STATE_SYNC.load(deps.storage, prop_id.clone())?;

    if !prop.expiration.is_expired(&env.block) {
        return Err(InterchainGovError::ProposalStillOpen(prop_id.clone()));
    }

    // let external_members = load_external_members(deps.storage, &env)?;
    Ok(app
        .response("tally_proposal")
        .add_attribute("prop_id", prop_id))
}

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
            prop_hash: prop_id,
            vote,
        } => do_vote(deps, env, adapter, prop_id, vote),
        InterchainGovExecuteMsg::TallyProposal { prop_id } => tally(deps, env, adapter, prop_id),
        InterchainGovExecuteMsg::TestAddMembers { members } => {
            test_add_members(deps, adapter, members)
        }
        _ => todo!(),
    }
}

// This currently allows for overriding votes
fn do_vote(
    deps: DepsMut,
    env: Env,
    app: InterchainGov,
    prop_id: String,
    vote: bool,
) -> AdapterResult {
    PROPOSAL_STATE_SYNC.assert_finalized(deps.storage, prop_id.clone())?;

    let (prop, _) = PROPOSAL_STATE_SYNC.load(deps.storage, prop_id.clone())?;
    if prop.expiration.is_expired(&env.block) {
        return Err(InterchainGovError::ProposalExpired(prop_id.clone()));
    }

    PROPOSAL_STATE_SYNC.finalize_kv_state(
        deps.storage,
        prop_id.clone(),
        Some((prop, Vote::new(vote))),
    )?;

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
                // Filter out self.
                members.members.retain(|m| m != &ChainName::new(&env));

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

/// Send finalization message over IBC
/// TODO: call after all successful proposals have been cleared
fn finalize(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    app: InterchainGov,
    prop_id: ProposalId,
) -> AdapterResult {
    // State is already finalized after receiving the final proposal.

    let target_module = this_module(&app)?;
    let callback = CallbackInfo::new(FINALIZE_CALLBACK_ID, None);

    let external_members = MEMBERS_STATE_SYNC.external_members(deps.storage, &env)?;
    let finalize_messages = external_members
        .members
        .iter()
        .map(|host| {
            let ibc_client = app.ibc_client(deps.as_ref());
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

/// Do a proposal to add new members to the governance
fn propose_gov_members(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    app: &InterchainGov,
    updated_members: Members,
    _proposal: ProposalMsg,
) -> AdapterResult {
    todo!();
    // TODO: auth

    // If the proposed change is identical to our current state, no action required.
    let members = MEMBERS_STATE_SYNC.load_members(deps.storage)?;
    if updated_members == members {
        return Ok(app.response("add_members"));
    }

    // Now propose change to others.

    // We initiate the state here.
    let member_controller = MEMBERS_STATE_SYNC;
    member_controller.initiate_members(deps.storage, &env, updated_members.clone())?;

    // Check member diff, see which members are added and assert they are connected.
    let mut new_members = updated_members
        .members
        .iter()
        .filter(|m| !members.members.contains(m))
        .collect::<Vec<_>>();

    let ibc_host_addr = app.ibc_host(deps.as_ref())?;
    for new_member in new_members.iter() {
        if IBC_INFRA
            .query(&deps.querier, ibc_host_addr.clone(), &new_member)?
            .is_none()
        {
            return Err(InterchainGovError::UnknownRemoteHost {
                host: new_member.to_string(),
            });
        };
    }

    let ibc_client = app.ibc_client(deps.as_ref());
    let target_module = this_module(&app)?;

    let propose_msg = ProposalMsg {
        title: "Member Update Proposal".to_string(),
        description: format!("New member group: {:?}", new_members),
        // TODO: make these a config
        min_voting_period: None,
        expiration: cw_utils::Expiration::AtTime(env.block.time.plus_days(1)),
        action: ProposalAction::UpdateMembers {
            members: updated_members.clone(),
        },
    };

    let prop_hash = propose_msg.hash();
    let prop = Proposal::new(propose_msg, &info.sender, &env);

    // Propose to all current members of the gov
    let mut msgs = vec![];
    for member in members.members.iter() {
        msgs.push(ibc_client.module_ibc_action(
            member.to_string(),
            target_module.clone(),
            &InterchainGovIbcMsg::ProposeProposal {
                prop_hash: prop_hash.clone(),
                prop: prop.clone(),
                chain: member.clone(),
            },
            None,
        )?);
    }

    Ok(app.response("add_members").add_messages(msgs))
}

fn this_module(app: &InterchainGov) -> AbstractResult<ModuleInfo> {
    ModuleInfo::from_id(app.module_id(), app.version().into())
}
