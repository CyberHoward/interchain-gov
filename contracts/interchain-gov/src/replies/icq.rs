use crate::contract::{AdapterResult, InterchainGov};
use crate::state::{PENDING_QUERIES, PENDING_REPLIES};
use crate::InterchainGovError;
use abstract_adapter::sdk::AbstractResponse;
use cosmwasm_std::{
    from_json, DepsMut, Env, Reply, StdError, StdResult, SubMsgResponse, SubMsgResult,
};
use neutron_query::neutron_sdk::bindings::msg::MsgRegisterInterchainQueryResponse;

pub fn icq_reply(deps: DepsMut, _env: Env, app: InterchainGov, reply: Reply) -> AdapterResult {
    println!("icq_reply: {:?}", reply);

    // remove the pending reply
    let pending = PENDING_REPLIES.load(deps.storage, reply.id)?;
    PENDING_REPLIES.remove(deps.storage, reply.id);

    let query_id = get_query_id(reply.result)?;

    // save to pending queries for use in the sudo handler
    PENDING_QUERIES.save(deps.storage, query_id, &pending)?;

    Ok(app
        .response("icq_reply")
        .add_attribute("query_id", query_id.to_string()))
}

pub fn get_query_id(msg_result: SubMsgResult) -> StdResult<u64> {
    let res: MsgRegisterInterchainQueryResponse = from_json(
        msg_result
            .into_result()
            .map_err(StdError::generic_err)?
            .data
            .ok_or_else(|| StdError::generic_err("no result"))?
            .as_slice(),
    )?;

    Ok(res.id)
}
