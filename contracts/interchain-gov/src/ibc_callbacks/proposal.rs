use abstract_adapter::sdk::AbstractResponse;
use abstract_adapter::std::ibc::{CallbackResult, IbcResponseMsg};
use cosmwasm_std::{DepsMut, Env, from_json, MessageInfo};

use crate::contract::{AdapterResult, InterchainGov};
use crate::InterchainGovError;
use crate::msg::InterchainGovIbcMsg;
use crate::state::{DataState, PROPOSAL_STATE};

/// Get a callback when a proposal is synced
pub fn proposal_callback(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    app: InterchainGov,
    ibc_msg: IbcResponseMsg,
) -> AdapterResult {

    match ibc_msg.result {
        CallbackResult::Execute { initiator_msg, result } => {
            let initiator_msg: InterchainGovIbcMsg = from_json(initiator_msg)?;
            match initiator_msg {
                InterchainGovIbcMsg::ProposeProposal {
                    prop_hash: prop_id,
                    chain,
                    ..
                } => {
                    if ibc_msg.msg.some() {
                        return Err(InterchainGovError::UnknownCallbackMessage(ibc_msg.id))
                    }

                    if let Some(prev_state) = PROPOSAL_STATE.may_load(deps.storage, (prop_id.clone(), chain.clone()))? {
                        // TODO: check prev state
                        return Err(InterchainGovError::PreExistingProposalState {
                            prop_id: prop_id.clone(),
                            chain: chain.clone(),
                            state: prev_state
                        })
                    }

                    // TODO: check the previous proposal state
                    PROPOSAL_STATE.save(deps.storage, (prop_id, chain), &DataState::Proposed)?
                }
                // Wrong initiator message
                _ => unimplemented!()
            }
        },
        CallbackResult::Query { .. } => {
            // TODO: proper error
            unreachable!()
        },
        CallbackResult::FatalError(_) => {
            return Err(InterchainGovError::IbcFailed(ibc_msg));
        }
    }

    Ok(app.response("proposal_callback"))
}
