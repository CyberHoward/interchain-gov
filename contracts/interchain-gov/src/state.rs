use std::fmt::Display;

use abstract_adapter::objects::chain_name::ChainName;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Decimal, Deps, DepsMut, Env, StdResult, Storage};
use cw_storage_plus::{Item, Map};
use cw_utils::Expiration;
use dao_voting::status::Status;
use dao_voting::threshold::{PercentageThreshold, Threshold};

pub type ProposalId = String;
pub type StorageKey = String;
pub type Key = String;

/// LOCAL
/// Instantiate: members ([A]) -> DNE
/// Propose adding B: members ([A]) -> initialized, Proposal([A, B])
/// Propose Callback: members([A, B]) -> finalized, None
///
/// REMOTE
/// Instantiate: members ([B]) -> DNE
/// Proposal received: members([A, B]) -> proposed, Backup([A])

pub const PROPOSAL_STATE: Map<(ProposalId, ChainName), DataState> = Map::new("prop_state");

pub(self) const MEMBERS_KEY: &'static str = "members";
pub(self) const MEMBERS: Item<Members> = Item::new(MEMBERS_KEY);

pub const LOCAL_VOTE: Map<ProposalId, Vote> = Map::new("vote");

pub const PROPOSALS: Map<ProposalId, Proposal> = Map::new("props");

pub const ALLOW_JOINING_GOV: Item<Members> = Item::new("alw");
/// Local members to local data status
/// Remote member statuses

pub mod members_sync_state {
    use cosmwasm_std::{to_json_binary, StdResult, Storage};
    use cw_storage_plus::Item;
    use ibc_sync_state::{DataState, ItemStateSyncController, StateChange, SyncStateError, SyncStateResult};

    use super::{Members, MEMBERS_KEY};

    pub struct MembersSyncState {
        item_state_controller: ItemStateSyncController,
        members: Item<'static, Members>,
    }

    impl MembersSyncState {
        pub fn new() -> Self {
            MembersSyncState {
                item_state_controller: ItemStateSyncController::new(),
                members: super::MEMBERS,
            }
        }

        pub fn load_members(&self, storage: &dyn Storage) -> StdResult<Members> {
            self.members.load(storage)
        }

        pub fn load_state_change(&self, storage: &dyn Storage) -> SyncStateResult<StateChange> {
            let state_change = self.item_state_controller.load_state_change(storage, MEMBERS_KEY)?;
        }

        pub fn initiate_members(
            &self,
            storage: &mut dyn Storage,
            members: Members,
        ) -> SyncStateResult<()> {
            self.item_state_controller
                .assert_finalized(storage, MEMBERS_KEY.to_string())?;
            self.members.save(storage, &members)?;
            self.item_state_controller
                .initiate_item_state(
                    storage,
                    MEMBERS_KEY.to_string(),
                    StateChange::Proposal(to_json_binary(&members)?),
                )
                .map_err(Into::into)
        }

        pub fn propose_members(
            &self,
            storage: &mut dyn Storage,
            members: Members,
        ) -> SyncStateResult<()> {
            self.members.save(storage, &members)?;
            self.item_state_controller.propose_item_state(
                storage,
                MEMBERS_KEY.to_string(),
                StateChange::Proposal(to_json_binary(&members)?),
            )
        }

        pub fn finalize_members(
            &self,
            storage: &mut dyn Storage,
            // Members to finalize
            // Uses initialized / proposed state None
            members: Option<Members>,
        ) -> SyncStateResult<()> {

            let members = {
                if let Some(members) = members {
                    members
                } else {
                    self.load_state_change(storage)?;
                }
                members
            };

            self.item_state_controller.finalize_item_state(
                storage,
                MEMBERS_KEY.to_string(),
                StateChange::Proposal(to_json_binary(&members)?),
            )
        }

        pub fn assert_finalized(&self, storage: &dyn Storage) -> SyncStateResult<()> {
            self.item_state_controller
                .assert_finalized(storage, MEMBERS_KEY.to_string())
        }

        // pub fn assert_proposed(&self, storage: &dyn Storage) -> SyncStateResult<()> {
        //     self.item_state_controller.assert_proposed(storage, MEMBERS_KEY.to_string())
        // }

        // pub fn assert_initiated(&self, storage: &dyn Storage) -> SyncStateResult<()> {
        //     self.item_state_controller.assert_initiated(storage, MEMBERS_KEY.to_string())
        // }
    }
}

#[cw_serde]
pub struct Members {
    pub members: Vec<ChainName>,
}

impl Members {
    pub fn new(env: &Env) -> Self {
        Members {
            members: vec![ChainName::new(env)],
        }
    }
}

#[non_exhaustive]
#[cw_serde]
pub enum ProposalAction {
    /// Add member action
    UpdateMembers {
        members: Members,
    },
    SyncItem {
        key: String,
        value: Binary,
    },
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
    pub action: ProposalAction,
}

impl Display for ProposalMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.title, self.description)
    }
}

impl ProposalMsg {
    pub fn hash(&self) -> String {
        let hash = <sha2::Sha256 as sha2::Digest>::digest(self.to_string());
        let prop_id = base64::prelude::BASE64_STANDARD.encode(hash.as_slice());
        prop_id
    }
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
    pub action: ProposalAction,
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
            action,
        } = proposal;

        Proposal {
            title,
            description,
            action,
            min_voting_period,
            expiration,
            proposer: proposer.to_string(),
            proposer_chain: ChainName::new(env),
            threshold: Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Percent(Decimal::percent(100)),
            },
            // status: Status::Open
        }
    }

    pub fn hash(&ProposalMsg) -> String {
        let hash = <sha2::Sha256 as sha2::Digest>::digest(self.to_string());
        let prop_id = base64::prelude::BASE64_STANDARD.encode(hash.as_slice());
        let prop = Proposal::new(proposal, &info.sender, &env);

    }
}

#[cw_serde]
pub enum Vote {
    Yes,
    NoVote,
}
