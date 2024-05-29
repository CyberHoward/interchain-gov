use abstract_adapter::sdk::AbstractResponse;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Order, StdResult, Storage, to_json_binary};
use cw_storage_plus::Item;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{
    contract::{AdapterResult, InterchainGov},
    msg::InterchainGovInstantiateMsg,
    state::{Members, MEMBERS},
};
use crate::state::{DataState, ITEMS_DATA_STATE, StateChange, StorageKey};

pub fn instantiate_handler(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    app: InterchainGov,
    msg: InterchainGovInstantiateMsg,
) -> AdapterResult {
    MEMBERS.save(deps.storage, &Members::new(&env))?;

    Ok(app.response("instantiate"))
}


/// Make a proposal to other chains for a state change
pub fn propose_item_state<'a, T: Serialize + DeserializeOwned>(
    storage: &mut dyn Storage,
    item: &Item<T>,
    proposed_state: T,
)  -> StdResult<()> {
    let key = std::str::from_utf8(item.as_slice())?.to_string();
    let pending = to_json_binary(&proposed_state)?;
    ITEMS_DATA_STATE.save(storage, (key, DataState::Initiated), &StateChange::Proposal(pending))
}

pub fn get_item_state<'a, T: Serialize + DeserializeOwned>(
    storage: &dyn Storage,
    item: &Item<T>,
) -> StdResult<DataState> {
    let key: StorageKey = std::str::from_utf8(item.as_slice())?.to_string();
    let items = ITEMS_DATA_STATE.prefix(key).keys(storage, None, None, Order::Ascending).take(1).collect::<StdResult<Vec<DataState>>>()?;
    Ok(items.first().map_or(DataState::Finalized, |state| state.to_owned()))
}