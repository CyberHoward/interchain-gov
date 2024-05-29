pub mod finalize;
pub mod proposal;

pub const PROPOSE_CALLBACK_ID: &str = "propose_callback";
pub const FINALIZE_CALLBACK_ID: &str = "finalize_callback";

pub use self::{finalize::finalize_callback, proposal::proposal_callback};
