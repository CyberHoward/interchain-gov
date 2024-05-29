use cosmwasm_std::StdError;

use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum SyncStateError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Data {key} not finalized. Status: {state}")]
    DataNotFinalized { key: String, state: String },

    #[error("No proposed state for state transition")]
    NoProposedState,
}
