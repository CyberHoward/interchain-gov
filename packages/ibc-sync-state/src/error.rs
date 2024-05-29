use crate::item::{DataState, ProposalId};
use abstract_adapter::objects::chain_name::ChainName;
use abstract_adapter::objects::module::ModuleInfo;
use abstract_adapter::std::ibc::IbcResponseMsg;
use abstract_adapter::{sdk::AbstractSdkError, std::AbstractError, AdapterError};
use cosmwasm_std::StdError;
use cw_asset::AssetError;
use cw_controllers::AdminError;
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
