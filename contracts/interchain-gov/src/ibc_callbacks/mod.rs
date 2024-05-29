pub mod proposal;
pub mod finalize;


pub const PROPOSE_CALLBACK_ID: &str = "propose_callback";
pub const FINALIZE_CALLBACK_ID: &str = "finalize_callback";

pub use self::{proposal::proposal_callback, finalize::finalize_callback};
