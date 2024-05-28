use crate::contract::{Intersync, IntersyncResult};

use abstract_app::traits::AbstractResponse;
use cosmwasm_std::{DepsMut, Env, Reply};

pub fn instantiate_reply(_deps: DepsMut, _env: Env, app: Intersync, _reply: Reply) -> IntersyncResult {
    Ok(app.response("instantiate_reply"))
}
