use std::fmt::Display;

use abstract_adapter::objects::chain_name::ChainName;
use base64::Engine;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Env, Uint128};
use cw_storage_plus::{Item, Map};
use cw_utils::Expiration;
use dao_voting::threshold::{PercentageThreshold, Threshold};
use ibc_sync_state::MapStateSyncController;
use members_sync_state::MembersSyncState;

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

// pub const PROPOSAL_STATE: Map<(ProposalId, ChainName), DataState> = Map::new("prop_state");

pub const MEMBERS_KEY: &str = "members";
pub const MEMBERS: Item<Members> = Item::new(MEMBERS_KEY);
pub const MEMBERS_STATE_SYNC: MembersSyncState = MembersSyncState::new();
pub const OUTSTANDING_ACKS: Item<Vec<ChainName>> = Item::new("acks");
pub const ALLOW_JOINING_GOV: Item<Members> = Item::new("alw");

// TODO: should we actually have these as separate maps?

pub const VOTE: Map<ProposalId, GovernanceVote> = Map::new("vote");
pub const VOTES: Map<ProposalId, (ChainName, GovernanceVote)> = Map::new("votes");

/// Remote vote results, None = requested
pub const VOTE_RESULTS: Map<(ProposalId, &ChainName), Option<GovernanceVote>> =
    Map::new("vote_results");
/// Pending vote queries
pub const GOV_VOTE_QUERIES: Map<(ProposalId, &ChainName), Option<TallyResult>> =
    Map::new("pending_queries");
pub const TEMP_REMOTE_GOV_MODULE_ADDRS: Map<&ChainName, String> =
    Map::new("temp_remote_gov_module_addrs");

/// Map queryid -> (chain, prop_id)
pub const PENDING_REPLIES: Map<u64, (ChainName, ProposalId)> = Map::new("pending_replies");
/// Map queryid -> chain
pub const PENDING_QUERIES: Map<u64, (ChainName, ProposalId)> = Map::new("pending_replies");
const PROPOSALS: Map<ProposalId, (Proposal, Vote)> = Map::new("props");
pub const PROPOSAL_STATE_SYNC: MapStateSyncController<'_, ProposalId, (Proposal, Vote)> =
    MapStateSyncController::new(PROPOSALS);

pub const FINALIZED_PROPOSALS: Map<ProposalId, (Proposal, ProposalOutcome)> =
    Map::new("finalized_props");

/// Local members to local data status
/// Remote member statuses

pub mod members_sync_state {
    use abstract_adapter::objects::chain_name::ChainName;
    use cosmwasm_std::{from_json, to_json_binary, Env, StdResult, Storage};
    use cw_storage_plus::Item;
    use ibc_sync_state::{ItemStateSyncController, StateChange, SyncStateError, SyncStateResult};

    use super::{Members, MEMBERS_KEY};

    pub struct MembersSyncState {
        item_state_controller: ItemStateSyncController,
        members: Item<'static, Members>,
        outstanding_acks: Item<'static, Vec<ChainName>>,
    }

    impl MembersSyncState {
        pub const fn new() -> Self {
            MembersSyncState {
                item_state_controller: ItemStateSyncController::new(),
                members: super::MEMBERS,
                outstanding_acks: super::OUTSTANDING_ACKS,
            }
        }

        pub fn load_members(&self, storage: &dyn Storage) -> StdResult<Members> {
            self.members.load(storage)
        }

        pub fn save_members(&self, storage: &mut dyn Storage, members: &Members) -> StdResult<()> {
            self.members.save(storage, members)
        }

        pub fn external_members(
            &self,
            storage: &dyn Storage,
            env: &Env,
        ) -> SyncStateResult<Members> {
            let mut members = self.load_members(storage)?;
            members.members.retain(|m| m != &ChainName::new(env));
            Ok(members)
        }

        pub fn load_state_change(&self, storage: &dyn Storage) -> SyncStateResult<StateChange> {
            self.item_state_controller
                .load_state_change(storage, MEMBERS_KEY)
        }

        // Instantiate state change and register outstanding receipts
        pub fn initiate_members(
            &self,
            storage: &mut dyn Storage,
            env: &Env,
            members: Members,
        ) -> SyncStateResult<()> {
            self.item_state_controller
                .assert_finalized(storage, MEMBERS_KEY.to_string())?;
            self.item_state_controller.initiate_item_state(
                storage,
                MEMBERS_KEY.to_string(),
                StateChange::Proposal(to_json_binary(&members)?),
            )?;
            let members = self.external_members(storage, env)?;
            self.outstanding_acks.save(storage, &members.members)?;
            Ok(())
        }

        pub fn apply_ack(
            &self,
            storage: &mut dyn Storage,
            chain: ChainName,
        ) -> SyncStateResult<Option<ChainName>> {
            let mut acks = self.outstanding_acks.load(storage)?;
            // find chain in acks and remove it
            let receipt_i = acks.iter().position(|c| c == &chain);
            let ack_chain = match receipt_i {
                Some(receipt_i) => acks.remove(receipt_i),
                None => return Ok(None),
            };

            self.outstanding_acks.save(storage, &acks)?;
            Ok(Some(ack_chain))
        }

        pub fn has_outstanding_acks(&self, storage: &dyn Storage) -> StdResult<bool> {
            let acks = self.outstanding_acks.load(storage)?;
            Ok(!acks.is_empty())
        }

        pub fn propose_members(
            &self,
            storage: &mut dyn Storage,
            members: Members,
        ) -> SyncStateResult<()> {
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
            let (members, set) = {
                if let Some(members) = members {
                    (members, true)
                } else {
                    let members: Result<Members, _> = match self.load_state_change(storage)? {
                        StateChange::Proposal(members) => Ok(from_json(members)?),
                        _ => Err(SyncStateError::DataNotFinalized {
                            key: MEMBERS_KEY.to_string(),
                            state: "Backup".to_string(),
                        }),
                    };
                    (members?, false)
                }
            };

            self.members.save(storage, &members)?;

            self.item_state_controller.finalize_item_state(
                storage,
                MEMBERS_KEY.to_string(),
                set,
            )?;
            Ok(())
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

impl From<Vec<ChainName>> for Members {
    fn from(members: Vec<ChainName>) -> Self {
        Members { members }
    }
}

#[non_exhaustive]
#[cw_serde]
pub enum ProposalAction {
    Signal,
    UpdateMembers { members: Members },
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
        }
    }
}

#[non_exhaustive]
#[cw_serde]
pub enum Vote {
    Yes,
    No,
    NoVote,
}

#[non_exhaustive]
#[cw_serde]
pub enum Governance {
    CosmosSDK {
        proposal_id: u64,
    },
    DaoDao {
        dao_address: String,
        proposal_id: u64,
    },
    Manual {},
}

#[cw_serde]
pub struct GovernanceVote {
    pub vote: Vote,
    pub governance: Governance,
}

impl GovernanceVote {
    pub fn new(governance: Governance, vote: Vote) -> Self {
        GovernanceVote { vote, governance }
    }
}
#[cw_serde]
pub struct ProposalOutcome {
    pub passed: bool,
    pub votes_for: u8,
    pub votes_against: u8,
}

/// Tally result from the other chain
#[cw_serde]
pub struct TallyResult {
    pub yes: Uint128,
    pub no: Uint128,
    pub abstain: Uint128,
    pub no_with_veto: Uint128,
}

impl Vote {
    pub fn new(vote: bool) -> Self {
        if vote {
            Vote::Yes
        } else {
            Vote::No
        }
    }
}
