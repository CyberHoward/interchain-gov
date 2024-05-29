use cosmwasm_std::{from_json, to_json_binary, Storage};
use cw_storage_plus::{Map, PrimaryKey};
use serde::{de::DeserializeOwned, Serialize};

use crate::{DataState, Key, StateChange, StorageKey, SyncStateError, SyncStateResult};

pub const MAPS_DATA_STATE: Map<(StorageKey, Key, u8), StateChange> = Map::new("map_data");

pub struct MapStateSyncController<'a, K, V> {
    state_status_map: Map<'static, (StorageKey, Key, u8), StateChange>,
    map: Map<'a, K, V>,
}

impl<'a, K, V> MapStateSyncController<'a, K, V>
where
    V: Serialize + DeserializeOwned,
    K: PrimaryKey<'a> + Into<String> + Clone,
{
    pub const fn new(map: Map<'a, K, V>) -> Self {
        MapStateSyncController {
            state_status_map: MAPS_DATA_STATE,
            map,
        }
    }

    fn storage_key(&self) -> StorageKey {
        std::str::from_utf8(self.map.namespace())
            .unwrap()
            .to_string()
    }

    pub fn load_status(
        &self,
        storage: &dyn Storage,
        key: (impl Into<Key>, DataState),
    ) -> SyncStateResult<StateChange> {
        self.state_status_map
            .load(storage, (self.storage_key(), key.0.into(), key.1.to_num()))
            .map_err(Into::into)
    }

    pub fn load(&self, storage: &dyn Storage, key: K) -> SyncStateResult<V> {
        self.map.load(storage, key).map_err(Into::into)
    }

    pub fn may_load(&self, storage: &dyn Storage, key: K) -> SyncStateResult<Option<V>> {
        self.map.may_load(storage, key).map_err(Into::into)
    }

    pub fn has(&self, storage: &dyn Storage, key: K) -> bool {
        self.map.has(storage, key)
    }

    /// Load a state change
    /// Errors if no proposed state is found
    pub fn load_state_change(
        &self,
        storage: &dyn Storage,
        key: impl Into<Key>,
    ) -> SyncStateResult<StateChange> {
        let key = key.into();
        let state_change = self
            .load_status(storage, (key.clone(), DataState::Initiate))
            .or_else(|_| self.load_status(storage, (key.clone(), DataState::Proposed)))
            .ok();
        state_change.ok_or(SyncStateError::NoProposedState)
    }

    pub fn assert_finalized(
        &self,
        storage: &dyn Storage,
        key: impl Into<Key>,
    ) -> SyncStateResult<()> {
        let key = key.into();
        if self.state_status_map.has(
            storage,
            (
                self.storage_key(),
                key.clone(),
                DataState::Initiate.to_num(),
            ),
        ) || self.state_status_map.has(
            storage,
            (
                self.storage_key(),
                key.clone(),
                DataState::Proposed.to_num(),
            ),
        ) {
            return Err(SyncStateError::DataNotFinalized {
                key,
                state: "unknown".to_string(),
            });
        }
        Ok(())
    }

    pub fn propose_kv_state(
        &self,
        storage: &mut dyn Storage,
        key: impl Into<Key>,
        proposal_value: V,
    ) -> SyncStateResult<()> {
        self.state_status_map
            .save(
                storage,
                (self.storage_key(), key.into(), DataState::Proposed.to_num()),
                &StateChange::Proposal(to_json_binary(&proposal_value)?),
            )
            .map_err(Into::into)
    }

    pub fn initiate_kv_state(
        &self,
        storage: &mut dyn Storage,
        key: impl Into<Key>,
        state: StateChange,
    ) -> SyncStateResult<()> {
        let key = key.into();

        self.assert_finalized(storage, key.clone())?;

        self.state_status_map
            .save(
                storage,
                (self.storage_key(), key.into(), DataState::Initiate.to_num()),
                &state,
            )
            .map_err(Into::into)
    }
    // Remove init / proposed states
    pub fn finalize_kv_state(
        &self,
        storage: &mut dyn Storage,
        key: K,
        value: Option<V>,
    ) -> SyncStateResult<()> {
        let k = key.clone().into();

        let value = {
            if let Some(value) = value {
                value
            } else {
                let value: Result<V, _> = match self.load_state_change(storage, k.clone())? {
                    StateChange::Proposal(value) => Ok(from_json(&value)?),
                    _ => Err(SyncStateError::DataNotFinalized {
                        key: k.to_string(),
                        state: "Backup".to_string(),
                    }),
                };
                value?
            }
        };

        // Remove any of the states
        if self.state_status_map.has(
            storage,
            (self.storage_key(), k.clone(), DataState::Initiate.to_num()),
        ) {
            self.state_status_map.remove(
                storage,
                (self.storage_key(), k.clone(), DataState::Initiate.to_num()),
            );
        } else if self.state_status_map.has(
            storage,
            (self.storage_key(), k.clone(), DataState::Proposed.to_num()),
        ) {
            self.state_status_map.remove(
                storage,
                (self.storage_key(), k.clone(), DataState::Proposed.to_num()),
            );
        } else {
            return Err(SyncStateError::NoProposedState);
        }

        self.map.save(storage, key, &value)?;
        Ok(())
    }
}
