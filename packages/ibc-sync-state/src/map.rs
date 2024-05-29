use cw_storage_plus::Map;

use crate::{item::{Key, StorageKey}, DataState, StateChange};

pub const MAPS_DATA_STATE: Map<(StorageKey, Key, DataState), StateChange> = Map::new("map_data");
