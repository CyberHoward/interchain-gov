pub mod proposal;
pub mod finalize;
mod vote_result;


pub const PROPOSE_CALLBACK_ID: &str = "propose_callback";
pub const FINALIZE_CALLBACK_ID: &str = "finalize_callback";
pub const REGISTER_VOTE_ID: &str = "register_vote";

pub use self::{proposal::proposal_callback, finalize::finalize_callback};
