use cw_storage_plus::{KeyDeserialize, PrimaryKey};
use cosmwasm_schema::cw_serde;

/// Local members to local data status
/// Remote member statuses

// Mabye we should do Map<Key, DataStatus>

/// Different statuses for a data item
#[cw_serde]
pub enum DataState {
    Initiated = 0,
    Proposed = 1,
    Finalized = 2,
}

impl<'a> PrimaryKey<'a> for DataState {
    type Prefix = ();
    type SubPrefix = ();
    type Suffix = ();
    type SuperSuffix = ();

    fn key(&self) -> Vec<cw_storage_plus::Key> {
        match self {
            DataState::Initiated => 0.key(),
            DataState::Proposed => 1.key(),
            DataState::Finalized => 2.key(),
        }
    }
}


impl KeyDeserialize for DataState {
    type Output = DataState;

    fn from_vec(value: Vec<u8>) -> cosmwasm_std::StdResult<Self::Output> {
        if value.len() != 1 {
            return Err(cosmwasm_std::StdError::generic_err("Invalid byte vector length"));
        }

        match value[0] {
            0 => Ok(DataState::Initiated),
            1 => Ok(DataState::Proposed),
            2 => Ok(DataState::Finalized),
            _ => Err(cosmwasm_std::StdError::generic_err("Invalid byte value")),
        }
    }
}


impl DataState {
    pub fn is_initiated(&self) -> bool {
        matches!(self, DataState::Initiated)
    }
    pub fn is_proposed(&self) -> bool {
        matches!(self, DataState::Proposed)
    }

    pub fn is_finalized(&self) -> bool {
        matches!(self, DataState::Finalized)
    }
}
