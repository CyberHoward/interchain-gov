use crate::state::ProposalId;
use abstract_adapter::objects::chain_name::ChainName;
use abstract_adapter::objects::module::ModuleInfo;

use abstract_adapter::{sdk::AbstractSdkError, std::AbstractError, AdapterError};
use cosmwasm_std::StdError;
use cw_asset::AssetError;
use cw_controllers::AdminError;
use ibc_sync_state::{DataState, SyncStateError};
use thiserror::Error;

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
    SyncState(#[from] SyncStateError),

    #[error("{0}")]
    AdapterError(#[from] AdapterError),

    #[error("{0} are not implemented")]
    NotImplemented(String),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Proposal Already exists")]
    ProposalAlreadyExists(ProposalId),

    #[error("Data {key} not finalized. Status: {state}")]
    DataNotFinalized { key: String, state: DataState },

    #[error("Member {member} already exists")]
    MemberAlreadyExists { member: String },

    #[error("Remote {host} not available")]
    UnknownRemoteHost { host: String },

    #[error("Unauthorized IBC message from module: {0}")]
    UnauthorizedIbcModule(ModuleInfo),

    #[error("Unauthorized IBC message")]
    UnauthorizedIbcMessage,

    // #[error("IBC Message failed: {:?}")]
    // IbcFailed(IbcResponseMsg),
    #[error("Unknown callback message: {0}")]
    UnknownCallbackMessage(String),

    #[error("{prop_id} on {chain} already has state {state}")]
    PreExistingProposalState {
        prop_id: ProposalId,
        chain: ChainName,
        state: String,
    },
    #[error("Ibc failed: {0}")]
    IbcFailed(String),
}
