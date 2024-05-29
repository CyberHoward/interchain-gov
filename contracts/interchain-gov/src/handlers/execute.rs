use abstract_adapter::objects::chain_name::ChainName;
use abstract_adapter::objects::module::ModuleInfo;
use abstract_adapter::sdk::{AbstractSdkResult, IbcInterface};
use abstract_adapter::sdk::base::ModuleIbcEndpoint;
use abstract_adapter::std::AbstractResult;
use abstract_adapter::std::ibc::CallbackInfo;
use abstract_adapter::std::ibc_client::state::IBC_INFRA;
use abstract_adapter::traits::AbstractResponse;
use abstract_adapter::traits::ModuleIdentification;
use cosmwasm_std::{CosmosMsg, DepsMut, ensure_eq, Env, MessageInfo, Order, StdResult, to_json_string};

use crate::{
    contract::{AdapterResult, InterchainGov},
    InterchainGovError,
    msg::InterchainGovExecuteMsg,
};
use crate::handlers::instantiate::{get_item_state, propose_item_state};
use crate::ibc_callbacks::{FINALIZE_CALLBACK_ID, PROPOSE_CALLBACK_ID};
use crate::msg::{InterchainGovIbcMsg};
use crate::state::{DataState, LOCAL_VOTE, MEMBERS, Proposal, REMOTE_PROPOSAL_STATE, ProposalId, ProposalMsg, PROPOSALS, Vote};

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
        _ => todo!()
    }
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

    // We have initiate state for the proposal, it will be proposed when we send it to other chains and get all confirmations
    PROPOSALS.save(deps.storage, prop_id.clone(), &(prop.clone(), DataState::Initiated))?;

    // 2.
    LOCAL_VOTE.save(deps.storage, prop_id.clone(), &Vote::Yes)?;

    // 3.
    let target_module = this_module(&app)?;
    let callback = CallbackInfo::new(PROPOSE_CALLBACK_ID, None);
    // Loop through members and propose to them the proposal (TODO: do we actually need ourselves stored)?
    let external_members = MEMBERS.load(deps.storage)?.members.into_iter().filter(|m| m != &ChainName::new(&env)).collect::<Vec<_>>();

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

/// Send finalization message over IBC
fn finalize(deps: DepsMut, env: Env, info: MessageInfo, app: InterchainGov, prop_id: ProposalId) -> AdapterResult {
    // check whether it exists
    let (prop, local_prop_state) = PROPOSALS.load(deps.storage, prop_id.clone()).map_err(|_| InterchainGovError::ProposalNotFound(prop_id.clone()))?;

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

    let ibc_client = app.ibc_client(deps.as_ref());
    let target_module = this_module(&app)?;
    let callback = CallbackInfo::new(FINALIZE_CALLBACK_ID, None);

    let external_members = MEMBERS.load(deps.storage)?.members.into_iter().filter(|m| m != &ChainName::new(&env)).collect::<Vec<_>>();
    let finalize_messages = external_members.iter().map(|host| {
        ibc_client.module_ibc_action(host.to_string(), target_module.clone(), &InterchainGovIbcMsg::FinalizeProposal {
            prop_hash: prop_id.clone(),
            chain: host.clone(),
        }, Some(callback.clone()))
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



// /// Execute a proposal that we have
// fn execute_proposal(
//     deps: DepsMut,
//     info: MessageInfo,
//     app: InterchainGov,
//     prop: ProposalMsg,
//     env: Env,
// ) -> AdapterResult {
//     let msgs = match prop.action {
//         ProposalAction::InviteMember {
//             member
//         } => invite_member(deps, app, member)
//     };
//
//     Ok(app.response("execute_proposal"))
// }