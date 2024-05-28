use abstract_adapter::{sdk::AbstractSdkError, std::AbstractError, AdapterError};
use cosmwasm_std::StdError;
use cw_asset::AssetError;
use cw_controllers::AdminError;
use thiserror::Error;
use crate::state::{DataStatus, ProposalId};

#[derive(Error, Debug, PartialEq)]
pub enum InterchainGovError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Abstract(#[from] AbstractError),

    #[error("{0}")]
    AbstractSdk(#[from] AbstractSdkError),

    #[error("{0}")]
    Asset(#[from] AssetError),

    #[error("{0}")]
    Admin(#[from] AdminError),

    #[error("{0}")]
    AdapterError(#[from] AdapterError),

    #[error("{0} are not implemented")]
    NotImplemented(String),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Proposal Already exists")]
    ProposalAlreadyExists(ProposalId),

    #[error("Data {key} not finalized. Status: {status:?}")]
    DataNotFinalized {
        key: String,
        status: DataStatus
    },
    #[error("Member {member} already exists")]
    MemberAlreadyExists { member: String },

    #[error("Remote {chain} not available")]
    UnknownRemoteHost { chain: String },
}
