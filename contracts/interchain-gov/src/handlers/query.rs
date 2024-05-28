use crate::{
    contract::{AdapterResult, InterchainGov},
    msg::{ConfigResponse, InterchainGovQueryMsg},
};

use cosmwasm_std::{to_json_binary, Binary, Deps, Env, StdResult};
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

fn query_members(p0: Deps) -> StdResult<MembersResponse> {
    let members = MEMBERS.load(p0.storage)?;

    Ok(MembersResponse {
        members: members.0.members,
        status: members.1,
    })
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    Ok(ConfigResponse {})
}
