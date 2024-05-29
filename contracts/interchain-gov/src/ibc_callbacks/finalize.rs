use abstract_adapter::sdk::AbstractResponse;
use abstract_adapter::std::ibc::{CallbackResult, IbcResponseMsg};
use cosmwasm_std::{DepsMut, Env, from_json, MessageInfo, Order, StdResult};

use crate::contract::{AdapterResult, InterchainGov};
use crate::InterchainGovError;
use crate::msg::InterchainGovIbcMsg;
use crate::state::{DataState, Proposal, PROPOSALS, REMOTE_PROPOSAL_STATE};

/// Get a callback when a proposal is finalized
/// TODO: figure out how to abstract this state transition
pub fn finalize_callback(
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
                InterchainGovIbcMsg::FinalizeProposal {
                    prop_hash: prop_id,
                    chain,
                    ..
                } => {
                    if ibc_msg.msg.is_some() {
                        return Err(InterchainGovError::UnknownCallbackMessage(ibc_msg.id))
                    }

                    // Ensure that it was proposed
                    let prev_state = REMOTE_PROPOSAL_STATE.may_load(deps.storage, (prop_id.clone(), &chain))?;
                    if prev_state.clone().map_or(true, |state| !state.is_proposed()) {
                        return Err(InterchainGovError::InvalidProposalState {
                            prop_id: prop_id.clone(),
                            chain: chain.clone(),
                            expected: Some(DataState::Proposed),
                            actual: prev_state.clone(),
                        })
                    }

                    // Remove the pending state
                    REMOTE_PROPOSAL_STATE.remove(deps.storage, (prop_id.clone(), &chain));

                    // If we have no more pending states, we can update the proposal state to proposed
                    let prop_states = REMOTE_PROPOSAL_STATE.prefix(prop_id.clone()).keys(deps.storage, None, None, Order::Ascending).take(1).collect::<StdResult<Vec<_>>>()?;
                    if prop_states.is_empty() {
                        PROPOSALS.update(deps.storage, prop_id.clone(), |prop| -> Result<(Proposal, DataState), InterchainGovError> {
                            match prop {
                                Some((prop, _)) => {
                                    Ok((prop, DataState::Finalized))
                                },
                                None => Err(InterchainGovError::ProposalNotFound(prop_id.clone()))
                            }
                        })?;
                    }
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

    Ok(app.response("finalize_callback"))
}
