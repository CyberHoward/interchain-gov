use abstract_adapter::sdk::AbstractResponse;
use abstract_adapter::std::ibc::{CallbackResult, IbcResponseMsg};
use cosmwasm_std::{from_json, DepsMut, Env, MessageInfo};

use crate::contract::{AdapterResult, InterchainGov};
use crate::msg::{InterchainGovIbcCallbackMsg, InterchainGovIbcMsg};
use crate::state::{MEMBERS_STATE_SYNC, PROPOSAL_STATE_SYNC};
use crate::InterchainGovError;

/// Get a callback when a proposal is synced
pub fn proposal_callback(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    app: InterchainGov,
    ibc_msg: IbcResponseMsg,
) -> AdapterResult {
    match ibc_msg {
        IbcResponseMsg {
            id: _,
            msg: Some(callback_msg),
            result: CallbackResult::Execute { result: Ok(_), .. },
        } => {
            let callback_msg: InterchainGovIbcCallbackMsg = from_json(callback_msg)?;
            match callback_msg {
                InterchainGovIbcCallbackMsg::JoinGov { proposed_to } => {
                    MEMBERS_STATE_SYNC.apply_ack(deps.storage, proposed_to)?;

                    if !MEMBERS_STATE_SYNC.has_outstanding_acks(deps.storage)? {
                        // finalize my proposal
                        MEMBERS_STATE_SYNC.finalize_members(deps.storage, None)?;
                        // send finalization to the other chain
                        // TODO: Send confirmation to members
                    }
                }
                InterchainGovIbcCallbackMsg::ProposeProposal {
                    prop_hash: prop_id,
                    proposed_to,
                    ..
                } => {
                    PROPOSAL_STATE_SYNC.apply_ack(deps.storage, proposed_to)?;

                    if PROPOSAL_STATE_SYNC.has_outstanding_acks(deps.storage)? {
                        PROPOSAL_STATE_SYNC.finalize_kv_state(deps.storage, prop_id, None)?
                    }
                }
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

    Ok(app.response("proposal_callback"))
}
