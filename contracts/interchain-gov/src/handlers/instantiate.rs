use crate::{
    contract::{AdapterResult, InterchainGov},
    msg::InterchainGovInstantiateMsg,
    state::{Config, Members, CONFIG, MEMBERS},
};

use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

pub fn instantiate_handler(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _adapter: InterchainGov,
    msg: InterchainGovInstantiateMsg,
) -> AdapterResult {
    MEMBERS.save(deps.storage, &Members { members: msg.members} )?;

    Ok(Response::new())
}
