use abstract_adapter::objects::chain_name::ChainName;
use cosmwasm_std::{from_json, to_json_binary, StdResult, Storage};
use cw_storage_plus::{Item, Map, PrimaryKey};
use serde::{de::DeserializeOwned, Serialize};

use crate::{DataState, Key, StateChange, StorageKey, SyncStateError, SyncStateResult};

pub const MAPS_DATA_STATE: Map<(StorageKey, Key, u8), StateChange> = Map::new("map_data");
pub const OUTSTANDING_ACKS: Item<Vec<ChainName>> = Item::new("map_acks");

pub struct MapStateSyncController<'a, K, V> {
    state_status_map: Map<'static, (StorageKey, Key, u8), StateChange>,
    map: Map<'a, K, V>,
    outstanding_acks: Item<'static, Vec<ChainName>>,
    // introduce outstanding acks
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
            outstanding_acks: OUTSTANDING_ACKS,
        }
    }

    pub const fn map(&self) -> &Map<'a, K, V> {
        &self.map
    }

    pub const fn state_status_map(&self) -> &Map<(StorageKey, Key, u8), StateChange> {
        &self.state_status_map
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

    pub fn data_state(&self, storage: &dyn Storage, key: impl Into<Key>) -> Option<DataState> {
        let key = key.into();
        if self.state_status_map.has(
            storage,
            (
                self.storage_key(),
                key.clone(),
                DataState::Initiated.to_num(),
            ),
        ) {
            return Some(DataState::Initiated);
        } else if self.state_status_map.has(
            storage,
            (
                self.storage_key(),
                key.clone(),
                DataState::Proposed.to_num(),
            ),
        ) {
            return Some(DataState::Proposed);
        }

        None
    }

    pub fn load(&self, storage: &dyn Storage, key: K) -> SyncStateResult<V> {
        self.map.load(storage, key).map_err(Into::into)
    }

    pub fn may_load(&self, storage: &dyn Storage, key: K) -> SyncStateResult<Option<V>> {
        self.map.may_load(storage, key).map_err(Into::into)
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
            .load_status(storage, (key.clone(), DataState::Initiated))
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
                DataState::Initiated.to_num(),
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

    fn assert_proposed(&self, storage: &dyn Storage, key: impl Into<Key>) -> SyncStateResult<()> {
        let key = key.into();
        if !self.state_status_map.has(
            storage,
            (
                self.storage_key(),
                key.clone(),
                DataState::Proposed.to_num(),
            ),
        ) {
            return Err(SyncStateError::DataNotProposed {
                key,
                state: "Unknown".to_string(),
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
        initiated_value: V,
        outstanding_acks: Vec<ChainName>,
    ) -> SyncStateResult<()> {
        let key = key.into();

        self.assert_finalized(storage, key.clone())?;

        self.state_status_map.save(
            storage,
            (
                self.storage_key(),
                key.into(),
                DataState::Initiated.to_num(),
            ),
            &StateChange::Proposal(to_json_binary(&initiated_value)?),
        )?;
        self.outstanding_acks.save(storage, &outstanding_acks)?;
        Ok(())
    }

    pub fn set_outstanding_finalization_acks(
        &self,
        storage: &mut dyn Storage,
        acks: Vec<ChainName>,
    ) -> SyncStateResult<()> {
        self.outstanding_acks.save(storage, &acks)?;
        Ok(())
    }

    // Remove init / proposed states
    // Errors if no proposed state is found or provided
    pub fn finalize_kv_state(
        &self,
        storage: &mut dyn Storage,
        key: K,
        value: Option<V>,
    ) -> SyncStateResult<()> {
        let k = key.clone().into();

        let (value, was_set) = {
            if let Some(value) = value {
                (value, true)
            } else {
                let value: Result<V, _> = match self.load_state_change(storage, k.clone())? {
                    StateChange::Proposal(value) => Ok(from_json(&value)?),
                    _ => Err(SyncStateError::DataNotFinalized {
                        key: k.to_string(),
                        state: "Backup".to_string(),
                    }),
                };
                (value?, false)
            }
        };

        // Remove any of the states
        if self.state_status_map.has(
            storage,
            (self.storage_key(), k.clone(), DataState::Initiated.to_num()),
        ) {
            self.state_status_map.remove(
                storage,
                (self.storage_key(), k.clone(), DataState::Initiated.to_num()),
            );
        } else if self.state_status_map.has(
            storage,
            (self.storage_key(), k.clone(), DataState::Proposed.to_num()),
        ) {
            self.state_status_map.remove(
                storage,
                (self.storage_key(), k.clone(), DataState::Proposed.to_num()),
            );
        } else if !was_set {
            return Err(SyncStateError::NoProposedState);
        }

        self.map.save(storage, key, &value)?;
        Ok(())
    }
}
