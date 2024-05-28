use crate::{
    contract::{Intersync, IntersyncResult},
    msg::IntersyncMigrateMsg,
};

use abstract_app::traits::AbstractResponse;
use cosmwasm_std::{DepsMut, Env};

/// Handle the app migrate msg
/// The top-level Abstract app does version checking and dispatches to this handler
pub fn migrate_handler(
    _deps: DepsMut,
    _env: Env,
    app: Intersync,
    _msg: IntersyncMigrateMsg,
) -> IntersyncResult {
    Ok(app.response("migrate"))
}
