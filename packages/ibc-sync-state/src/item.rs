use cosmwasm_std::Storage;
use cw_storage_plus::Map;

use crate::error::SyncStateError;
use crate::{DataState, StateChange, StorageKey, SyncStateResult};

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
    pub const fn new() -> Self {
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
            .load(storage, (key.clone(), DataState::Initiated))
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
            .has(storage, (key.clone(), DataState::Initiated.to_num()))
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
            .save(storage, (key.into(), DataState::Initiated.to_num()), &state)
            .map_err(Into::into)
    }
    // Remove init / proposed states
    pub fn finalize_item_state(
        &self,
        storage: &mut dyn Storage,
        key: impl Into<StorageKey>,
        set: bool,
    ) -> SyncStateResult<()> {
        let key = key.into();
        // remove any of the states
        if let Some(_change) = self
            .state_status_map
            .may_load(storage, (key.clone(), DataState::Initiated.to_num()))?
        {
            self.state_status_map
                .remove(storage, (key.clone(), DataState::Initiated.to_num()));
            Ok(())
        } else if let Some(_change) = self
            .state_status_map
            .may_load(storage, (key.clone(), DataState::Proposed.to_num()))?
        {
            self.state_status_map
                .remove(storage, (key.clone(), DataState::Proposed.to_num()));
            return Ok(());
        } else if !set {
            return Err(SyncStateError::NoProposedState);
        } else {
            Ok(())
        }
    }
}
