use crate::{
    contract::{AdapterResult, InterchainGov},
    msg::InterchainGovExecuteMsg,
    InterchainGovError,
};

use abstract_adapter::{
    traits::AbstractResponse,
};
use abstract_adapter::objects::chain_name::ChainName;
use abstract_adapter::objects::module::ModuleInfo;
use abstract_adapter::sdk::base::{ModuleIbcEndpoint};
use abstract_adapter::sdk::IbcInterface;
use abstract_adapter::std::ibc_client::state::IBC_INFRA;
use abstract_adapter::traits::ModuleIdentification;
use cosmwasm_std::{ensure_eq, DepsMut, Env, MessageInfo, to_json_binary};
use crate::msg::InterchainGovIbcMsg;
use crate::state::{DataState, MEMBERS, Proposal, ProposalMsg};





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
        },
        InterchainGovExecuteMsg::AddMembers { members } => add_members(deps, adapter, members),
        // InterchainGovExecuteMsg::TallyProposal { prop_hash } => {
        //     tally_proposal(deps, adapter, prop_hash)
        // },
        _ => todo!()
    }
}

/// A

fn add_members(
    deps: DepsMut,
    app: InterchainGov,
    new_members: Vec<ChainName>
) -> AdapterResult {
    // TODO: auth

    // check whether member is in the list
    let (mut members, status) = MEMBERS.load(deps.storage)?;

    ensure_eq!(status, DataState::Finalized, InterchainGovError::DataNotFinalized {
        key: "members".to_string(),
        state,
    });

    // check that all our other chains have been finalized?


    let ibc_client = app.ibc_client(deps.as_ref());
    let mut registration_messages = vec![];

    // Now that there's a new member, check that they're in the list of proxies
    let ibc_host_addr = app.ibc_host(deps.as_ref())?;
    for new_member in new_members.iter() {
        if IBC_INFRA.query(&deps.querier, ibc_host_addr.clone(), &new_member)?.is_none() {
            return Err(InterchainGovError::UnknownRemoteHost {
                host: new_member.to_string(),
            })
        };
        // Add the new member, updating status to Proposed
        members.members.push(new_member.clone());
    }


    MEMBERS.save(deps.storage, &(members.clone(), DataState::Proposed))?;

    let target_module = ModuleInfo::from_id(app.module_id(), app.version().into())?;


    // let ibc_register_msg = ibc_client.module_ibc_action(new_members.to_string(), target_module, &InterchainGovIbcMsg::SyncState {
    //     key: "members".to_string(),
    //     value: to_json_binary(members.members)?
    // }, None)?;

    let ibc_register_msg = ibc_client.module_ibc_action(new_members.to_string(), target_module, &InterchainGovIbcMsg::UpdateMembers {
        members
    }, None)?;

    Ok(app.response("add_members").add_message(ibc_register_msg))
}


fn propose(
    deps: DepsMut,
    info: MessageInfo,
    app: InterchainGov,
    proposal: ProposalMsg,
    env: Env,
) -> AdapterResult {

    let prop = Proposal::new(proposal, &info.sender, &env);

    Ok(app.response("create_proposal"))
}