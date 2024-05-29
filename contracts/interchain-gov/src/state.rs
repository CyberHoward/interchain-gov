use abstract_adapter::objects::chain_name::ChainName;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Decimal, Env};
use cw_storage_plus::{Item, Map};
use cw_utils::Expiration;
use dao_voting::threshold::{PercentageThreshold, Threshold};
pub use crate::data_state::DataState;

pub type ProposalId = String;
pub type StorageKey = String;
pub type Key = String;

#[cw_serde]
pub enum StateChange {
    Backup(Binary),
    Proposal(Binary),
}

/// LOCAL
/// Instantiate: members ([A]) -> DNE
/// Propose adding B: members ([A]) -> initialized, Proposal([A, B])
/// Propose Callback: members([A, B]) -> finalized, None
///
///
/// REMOTE
/// Instantiate: members ([B]) -> DNE
/// Proposal received: members([A, B]) -> proposed, Backup([A])

// TODO: implement KEy
pub const ITEMS_DATA_STATE: Map<(StorageKey, DataState), StateChange> = Map::new("item_data");
pub const MAPS_DATA_STATE: Map<(StorageKey, Key, DataState), StateChange> = Map::new("map_data");
pub const REMOTE_PROPOSAL_STATE: Map<(ProposalId, &ChainName), DataState> = Map::new("prop_state");

pub const MEMBERS: Item<Members> = Item::new("members");

pub const LOCAL_VOTE: Map<ProposalId, Vote> = Map::new("vote");

pub const PROPOSALS: Map<ProposalId, (Proposal, DataState)> = Map::new("props");

#[cw_serde]
pub struct Members {
    pub members: Vec<ChainName>,
}

impl Members {
    pub fn new(env: &Env) -> Self {
        Members {
            members: vec![ChainName::new(env)]
        }
    }
}

#[non_exhaustive]
#[cw_serde]
pub enum ProposalAction {
    /// Add member action
    InviteMember {
        member: ChainName
    },
    SyncItem {
        key: String,
        value: Binary
    }
}

#[cw_serde]
pub struct ProposalMsg {
    /// The title of the proposal
    pub title: String,
    /// The main body of the proposal text
    pub description: String,
    /// The minimum amount of time this proposal must remain open for
    /// voting. The proposal may not pass unless this is expired or
    /// None.
    pub min_voting_period: Option<Expiration>,
    /// The the time at which this proposal will expire and close for
    /// additional votes.
    pub expiration: Expiration,
    /// A standard action that the group can run
    pub actions: Vec<ProposalAction>
}

// https://github.com/DA0-DA0/dao-contracts/blob/development/contracts/proposal/dao-proposal-single/src/proposal.rs
#[cw_serde]
pub struct Proposal {
    /// The title of the proposal
    pub title: String,
    /// The main body of the proposal text
    pub description: String,
    /// The address that created this proposal.
    /// Needs to be a string to be translatable.
    pub proposer: String,
    /// The chain that created this proposal
    pub proposer_chain: ChainName,
    /// Action that the group will perform
    pub actions: Vec<ProposalAction>,
    /// The minimum amount of time this proposal must remain open for
    /// voting. The proposal may not pass unless this is expired or
    /// None.
    pub min_voting_period: Option<Expiration>,
    /// The the time at which this proposal will expire and close for
    /// additional votes.
    pub expiration: Expiration,
    /// The threshold at which this proposal will pass.
    pub threshold: Threshold,
    // /// The proposal status
    // pub status: Status,
}

impl Proposal {
    pub fn new(proposal: ProposalMsg, proposer: &Addr, env: &Env) -> Self {

        let ProposalMsg {
            title,
            description,
            min_voting_period,
            expiration,
            actions
        } = proposal;

        Proposal {
            title,
            description,
            actions,
            min_voting_period,
            expiration,
            proposer: proposer.to_string(),
            proposer_chain: ChainName::new(env),
            threshold: Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Percent(Decimal::percent(100))
            },
            // status: Status::Open
        }
    }
}

#[cw_serde]
pub enum Vote {
    Yes,
    No,
    NoVote
}