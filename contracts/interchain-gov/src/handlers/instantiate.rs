use crate::{
    contract::{AdapterResult, InterchainGov},
    msg::InterchainGovInstantiateMsg,
    state::{Members, ALLOW_JOINING_GOV, MEMBERS},
};
use abstract_adapter::sdk::AbstractResponse;

use crate::state::{DataState, StateChange, StorageKey, ITEMS_DATA_STATE, PROPOSAL_SEQ};
use cosmwasm_std::{to_json_binary, DepsMut, Env, MessageInfo, Order, StdResult, Storage};
use cw_storage_plus::{Item, Map};
use schemars::_serde_json::error::Category::Data;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub fn instantiate_handler(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    app: InterchainGov,
    msg: InterchainGovInstantiateMsg,
) -> AdapterResult {
    MEMBERS.save(deps.storage, &Members::new(&env))?;
    ALLOW_JOINING_GOV.save(deps.storage, msg.accept_proposal_from_gov.members)?;

    Ok(app.response("instantiate"))
}
