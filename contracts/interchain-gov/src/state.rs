use abstract_adapter::objects::chain_name::ChainName;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, CosmosMsg, Empty, Uint128};
use cw_storage_plus::{Item, Map};
use cw_utils::Expiration;
use dao_voting::{status::Status, threshold::Threshold, voting::Votes};

pub const PROPS: Map<String, (DataStatus, Binary)> = Map::new("props");
pub const MEMBERS: Item<Members> = Item::new("members");
pub const LOCAL_VOTE: Map<u32, Vote> = Item::new("local_vote");

/// Different statuses for a data item
#[cw_serde]
pub enum DataStatus {
    Initiate = 0,
    Proposed = 1,
    Finalized = 2,
}

#[cw_serde]
pub struct Members {
    pub members: Vec<String>,
}

// https://github.com/DA0-DA0/dao-contracts/blob/development/contracts/proposal/dao-proposal-single/src/proposal.rs
#[cw_serde]
pub struct Proposal {
    /// The title of the proposal
    pub title: String,
    /// The main body of the proposal text
    pub description: String,
    /// The address that created this proposal.
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
    /// The proposal status
    pub status: Status,
}

#[cw_serde]
enum Vote {
    Yes,
    NoVote
}