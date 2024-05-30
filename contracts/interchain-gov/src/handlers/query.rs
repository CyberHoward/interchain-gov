use abstract_adapter::objects::chain_name::ChainName;
use crate::{
    contract::{AdapterResult, InterchainGov},
    msg::{ConfigResponse, InterchainGovQueryMsg, MapState, ProposalStateResponse},
    state::PROPOSAL_STATE_SYNC,
};

use crate::msg::{MembersResponse, ProposalResponse, ProposalsResponse, VoteResponse, VoteResultsResponse};
use crate::state::{GovernanceVote, MEMBERS, ProposalId, VOTE, VOTE_RESULTS};
use cosmwasm_std::{to_json_binary, Binary, Deps, Env, Order, StdResult};
use ibc_sync_state::DataState;

pub fn query_handler(
    deps: Deps,
    env: Env,
    _adapter: &InterchainGov,
    msg: InterchainGovQueryMsg,
) -> AdapterResult<Binary> {
    match msg {
        InterchainGovQueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        InterchainGovQueryMsg::Members {} => to_json_binary(&query_members(deps)?),
        InterchainGovQueryMsg::Proposal { prop_id } => to_json_binary(&query_proposal(deps, prop_id)?),
        InterchainGovQueryMsg::Vote { prop_id } => to_json_binary(&query_vote(deps, env, prop_id)?),
        InterchainGovQueryMsg::VoteResults { prop_id } => to_json_binary(&query_vote_results(deps, env, prop_id)?),
        InterchainGovQueryMsg::ListProposalStates {} => to_json_binary(&query_props_state(deps)?),
        InterchainGovQueryMsg::Proposals { proposal_ids } => {
            to_json_binary(&query_proposals(deps, proposal_ids)?)
        }
        InterchainGovQueryMsg::ListProposals {} => to_json_binary(&query_list_proposals(deps)?),
        InterchainGovQueryMsg::ProposalState { prop_id } => {
            to_json_binary(&query_prop_state(deps, prop_id)?)
        }
    }
    .map_err(Into::into)
}

fn query_proposals(deps: Deps, proposal_ids: Vec<String>) -> AdapterResult<ProposalsResponse> {
    let mut proposals = vec![];
    for proposal_id in proposal_ids {
        let (prop, _vote) = PROPOSAL_STATE_SYNC.load(deps.storage, proposal_id.clone())?;
        let data_state = PROPOSAL_STATE_SYNC.data_state(deps.storage, proposal_id.clone());
        proposals.push(ProposalResponse {
            prop_id: proposal_id,
            prop: prop,
            state: data_state,
        })
    }
    Ok(ProposalsResponse { proposals })
}

fn query_list_proposals(deps: Deps) -> AdapterResult<ProposalsResponse> {
    let proposals = PROPOSAL_STATE_SYNC
        .map()
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    let proposals = proposals
        .into_iter()
        .map(|(prop_id, (prop, _vote))| {
            let data_state = PROPOSAL_STATE_SYNC.data_state(deps.storage, prop_id.clone());
            crate::msg::ProposalResponse {
                prop_id,
                prop,
                state: data_state,
            }
        })
        .collect();

    Ok(ProposalsResponse { proposals })
}

fn query_props_state(deps: Deps) -> AdapterResult<ProposalStateResponse> {
    let states = PROPOSAL_STATE_SYNC
        .state_status_map()
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    let states = states
        .into_iter()
        .map(|((nk, k, i), change)| MapState {
            namespace: nk,
            proposal_id: k,
            state: DataState::from_num(i).unwrap(),
            change,
        })
        .collect();

    Ok(ProposalStateResponse { state: states })
}

fn query_prop_state(deps: Deps, prop_id: ProposalId) -> AdapterResult<Option<MapState>> {
    let mut states = PROPOSAL_STATE_SYNC
        .state_status_map()
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    states.retain(|((_, a, _), _)| a == &prop_id);

    let states: Vec<MapState> = states
        .into_iter()
        .map(|((nk, k, i), change)| MapState {
            namespace: nk,
            proposal_id: k,
            state: DataState::from_num(i).unwrap(),
            change,
        })
        .collect();

    if states.is_empty() {
        Ok(None)
    } else {
        Ok(Some(states[0].clone()))
    }
}

fn query_proposal(deps: Deps, prop_id: ProposalId) -> AdapterResult<ProposalResponse> {
    let (prop, _) = PROPOSAL_STATE_SYNC.load(deps.storage, prop_id.clone())?;
    let data_state = PROPOSAL_STATE_SYNC.data_state(deps.storage, prop_id.clone());

    Ok(ProposalResponse {
        prop_id,
        prop,
        state: data_state,
    })
}

fn query_members(deps: Deps) -> AdapterResult<MembersResponse> {
    let members = MEMBERS.load(deps.storage)?;
    // TODO: fix
    Ok(MembersResponse { members })
}

fn query_config(_deps: Deps) -> AdapterResult<ConfigResponse> {
    Ok(ConfigResponse {})
}

fn query_vote(deps: Deps, env: Env, prop_id: ProposalId) -> StdResult<VoteResponse> {
    let GovernanceVote {
        vote,
        governance
    } = VOTE.load(deps.storage, prop_id.clone())?;

    Ok(VoteResponse {
        vote,
        governance,
        prop_id,
        chain: ChainName::new(&env)
    })
}

fn query_vote_results(deps: Deps, env: Env, prop_id: ProposalId) -> StdResult<VoteResultsResponse> {
    let results = VOTE_RESULTS.prefix(prop_id.clone()).range(deps.storage, None, None, Order::Ascending).collect::<StdResult<Vec<(ChainName, Option<GovernanceVote>)>>>()?;

    Ok(crate::msg::VoteResultsResponse {
        prop_id,
        results
    })
}
