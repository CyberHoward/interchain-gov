use crate::{
    contract::{Intersync, IntersyncResult},
    msg::IntersyncExecuteMsg,
    state::{CONFIG, COUNT},
};

use abstract_app::traits::AbstractResponse;
use cosmwasm_std::{DepsMut, Env, MessageInfo};

pub fn execute_handler(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    app: Intersync,
    msg: IntersyncExecuteMsg,
) -> IntersyncResult {
    match msg {
        IntersyncExecuteMsg::UpdateConfig {} => update_config(deps, info, app),
        IntersyncExecuteMsg::Increment {} => increment(deps, app),
        IntersyncExecuteMsg::Reset { count } => reset(deps, info, count, app),
    }
}

/// Update the configuration of the app
fn update_config(deps: DepsMut, msg_info: MessageInfo, app: Intersync) -> IntersyncResult {
    // Only the admin should be able to call this
    app.admin.assert_admin(deps.as_ref(), &msg_info.sender)?;
    let mut _config = CONFIG.load(deps.storage)?;

    Ok(app.response("update_config"))
}

fn increment(deps: DepsMut, app: Intersync) -> IntersyncResult {
    COUNT.update(deps.storage, |count| IntersyncResult::Ok(count + 1))?;

    Ok(app.response("increment"))
}

fn reset(deps: DepsMut, info: MessageInfo, count: i32, app: Intersync) -> IntersyncResult {
    app.admin.assert_admin(deps.as_ref(), &info.sender)?;
    COUNT.save(deps.storage, &count)?;

    Ok(app.response("reset"))
}
