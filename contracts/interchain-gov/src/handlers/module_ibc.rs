use abstract_adapter::objects::chain_name::ChainName;
use abstract_adapter::sdk::AbstractResponse;
use abstract_adapter::std::ibc::ModuleIbcMsg;
use cosmwasm_std::{from_json, DepsMut, Env};

use crate::{InterchainGovError, MY_ADAPTER_ID};
use crate::contract::{AdapterResult, InterchainGov};
use crate::data_state::DataState;
use crate::msg::InterchainGovIbcMsg;
use crate::state::{Proposal, PROPOSALS};

/// TODO
pub fn module_ibc_handler(
    deps: DepsMut,
    env: Env,
    app: InterchainGov,
    ibc_msg: ModuleIbcMsg,
) -> AdapterResult {
    println!("module_ibc_handler 1 : {:?}", ibc_msg);

    // First check that we received the message from the server
    if ibc_msg.source_module.id().ne(MY_ADAPTER_ID) {
        println!("UnauthorizedIbcModule: {:?}", ibc_msg.source_module.clone());
        return Err(InterchainGovError::UnauthorizedIbcModule(
            ibc_msg.source_module.clone(),
        ));
    };

    // check that the sender

    let ibc_msg: InterchainGovIbcMsg = from_json(&ibc_msg.msg)?;

    println!("parsed_msg: {:?}", ibc_msg);

    let our_chain = ChainName::new(&env);

    match ibc_msg {
        InterchainGovIbcMsg::ProposeProposal { prop_hash, prop, chain } => {
            // check that we were supposed to receive the message
            if chain != our_chain {
                return Err(InterchainGovError::WrongChain {
                    expected: our_chain,
                    actual: chain,
                });
            }
            // check that the proposal is not already existing
            PROPOSALS.update(deps.storage, prop_hash.clone(), |prev_prop| -> Result<(Proposal, DataState), InterchainGovError> {
                match prev_prop {
                    None => {
                        Ok((prop, DataState::Initiated))
                    }
                    Some(_) => {
                        Err(InterchainGovError::ProposalAlreadyExists(prop_hash))
                    }
                }
            })?;
            Ok(app.response("module_ibc").add_attribute("action", "propose"))
        }
        InterchainGovIbcMsg::FinalizeProposal { prop_hash: prop_id, chain } => {
            // check that we were supposed to receive the message
            if chain != our_chain {
                return Err(InterchainGovError::WrongChain {
                    expected: our_chain,
                    actual: chain,
                });
            }

            // check that the proposal is not already existing
            PROPOSALS.update(deps.storage, prop_id.clone(), |prev_prop| -> Result<(Proposal, DataState), InterchainGovError> {
                match prev_prop {
                    None => {
                        Err(InterchainGovError::InvalidProposalState {
                            prop_id,
                            expected: Some(DataState::Initiated),
                            actual: None,
                            chain: our_chain.clone()
                        })
                    }
                    Some((_prop, state)) => {
                       if !state.is_initiated() {
                            return Err(InterchainGovError::InvalidProposalState {
                                prop_id,
                                expected: Some(DataState::Initiated),
                                actual: Some(state),
                                chain: our_chain.clone()
                            });
                        }
                        Ok((_prop, DataState::Proposed))
                    }
                }
            })?;

            Ok(app.response("module_ibc").add_attribute("action", "finalize"))
        }
        _ => Err(InterchainGovError::UnauthorizedIbcMessage {}),
    }
}
