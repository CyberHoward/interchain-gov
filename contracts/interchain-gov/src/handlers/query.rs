use crate::{
    contract::{AdapterResult, InterchainGov},
    msg::{ConfigResponse, InterchainGovQueryMsg},
};

use cosmwasm_std::{to_json_binary, Binary, Deps, Env, StdResult, Order};
use crate::handlers::instantiate::get_item_state;
use crate::msg::{MembersResponse, ProposalResponse, ProposalsResponse};
use crate::state::{MEMBERS, ProposalId, PROPOSALS};

pub fn query_handler(
    deps: Deps,
    _env: Env,
    _adapter: &InterchainGov,
    msg: InterchainGovQueryMsg,
) -> AdapterResult<Binary> {
    match msg {
        InterchainGovQueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        InterchainGovQueryMsg::Members {} => to_json_binary(&query_members(deps)?),
        InterchainGovQueryMsg::Proposal { prop_id } => to_json_binary(&query_proposal(deps, prop_id)?),
        InterchainGovQueryMsg::ListProposals { } => to_json_binary(&query_proposals(deps)?),
    }
    .map_err(Into::into)
}

fn query_proposals(deps: Deps) -> StdResult<ProposalsResponse> {
    let proposals = PROPOSALS.range(deps.storage, None, None, Order::Ascending).collect::<StdResult<Vec<_>>>()?;

    let proposals = proposals.into_iter().map(|(prop_id, (prop, state))| {
        crate::msg::ProposalResponse {
            prop_id,
            prop,
            state,
        }
    }).collect();

    Ok(ProposalsResponse {
        proposals,
    })
}

fn query_proposal(deps: Deps, prop_id: ProposalId) -> StdResult<ProposalResponse> {
    let (prop, state) = PROPOSALS.load(deps.storage, prop_id.clone())?;

    Ok(ProposalResponse {
        prop_id,
        prop,
        state,
    })
}

fn query_members(deps: Deps) -> StdResult<MembersResponse> {
    let members = MEMBERS.load(deps.storage)?;
    let state = get_item_state(deps.storage, &MEMBERS)?;

    Ok(MembersResponse {
        members: members.members,
        state,
    })
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    Ok(ConfigResponse {})
}
