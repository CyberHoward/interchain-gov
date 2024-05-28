use crate::{
    error::IntersyncError,
    handlers,
    msg::{IntersyncExecuteMsg, IntersyncInstantiateMsg, IntersyncMigrateMsg, IntersyncQueryMsg},
    replies::{self, INSTANTIATE_REPLY_ID},
    APP_VERSION, MY_APP_ID,
};

use abstract_app::AppContract;
use cosmwasm_std::Response;

/// The type of the result returned by your app's entry points.
pub type IntersyncResult<T = Response> = Result<T, IntersyncError>;

/// The type of the app that is used to build your app and access the Abstract SDK features.
pub type Intersync =
    AppContract<IntersyncError, IntersyncInstantiateMsg, IntersyncExecuteMsg, IntersyncQueryMsg, IntersyncMigrateMsg>;

const APP: Intersync = Intersync::new(MY_APP_ID, APP_VERSION, None)
    .with_instantiate(handlers::instantiate_handler)
    .with_execute(handlers::execute_handler)
    .with_query(handlers::query_handler)
    .with_migrate(handlers::migrate_handler)
    .with_dependencies(&[])
    .with_replies(&[(INSTANTIATE_REPLY_ID, replies::instantiate_reply)]);

// Export handlers
#[cfg(feature = "export")]
abstract_app::export_endpoints!(APP, Intersync);

abstract_app::cw_orch_interface!(APP, Intersync, IntersyncInterface);

// TODO: add to docmuentation
// https://linear.app/abstract-sdk/issue/ABS-414/add-documentation-on-dependencycreation-trait
#[cfg(not(target_arch = "wasm32"))]
impl<Chain: cw_orch::environment::CwEnv> abstract_interface::DependencyCreation
    for crate::IntersyncInterface<Chain>
{
    type DependenciesConfig = cosmwasm_std::Empty;
}
