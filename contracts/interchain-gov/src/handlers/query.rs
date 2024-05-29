use crate::{
    contract::{AdapterResult, InterchainGov},
    msg::{ConfigResponse, InterchainGovQueryMsg},
    state::{MEMBERS_STATE_SYNC, PROPOSAL_STATE_SYNC},
};

use crate::msg::{MembersResponse, ProposalResponse, ProposalsResponse};
use crate::state::{ProposalId, MEMBERS};
use cosmwasm_std::{to_json_binary, Binary, Deps, Env, Order, StdResult};

pub fn query_handler(
    deps: Deps,
    _env: Env,
    _adapter: &InterchainGov,
    msg: InterchainGovQueryMsg,
) -> AdapterResult<Binary> {
    match msg {
        InterchainGovQueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        InterchainGovQueryMsg::Members {} => to_json_binary(&query_members(deps)?),
        InterchainGovQueryMsg::Proposal { prop_id } => {
            to_json_binary(&query_proposal(deps, prop_id)?)
        }
        InterchainGovQueryMsg::ListProposals {} => to_json_binary(&query_proposals(deps)?),
    }
    .map_err(Into::into)
}

fn query_proposals(deps: Deps) -> AdapterResult<ProposalsResponse> {
    let proposals = PROPOSAL_STATE_SYNC
        .map()
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    let proposals = proposals
        .into_iter()
        .map(|(prop_id, (prop, state))| crate::msg::ProposalResponse { prop_id, prop })
        .collect();

    Ok(ProposalsResponse { proposals })
}

fn query_proposal(deps: Deps, prop_id: ProposalId) -> AdapterResult<ProposalResponse> {
    let (prop, state) = PROPOSAL_STATE_SYNC.load(deps.storage, prop_id.clone())?;

    Ok(ProposalResponse { prop_id, prop })
}

fn query_members(deps: Deps) -> AdapterResult<MembersResponse> {
    let members = MEMBERS.load(deps.storage)?;
    // TODO: fix
    Ok(MembersResponse { members })
}

fn query_config(_deps: Deps) -> AdapterResult<ConfigResponse> {
    Ok(ConfigResponse {})
}
