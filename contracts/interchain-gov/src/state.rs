use abstract_adapter::objects::chain_name::ChainName;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Decimal, Env};
use cw_storage_plus::{Item, Map};
use cw_utils::Expiration;
use dao_voting::{status::Status};
use dao_voting::threshold::{PercentageThreshold, Threshold};


pub type ProposalId = u32;

pub const LOCAL_VOTE: Map<ProposalId, Vote> = Map::new("our_vote");

pub const PROPS: Map<ProposalId, (DataState, Binary)> = Map::new("props");


/// Local members to local data status
pub const MEMBERS: Item<(Members, DataState)> = Item::new("members");
/// Remote member statuses
pub const MEMBER_UPDATES: Map<ChainName, DataState> = Map::new("member_updates");

// Mabye we should do Map<Key, DataStatus>

/// Different statuses for a data item
#[cw_serde]
pub enum DataState {
    Initiate = 0,
    Proposed = 1,
    Finalized = 2,
}

impl DataState {
    pub fn is_proposed(&self) -> bool {
        matches!(self, DataState::Proposed)
    }

    pub fn is_finalized(&self) -> bool {
        matches!(self, DataState::Finalized)
    }
}

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
    /// The minimum amount of time this proposal must remain open for
    /// voting. The proposal may not pass unless this is expired or
    /// None.
    pub min_voting_period: Option<Expiration>,
    /// The the time at which this proposal will expire and close for
    /// additional votes.
    pub expiration: Expiration,
    /// The threshold at which this proposal will pass.
    pub threshold: Threshold,
    /// The proposal status
    pub status: Status,
}

impl Proposal {
    pub fn new(proposal: ProposalMsg, proposer: &Addr, env: &Env) -> Self {
        Proposal {
            title: proposal.title,
            description: proposal.description,
            proposer: proposer.to_string(),
            proposer_chain: ChainName::new(env),
            min_voting_period: proposal.min_voting_period,
            threshold: Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Percent(Decimal::percent(100))
            },
            expiration: proposal.expiration,
            status: Status::Open
        }
    }
}

#[cw_serde]
pub enum Vote {
    Yes,
    NoVote
}