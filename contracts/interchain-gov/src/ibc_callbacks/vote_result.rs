use abstract_adapter::sdk::AbstractResponse;
use abstract_adapter::std::ibc::{CallbackResult, IbcResponseMsg};
use cosmwasm_std::{DepsMut, Env, from_json, MessageInfo, QueryRequest, to_json_string, WasmQuery};

use crate::contract::{AdapterResult, InterchainGov};
use crate::InterchainGovError;
use crate::msg::{InterchainGovQueryMsg, VoteResponse};
use crate::state::{GovernanceVote, VOTE_RESULTS};

/// Get a callback when a proposal is finalized
/// TODO: figure out how to abstract this state transition
pub fn vote_result_callback(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    app: InterchainGov,
    ibc_msg: IbcResponseMsg,
) -> AdapterResult {

    println!("finalize_callback");

    match ibc_msg.result.clone() {
        CallbackResult::Query { query, ref result } => {
            println!("Query result");
            if result.is_err() {
                println!("Query error");
                return Err(InterchainGovError::IbcFailed("failed".to_string()));
            }

            // Retrieve the prop id from the original message
            let prop_id = match query {
                QueryRequest::Wasm(wasm) => {
                    match wasm {
                        WasmQuery::Smart {
                            contract_addr,
                            msg,
                        } => {
                            let msg: InterchainGovQueryMsg = from_json(msg)?;
                            match msg {
                                InterchainGovQueryMsg::Vote { prop_id } => {
                                    prop_id
                                },
                                _ => unimplemented!("InterchainGovQueryMsg")
                            }
                        },
                        _ => unimplemented!("WasmQuery")
                    }
                },
                _ => unimplemented!("QueryRequest")
            };

            // Get the result of the query
            let query_results = result.clone().unwrap();
            println!("Query results: {:?}", query_results);
            // TODO: check length
            let query_result = query_results.first().unwrap();
            let query_result: VoteResponse = from_json(query_result)?;

            // Update the response from the chain
            let chain = query_result.chain;

            VOTE_RESULTS.update(deps.storage, (prop_id.clone(), &chain), |prev_res| -> Result<Option<GovernanceVote>, InterchainGovError> {
                match prev_res {
                    Some(prev) => {
                        match prev {
                            Some(prev) => {
                                Err(InterchainGovError::ExistingVoteResult {
                                    prop_id: prop_id.clone(),
                                    chain: chain.clone(),
                                })
                            },
                            None => {
                                println!("Vote response added");
                                Ok(Some(GovernanceVote::new(query_result.governance, query_result.vote)))
                            }
                        }
                    },
                    None => {
                        Err(InterchainGovError::UnrequestedVote {
                            prop_id: prop_id.clone(),
                            chain: chain.clone(),
                        })
                    }
                }
            })?;
        },
        CallbackResult::Execute { initiator_msg, result } => {
            unreachable!("vote_result Execute callback")
        },
        CallbackResult::FatalError(e) => {
            println!("Fatal error");
            return Err(InterchainGovError::IbcFailed(e));
        }
    }

    Ok(app.response("vote_result_callback"))
}
