pub mod api;
pub mod contract;
mod dependencies;
pub mod error;
mod handlers;
mod ibc_callbacks;
pub mod msg;
pub mod state;
pub mod replies;


pub use contract::interface::InterchainGovInterface;
pub use error::InterchainGovError;
pub use msg::{InterchainGovExecuteMsg, InterchainGovInstantiateMsg};

/// The version of your Adapter
pub const ADAPTER_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const MY_NAMESPACE: &str = "yournamespace";
pub const MY_ADAPTER_NAME: &str = "interchain-gov";
pub const MY_ADAPTER_ID: &str = const_format::formatcp!("{MY_NAMESPACE}:{MY_ADAPTER_NAME}");
