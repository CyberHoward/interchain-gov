use abstract_adapter::sdk::AbstractResponse;
use abstract_adapter::std::ibc::{CallbackResult, IbcResponseMsg};
use cosmwasm_std::{from_json, DepsMut, Env, MessageInfo};

use crate::contract::{AdapterResult, InterchainGov};
use crate::msg::InterchainGovIbcCallbackMsg;
use crate::state::{MEMBERS_STATE_SYNC, PROPOSAL_STATE_SYNC};
use crate::InterchainGovError;

/// Get a callback when a proposal is finalized
/// TODO: figure out how to abstract this state transition
pub fn finalize_callback(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    app: InterchainGov,
    ibc_msg: IbcResponseMsg,
) -> AdapterResult {
    println!("finalize_callback");
    match ibc_msg {
        IbcResponseMsg {
            id: _,
            msg: Some(callback_msg),
            result: CallbackResult::Execute { result: Ok(_), .. },
        } => {
            let callback_msg: InterchainGovIbcCallbackMsg = from_json(callback_msg)?;
            match callback_msg {
                InterchainGovIbcCallbackMsg::FinalizeProposal {
                    proposed_to,
                    prop_hash: _prop_id,
                } => {
                    PROPOSAL_STATE_SYNC.apply_ack(deps.storage, proposed_to)?;

                    if !PROPOSAL_STATE_SYNC.has_outstanding_acks(deps.storage)? {
                        // finalize my proposal
                        MEMBERS_STATE_SYNC.finalize_members(deps.storage, None)?;
                    }
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
        _ => panic!("{:?}", ibc_msg),
    }

    Ok(app.response("finalize_callback"))
}
