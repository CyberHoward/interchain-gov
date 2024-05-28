use crate::contract::InterchainGov;

use abstract_adapter::objects::AccountId;
use cosmwasm_schema::QueryResponses;

// This is used for type safety and re-exporting the contract endpoint structs.
abstract_adapter::adapter_msg_types!(InterchainGov, InterchainGovExecuteMsg, InterchainGovQueryMsg);
use crate::{state::Proposal};

/// App instantiate message
#[cosmwasm_schema::cw_serde]
pub struct InterchainGovInstantiateMsg {
    pub count: i32,
}

/// App execute messages
#[cosmwasm_schema::cw_serde]
#[derive(cw_orch::ExecuteFns)]
#[impl_into(ExecuteMsg)]
pub enum InterchainGovExecuteMsg {
    /// Called by gov when a chain wants to create a proposal
    CreateProposal {
        proposal: Proposal,
    },
    /// Can be called by any chain to trigger tallying
    TallyProposal {
        prop_hash: String,
    },
    ///Called by gov to vote on a proposal
    VoteProposal {
        prop_hash: String,
        vote: bool,
    }
}

#[cosmwasm_schema::cw_serde]
pub struct InterchainGovMigrateMsg {}

/// App query messages
#[cosmwasm_schema::cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
#[impl_into(QueryMsg)]
pub enum InterchainGovQueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(CountResponse)]
    Count {},
}

#[cosmwasm_schema::cw_serde]
pub struct ConfigResponse {}

#[cosmwasm_schema::cw_serde]
pub struct CountResponse {
    pub count: i32,
}
