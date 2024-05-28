use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, CosmosMsg, Empty, Uint128};
use cw_storage_plus::{Item, Map};
use cw_utils::Expiration;
use dao_voting::{threshold::Threshold, voting::Votes};

#[cosmwasm_schema::cw_serde]
pub struct Config {}

pub const CONFIG: Item<Config> = Item::new("config");
pub const COUNT: Item<i32> = Item::new("count");

pub const DATA: Map<String, (Status, Binary)> = Map::new("gov_data");
pub const ITEM: Item<Members> = Item::new("members");

#[cw_serde]
pub enum Status {
    Initiate = 0,
    Stable = 1,
    Locked = 2,
}

#[cw_serde]
pub struct Members {
    pub status: Status,
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
    /// The block height at which this proposal was created. Voting
    /// power queries should query for voting power at this block
    /// height.
    pub start_height: u64,
    /// The minimum amount of time this proposal must remain open for
    /// voting. The proposal may not pass unless this is expired or
    /// None.
    pub min_voting_period: Option<Expiration>,
    /// The the time at which this proposal will expire and close for
    /// additional votes.
    pub expiration: Expiration,
    /// The threshold at which this proposal will pass.
    pub threshold: Threshold,
    /// The total amount of voting power at the time of this
    /// proposal's creation.
    pub total_power: Uint128,
    /// The messages that will be executed should this proposal pass.
    pub msgs: Vec<CosmosMsg<Empty>>,
    /// The proposal status
    pub status: Status,
    /// Votes on a particular proposal
    pub votes: Votes,
    /// Whether or not revoting is enabled. If revoting is enabled, a proposal
    /// cannot pass until the voting period has elapsed.
    pub allow_revoting: bool,
}
