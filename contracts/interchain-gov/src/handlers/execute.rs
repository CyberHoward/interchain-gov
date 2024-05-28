use crate::{
    contract::{AdapterResult, InterchainGov},
    msg::InterchainGovExecuteMsg,
    state::{CONFIG, STATUS},
    InterchainGovError, MY_NAMESPACE,
};

use abstract_adapter::{
    objects::namespace::Namespace,
    sdk::{AccountVerification, ModuleRegistryInterface},
    traits::AbstractResponse,
};
use cosmwasm_std::{ensure_eq, DepsMut, Env, MessageInfo};

pub fn execute_handler(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    adapter: InterchainGov,
    msg: InterchainGovExecuteMsg,
) -> AdapterResult {
    match msg {
        InterchainGovExecuteMsg::CreateProposal { proposal } => {
            create_proposal(deps, info, adapter, proposal)
        },
        InterchainGovExecuteMsg::TallyProposal { prop_hash } => {
            tally_proposal(deps, adapter, prop_hash)
        },
    }
}

fn create_proposal(
    deps: DepsMut,
    info: MessageInfo,
    adapter: InterchainGov,
    proposal: Proposal,
) -> AdapterResult {
    let config = CONFIG.load(deps.storage)?;
    let status = STATUS.load(deps.storage)?;

    ensure_eq!(status, DataStatus::Initiate, InterchainGovError::ProposalAlreadyExists);

    let registry = adapter
        .base
        .module_registry(deps.as_ref())
        .get_registry(MY_NAMESPACE)?;

    let prop_hash = registry.hash_proposal(&proposal)?;

    let prop = Proposal {
        title: proposal.title,
        description: proposal.description,
        proposer: info.sender,
        proposer_chain: proposal.proposer_chain,
        min_voting_period: proposal.min_voting_period,
        expiration: proposal.expiration,
        status: Status::Proposed,
    };

    STATUS.save(deps.storage, &DataStatus::Proposed)?;

    Ok(prop_hash.to_response())
}