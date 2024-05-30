mod error;
mod item;
mod map;

use std::fmt::Display;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;
pub use error::SyncStateError;
pub use item::ItemStateSyncController;
pub use map::MapStateSyncController;

pub type SyncStateResult<T> = Result<T, SyncStateError>;

pub type StorageKey = String;
pub type Key = String;

#[cw_serde]
pub enum StateChange {
    Backup(Binary),
    Proposal(Binary),
}

/// Different statuses for a data item
#[cw_serde]
pub enum DataState {
    Initiated = 0,
    Proposed = 1,
}

impl DataState {
    pub fn to_num(&self) -> u8 {
        match self {
            DataState::Initiated => 0,
            DataState::Proposed => 1,
        }
    }

    pub fn from_num(num: u8) -> Option<Self> {
        match num {
            0 => Some(DataState::Initiated),
            1 => Some(DataState::Proposed),
            _ => None,
        }
    }

    pub fn is_proposed(&self) -> bool {
        matches!(self, DataState::Proposed)
    }
}

impl Display for DataState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataState::Initiated => write!(f, "Initiate"),
            DataState::Proposed => write!(f, "Proposed"),
        }
    }
}
