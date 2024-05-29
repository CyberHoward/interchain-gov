use crate::{
    contract::{AdapterResult, InterchainGov},
    msg::InterchainGovInstantiateMsg,
    state::{Members, ALLOW_JOINING_GOV, MEMBERS},
};
use abstract_adapter::sdk::AbstractResponse;

use cosmwasm_std::{DepsMut, Env, MessageInfo};

pub fn instantiate_handler(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    app: InterchainGov,
    msg: InterchainGovInstantiateMsg,
) -> AdapterResult {
    MEMBERS.save(deps.storage, &Members::new(&env))?;
    ALLOW_JOINING_GOV.save(deps.storage, &msg.accept_proposal_from_gov)?;

    Ok(app.response("instantiate"))
}
