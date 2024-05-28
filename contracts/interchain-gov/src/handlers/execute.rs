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
use cosmwasm_std::{ensure_eq, DepsMut, Env, MessageInfo};
use crate::msg::InterchainGovIbcMsg;
use crate::state::{DataStatus, MEMBERS, Proposal, ProposalMsg};

pub fn execute_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    adapter: InterchainGov,
    msg: InterchainGovExecuteMsg,
) -> AdapterResult {
    match msg {
        InterchainGovExecuteMsg::CreateProposal { proposal } => {
            create_proposal(deps, info, adapter, proposal, env)
        },
        InterchainGovExecuteMsg::AddMember { chain } => add_member(deps, adapter, chain),
        // InterchainGovExecuteMsg::TallyProposal { prop_hash } => {
        //     tally_proposal(deps, adapter, prop_hash)
        // },
        _ => todo!()
    }
}

/// A

fn add_member(
    deps: DepsMut,
    app: InterchainGov,
    member: ChainName
) -> AdapterResult {
    // TODO: auth

    // check whether member is in the list
    let (mut members, status) = MEMBERS.load(deps.storage)?;
    ensure_eq!(status, DataStatus::Finalized, InterchainGovError::DataNotFinalized {
        key: "members".to_string(),
        status,
    });

    // Now that there's a new member, check that they're in the list of proxies
    let ibc_host_addr = app.ibc_host(deps.as_ref())?;
    if IBC_INFRA.query(&deps.querier, ibc_host_addr, &member)?.is_none() {
        return Err(InterchainGovError::UnknownRemoteHost {
            chain: member.to_string(),
        })
    };

    // Add the new member, updating status to Proposed
    members.members.push(member.clone());
    MEMBERS.save(deps.storage, &(members.clone(), DataStatus::Proposed))?;

    let target_module = ModuleInfo::from_id(app.module_id(), app.version().into())?;

    let ibc_client = app.ibc_client(deps.as_ref());

    let ibc_register_msg = ibc_client.module_ibc_action(member.to_string(),target_module, &InterchainGovIbcMsg::Register, None)?;

    Ok(app.response("add_member").add_message(ibc_register_msg))
}


fn create_proposal(
    deps: DepsMut,
    info: MessageInfo,
    app: InterchainGov,
    proposal: ProposalMsg,
    env: Env,
) -> AdapterResult {

    let prop = Proposal::new(proposal, &info.sender, &env);


    Ok(app.response("create_proposal"))
}