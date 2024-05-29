use crate::{
    contract::{AdapterResult, InterchainGov},
    InterchainGovError,
    msg::InterchainGovExecuteMsg,
};

use abstract_adapter::traits::AbstractResponse;
use abstract_adapter::objects::chain_name::ChainName;
use abstract_adapter::objects::module::ModuleInfo;
use abstract_adapter::sdk::base::ModuleIbcEndpoint;
use abstract_adapter::sdk::{AbstractSdkResult, IbcInterface};
use abstract_adapter::std::AbstractResult;
use abstract_adapter::std::ibc::CallbackInfo;
use abstract_adapter::std::ibc_client::state::IBC_INFRA;
use abstract_adapter::traits::ModuleIdentification;
use cosmwasm_std::{CosmosMsg, DepsMut, ensure_eq, Env, MessageInfo, to_json_binary};
use crate::handlers::instantiate::{get_item_state, propose_item_state};
use crate::ibc_callbacks::PROPOSE_CALLBACK_ID;
use crate::msg::{InterchainGovIbcCallbackMsg, InterchainGovIbcMsg};
use crate::state::{DataState, LOCAL_VOTE, MEMBERS, Proposal, ProposalAction, ProposalMsg, PROPOSALS, Vote};





pub fn execute_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    adapter: InterchainGov,
    msg: InterchainGovExecuteMsg,
) -> AdapterResult {
    match msg {
        InterchainGovExecuteMsg::Propose { proposal } => propose(deps, info, adapter, proposal, env),
        InterchainGovExecuteMsg::AddMembers { members } => invite_member(deps, adapter, members),
        // InterchainGovExecuteMsg::TallyProposal { prop_hash } => {
        //     tally_proposal(deps, adapter, prop_hash)
        // },
        _ => todo!()
    }
}

/// A

fn invite_member(
    deps: DepsMut,
    app: InterchainGov,
    new_member: ChainName
) -> AdapterResult {
    // TODO: auth

    // check whether member is in the list
    let mut members= MEMBERS.load(deps.storage)?;
    let members_state = get_item_state(&deps.storage, &MEMBERS)?;

    ensure_eq!(members_state, DataState::Finalized, InterchainGovError::DataNotFinalized {
        key: "members".to_string(),
        state,
    });

    // check that all our other chains have been finalized?


    let ibc_client = app.ibc_client(deps.as_ref());

    // Now that there's a new member, check that they're in the list of proxies
    let ibc_host_addr = app.ibc_host(deps.as_ref())?;

    if IBC_INFRA.query(&deps.querier, ibc_host_addr.clone(), &new_member)?.is_none() {
        return Err(InterchainGovError::UnknownRemoteHost {
            host: new_member.to_string(),
        })
    };
    // Add the new member, updating status to Proposed
    members.members.push(new_member.clone());

    propose_item_state(&deps.storage, &MEMBERS, members)?;

    let target_module = this_module(&app)?;


    // let ibc_register_msg = ibc_client.module_ibc_action(new_members.to_string(), target_module, &InterchainGovIbcMsg::SyncState {
    //     key: "members".to_string(),
    //     value: to_json_binary(members.members)?
    // }, None)?;

    let ibc_register_msg = ibc_client.module_ibc_action(new_member.to_string(), target_module, &InterchainGovIbcMsg::UpdateMembers {
        members
    }, None)?;

    Ok(app.response("add_members").add_message(ibc_register_msg))
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

    // 1.
    let hash = <sha2::Sha256 as sha2::Digest>::digest(proposal.clone());
    let prop_id = format!("{:?}", hash);
    let prop = Proposal::new(proposal, &info.sender, &env);
    // (no need to finalize proposal state because it's final)
    PROPOSALS.save(deps.storage, prop_id.clone(), &prop)?;

    // 2.
    LOCAL_VOTE.save(deps.storage, prop_id, &Vote::Yes)?;

    // 3.
    let ibc_client = app.ibc_client(deps.as_ref());
    let target_module = this_module(&app)?;

    // Loop through members and propose to them the proposal (TODO: do we actually need ourselves stored)?
    let external_members = MEMBERS.load(deps.storage)?.members.into_iter().filter(|m| m != &ChainName::new(&env)).collect::<Vec<_>>();

    let sync_msgs = external_members.iter().map(|host| {

        ibc_client.module_ibc_action(host.to_string(), target_module, &InterchainGovIbcMsg::ProposeProposal {
            prop: prop.clone(),
            prop_hash: prop_id.clone(),
            chain: host.clone(),
        }, None)
    }).collect::<AbstractSdkResult<Vec<CosmosMsg>>>()?;

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
    let msgs = match prop.action {
        ProposalAction::InviteMember {
            member
        } => invite_member(deps, app, member)
    };

    Ok(app.response("execute_proposal"))
}