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

    #[error("Unknown callback message: {0}")]
    UnknownCallbackMessage(String),

    #[error("{prop_id} on {chain} already has state {state:?}")]
    PreExistingProposalState {
        prop_id: ProposalId,
        chain: ChainName,
        state: String,
    },
    #[error("Ibc failed: {0}")]
    IbcFailed(String),

    #[error("Proposal {0} not found")]
    ProposalNotFound(String),

    #[error("Proposal {prop_id} not {expected:?} on chain: {chain}, rather {actual:?}")]
    InvalidProposalState {
        prop_id: ProposalId,
        expected: Option<DataState>,
        actual: Option<DataState>,
        chain: ChainName,
    },

    #[error("Invalid chain. Expected: {expected:?}, Actual: {actual:?}")]
    WrongChain {
        expected: ChainName,
        actual: ChainName,
    },

    #[error("Proposal {0} expired")]
    ProposalExpired(String),

    #[error("Proposal {0} open")]
    ProposalStillOpen(String),

    #[error("Votes still pending for {prop_id} on {chains:?}")]
    VotesStillPending {
        prop_id: ProposalId,
        chains: Vec<ChainName>
    },

    #[error("Gov Votes still pending for {prop_id} on {chains:?}")]
    GovVotesStillPending {
        prop_id: ProposalId,
        chains: Vec<ChainName>
    },

    #[error("Missing vote results for {prop_id}, should call RequestVoteResults first")]
    MissingVoteResults {
        prop_id: ProposalId,
    },

    #[error("Votes already finalized {0}")]
    VotesAlreadyFinalized(String),

    #[error("Gov votes already queried {0}")]
    GovVotesAlreadyQueried(String),

    #[error("Unrequested vote result for {chain:?}, {prop_id}")]
    UnrequestedVote {
        chain: ChainName,
        prop_id: ProposalId
    },
    #[error("Existing vote result for {chain:?}, {prop_id}")]
    ExistingVoteResult {
        chain: ChainName,
        prop_id: ProposalId
    },
}
