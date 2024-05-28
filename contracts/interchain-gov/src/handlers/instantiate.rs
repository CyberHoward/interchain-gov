use abstract_adapter::sdk::AbstractResponse;
use crate::{
    contract::{AdapterResult, InterchainGov},
    msg::InterchainGovInstantiateMsg,
    state::{Members, MEMBERS},
};

use cosmwasm_std::{DepsMut, Env, MessageInfo};
use crate::state::DataState;

pub fn instantiate_handler(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    app: InterchainGov,
    msg: InterchainGovInstantiateMsg,
) -> AdapterResult {
    MEMBERS.save(deps.storage, &(Members::new(&env), DataState::Finalized))?;

    Ok(app.response("instantiate"))
}
