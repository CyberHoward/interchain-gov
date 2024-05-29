use crate::{
    contract::{AdapterResult, InterchainGov},
    msg::{ConfigResponse, InterchainGovQueryMsg},
};

use cosmwasm_std::{to_json_binary, Binary, Deps, Env, StdResult};
use crate::handlers::instantiate::get_item_state;
use crate::msg::MembersResponse;
use crate::state::{MEMBERS};

pub fn query_handler(
    deps: Deps,
    _env: Env,
    _adapter: &InterchainGov,
    msg: InterchainGovQueryMsg,
) -> AdapterResult<Binary> {
    match msg {
        InterchainGovQueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        InterchainGovQueryMsg::Members {} => to_json_binary(&query_members(deps)?),
    }
    .map_err(Into::into)
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
