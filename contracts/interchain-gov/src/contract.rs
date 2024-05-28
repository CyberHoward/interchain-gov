use crate::{
    error::InterchainGovError,
    handlers,
    msg::{InterchainGovExecuteMsg, InterchainGovInstantiateMsg, InterchainGovQueryMsg},
    ADAPTER_VERSION, MY_ADAPTER_ID,
};

use abstract_adapter::AdapterContract;
use cosmwasm_std::Response;
use crate::dependencies::IBC_CLIENT_DEP;

/// The type of the adapter that is used to build your Adapter and access the Abstract SDK features.
pub type InterchainGov = AdapterContract<
    InterchainGovError,
    InterchainGovInstantiateMsg,
    InterchainGovExecuteMsg,
    InterchainGovQueryMsg,
>;
/// The type of the result returned by your Adapter's entry points.
pub type AdapterResult<T = Response> = Result<T, InterchainGovError>;

const INTERCHAIN_GOV: InterchainGov = InterchainGov::new(MY_ADAPTER_ID, ADAPTER_VERSION, None)
    .with_instantiate(handlers::instantiate_handler)
    .with_execute(handlers::execute_handler)
    .with_query(handlers::query_handler)
    .with_module_ibc()
    .with_dependencies(&[IBC_CLIENT_DEP]);

// Export handlers
#[cfg(feature = "export")]
abstract_adapter::export_endpoints!(INTERCHAIN_GOV, InterchainGov);

abstract_adapter::cw_orch_interface!(
    INTERCHAIN_GOV,
    InterchainGov,
    InterchainGovInstantiateMsg,
    InterchainGovInterface
);
