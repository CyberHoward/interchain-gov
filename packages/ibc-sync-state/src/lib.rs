mod error;
mod item;
mod map;

pub use error::SyncStateError;
pub use item::{DataState, ItemStateSyncController, StateChange};

pub type SyncStateResult<T> = Result<T, SyncStateError>;
