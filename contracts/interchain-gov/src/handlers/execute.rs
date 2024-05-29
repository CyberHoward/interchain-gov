use crate::{
    contract::{AdapterResult, InterchainGov},
    msg::InterchainGovExecuteMsg,
    state::{Members, MEMBERS, MEMBERS_STATE_SYNC},
    InterchainGovError,
};

use crate::msg::InterchainGovIbcMsg;
use crate::state::{Proposal, ProposalAction, ProposalMsg, Vote, LOCAL_VOTE, PROPOSALS};
use abstract_adapter::objects::chain_name::ChainName;
use abstract_adapter::objects::module::ModuleInfo;
use abstract_adapter::sdk::base::ModuleIbcEndpoint;
use abstract_adapter::sdk::{AbstractSdkResult, IbcInterface};

use abstract_adapter::std::ibc_client::state::IBC_INFRA;
use abstract_adapter::std::AbstractResult;
use abstract_adapter::traits::AbstractResponse;
use abstract_adapter::traits::ModuleIdentification;
use base64::Engine;
use cosmwasm_std::{CosmosMsg, DepsMut, Env, MessageInfo};

pub fn execute_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    adapter: InterchainGov,
    msg: InterchainGovExecuteMsg,
) -> AdapterResult {
    match msg {
        InterchainGovExecuteMsg::Propose { proposal } => {
            propose(deps, info, adapter, proposal, env)
        }
        InterchainGovExecuteMsg::ProposeGovMembers { members: _ } => {
            // TODO: Remove this as it's just a proposal.
            todo!()
        }
        _ => todo!(),
    }
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
    // TODO: auth

    // If the proposed change is identical to our current state, no action required.
    let members = MEMBERS_STATE_SYNC.load_members(deps.storage)?;
    if updated_members == members {
        return Ok(app.response("add_members"));
    }

    // Now propose change to others.

    // We initiate the state here.
    let member_controller = MEMBERS_STATE_SYNC;
    member_controller.initiate_members(deps.storage, updated_members.clone())?;

    let ibc_client = app.ibc_client(deps.as_ref());

    // Check member diff, see which members are added and assert they are connected.
    let new_members = updated_members
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

/// Propose a new message to the interchain DAO
/// 1. Create a new proposal
/// 2. Vote on our own proposal
/// 3. Propose to other chains, adding a callback to update the proposal's proposed state (to ensure they received it)
fn propose(
    deps: DepsMut,
    info: MessageInfo,
    app: InterchainGov,
    proposal: ProposalMsg,
    env: Env,
) -> AdapterResult {
    let hash = <sha2::Sha256 as sha2::Digest>::digest(proposal.to_string());
    let prop_id = base64::prelude::BASE64_STANDARD.encode(hash.as_slice());
    let prop = Proposal::new(proposal.clone(), &info.sender, &env);

    // If this chain is only member, approve prop and execute prop action.
    // Otherwise, propose to other chains
    let members = MEMBERS.load(deps.storage)?;

    // Execute proposal directly
    if members.members.len() == 1 {
        // 1.
        PROPOSALS.save(deps.storage, prop_id.clone(), &prop)?;

        // 2.
        LOCAL_VOTE.save(deps.storage, prop_id, &Vote::Yes)?;

        // 3.
        return execute_proposal(deps, info, app, proposal, env);
    }

    // // (no need to finalize proposal state because it's final)
    // PROPOSALS.save(deps.storage, prop_id.clone(), &prop)?;

    // // 2.
    // LOCAL_VOTE.save(deps.storage, prop_id, &Vote::Yes)?;

    // 3.
    let ibc_client = app.ibc_client(deps.as_ref());
    let target_module = this_module(&app)?;

    let external_members = MEMBERS
        .load(deps.storage)?
        .members
        .into_iter()
        .filter(|m| m != &ChainName::new(&env))
        .collect::<Vec<_>>();

    let sync_msgs = external_members
        .iter()
        .map(|host| {
            ibc_client.module_ibc_action(
                host.to_string(),
                target_module.clone(),
                &InterchainGovIbcMsg::ProposeProposal {
                    prop: prop.clone(),
                    prop_hash: prop_id.clone(),
                    chain: host.clone(),
                },
                None,
            )
        })
        .collect::<AbstractSdkResult<Vec<CosmosMsg>>>()?;

    Ok(app.response("propose").add_messages(sync_msgs))
}

/// Execute a proposal that we have
fn execute_proposal(
    deps: DepsMut,
    info: MessageInfo,
    app: InterchainGov,
    prop: ProposalMsg,
    env: Env,
) -> AdapterResult {
    let _msgs = match &prop.action {
        ProposalAction::UpdateMembers { ref members } => {
            propose_gov_members(deps, env, info, &app, members.clone(), prop)
        }
        _ => todo!(),
    };

    Ok(app.response("execute_proposal"))
}
