use crate::{error::InterchainGovError, handlers, msg::{InterchainGovExecuteMsg, InterchainGovInstantiateMsg, InterchainGovQueryMsg}, ADAPTER_VERSION, MY_ADAPTER_ID, ibc_callbacks, replies};

use abstract_adapter::AdapterContract;
use cosmwasm_std::{Empty, Response};
use crate::dependencies::IBC_CLIENT_DEP;
use crate::ibc_callbacks::{REGISTER_VOTE_ID, PROPOSE_CALLBACK_ID, FINALIZE_CALLBACK_ID};
use crate::msg::InterchainGovSudoMsg;

/// The type of the adapter that is used to build your Adapter and access the Abstract SDK features.
pub type InterchainGov = AdapterContract<
    InterchainGovError,
    InterchainGovInstantiateMsg,
    InterchainGovExecuteMsg,
    InterchainGovQueryMsg,
    Empty,
    InterchainGovSudoMsg
>;
/// The type of the result returned by your Adapter's entry points.
pub type AdapterResult<T = Response> = Result<T, InterchainGovError>;

const INTERCHAIN_GOV: InterchainGov = InterchainGov::new(MY_ADAPTER_ID, ADAPTER_VERSION, None)
    .with_instantiate(handlers::instantiate_handler)
    .with_execute(handlers::execute_handler)
    .with_query(handlers::query_handler)
    .with_module_ibc(handlers::module_ibc_handler)
    .with_ibc_callbacks(&[
        (PROPOSE_CALLBACK_ID, ibc_callbacks::proposal_callback),
        (FINALIZE_CALLBACK_ID, ibc_callbacks::finalize_callback),
        (REGISTER_VOTE_ID, ibc_callbacks::vote_result_callback)
    ])
    // WE need a reply for every member
    .with_replies(&[
        (0, replies::icq_reply),
        (1, replies::icq_reply),
        (2, replies::icq_reply),
        (3, replies::icq_reply),
        (4, replies::icq_reply),
        (5, replies::icq_reply),
        (6, replies::icq_reply),
    ])
    .with_sudo(handlers::sudo_handler)
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
