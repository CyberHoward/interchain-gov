use abstract_adapter::sdk::AbstractResponse;
use abstract_adapter::std::ibc::{CallbackResult, IbcResponseMsg};
use cosmwasm_std::{from_json, DepsMut, Env, MessageInfo, Order, StdResult};

use crate::contract::{AdapterResult, InterchainGov};
use crate::msg::{InterchainGovIbcCallbackMsg, InterchainGovIbcMsg};
use crate::state::{Proposal, MEMBERS_STATE_SYNC};
use crate::InterchainGovError;

/// Get a callback when a proposal is finalized
/// TODO: figure out how to abstract this state transition
pub fn finalize_callback(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    app: InterchainGov,
    ibc_msg: IbcResponseMsg,
) -> AdapterResult {
    println!("finalize_callback");
    match ibc_msg {
        IbcResponseMsg {
            id: _,
            msg: Some(callback_msg),
            result:
                CallbackResult::Execute {
                    initiator_msg,
                    result: Ok(_),
                },
        } => {
            let callback_msg: InterchainGovIbcCallbackMsg = from_json(callback_msg)?;
            match callback_msg {
                InterchainGovIbcCallbackMsg::FinalizeProposal {
                    proposed_to,
                    prop_hash: prop_id,
                } => {
                    MEMBERS_STATE_SYNC.apply_ack(deps.storage, proposed_to)?;

                    println!("Finalized proposal {:?}, all instances updated", prop_id);
                }
                // Wrong callback message
                _ => unimplemented!(),
            }
        }
        IbcResponseMsg {
            result: CallbackResult::Execute { result: Err(e), .. },
            ..
        } => {
            return Err(InterchainGovError::IbcFailed(e));
        }
        _ => panic!("unexpected callback result"),
    }

    Ok(app.response("finalize_callback"))
}
