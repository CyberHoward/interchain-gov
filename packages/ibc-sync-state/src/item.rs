use std::fmt::Display;

use abstract_adapter::objects::chain_name::ChainName;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Decimal, Deps, DepsMut, Env, StdResult, Storage};
use cw_storage_plus::{Item, Map};
use cw_utils::Expiration;
use dao_voting::status::Status;
use dao_voting::threshold::{PercentageThreshold, Threshold};

use crate::error::SyncStateError;
use crate::SyncStateResult;

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
/// REMOTE
/// Instantiate: members ([B]) -> DNE
/// Proposal received: members([A, B]) -> proposed, Backup([A])
const ITEMS_DATA_STATE: Map<(StorageKey, u8), StateChange> = Map::new("item_data");

/// Local members to local data status
/// Remote member statuses

pub struct ItemStateSyncController {
    state_status_map: Map<'static, (StorageKey, u8), StateChange>,
}

impl ItemStateSyncController {
    pub fn new() -> Self {
        ItemStateSyncController {
            state_status_map: ITEMS_DATA_STATE,
        }
    }
    pub fn load(
        &self,
        storage: &dyn Storage,
        key: (impl Into<StorageKey>, DataState),
    ) -> SyncStateResult<StateChange> {
        self.state_status_map
            .load(storage, (key.0.into(), key.1.to_num()))
            .map_err(Into::into)
    }

    /// Load a state change
    /// Errors if no proposed state is found
    pub fn load_state_change(
        &self,
        storage: &dyn Storage,
        key: impl Into<StorageKey>,
    ) -> SyncStateResult<StateChange> {
        let key = key.into();
        let state_change = self
            .load(storage, (key.clone(), DataState::Initiate))
            .or_else(|_| self.load(storage, (key.clone(), DataState::Proposed)))
            .ok();
        state_change.ok_or(SyncStateError::NoProposedState)
    }

    pub fn assert_finalized(
        &self,
        storage: &dyn Storage,
        key: impl Into<StorageKey>,
    ) -> SyncStateResult<()> {
        let key = key.into();
        if self
            .state_status_map
            .has(storage, (key.clone(), DataState::Initiate.to_num()))
            || self
                .state_status_map
                .has(storage, (key.clone(), DataState::Proposed.to_num()))
        {
            return Err(SyncStateError::DataNotFinalized {
                key,
                state: "unknown".to_string(),
            });
        }
        Ok(())
    }

    pub fn propose_item_state(
        &self,
        storage: &mut dyn Storage,
        key: impl Into<StorageKey>,
        state: StateChange,
    ) -> SyncStateResult<()> {
        self.state_status_map
            .save(storage, (key.into(), DataState::Proposed.to_num()), &state)
            .map_err(Into::into)
    }

    pub fn initiate_item_state(
        &self,
        storage: &mut dyn Storage,
        key: impl Into<StorageKey>,
        state: StateChange,
    ) -> SyncStateResult<()> {
        self.state_status_map
            .save(storage, (key.into(), DataState::Initiate.to_num()), &state)
            .map_err(Into::into)
    }
    // Remove init / proposed states
    pub fn finalize_item_state(
        &self,
        storage: &mut dyn Storage,
        key: impl Into<StorageKey>,
    ) -> SyncStateResult<StateChange> {
        let key = key.into();
        // remove any of the states
        if let Some(change) = self
            .state_status_map
            .may_load(storage, (key.clone(), DataState::Initiate.to_num()))?
        {
            self.state_status_map
                .remove(storage, (key.clone(), DataState::Initiate.to_num()));
            return Ok(change);
        } else if let Some(change) = self
            .state_status_map
            .may_load(storage, (key.clone(), DataState::Proposed.to_num()))?
        {
            self.state_status_map
                .remove(storage, (key.clone(), DataState::Proposed.to_num()));
            return Ok(change);
        } else {
            return Err(SyncStateError::NoProposedState);
        }
    }
}

/// Different statuses for a data item
#[cw_serde]
pub enum DataState {
    Initiate = 0,
    Proposed = 1,
}

impl DataState {
    pub fn to_num(&self) -> u8 {
        match self {
            DataState::Initiate => 0,
            DataState::Proposed => 1,
        }
    }
    pub fn is_proposed(&self) -> bool {
        matches!(self, DataState::Proposed)
    }
}

impl Display for DataState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataState::Initiate => write!(f, "Initiate"),
            DataState::Proposed => write!(f, "Proposed"),
        }
    }
}
