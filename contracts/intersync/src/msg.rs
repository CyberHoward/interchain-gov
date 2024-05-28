use crate::contract::Intersync;

use cosmwasm_schema::QueryResponses;

// This is used for type safety and re-exporting the contract endpoint structs.
abstract_app::app_msg_types!(Intersync, IntersyncExecuteMsg, IntersyncQueryMsg);

/// App instantiate message
#[cosmwasm_schema::cw_serde]
pub struct IntersyncInstantiateMsg {
    pub count: i32,
}

/// App execute messages
#[cosmwasm_schema::cw_serde]
#[derive(cw_orch::ExecuteFns)]
#[impl_into(ExecuteMsg)]
pub enum IntersyncExecuteMsg {
    /// Called by gov when a chain wants to create a proposal
    CreateProposal {

    },
    /// Can be called by any chain to trigger tallying
    TallyProposal {
    
    },
    ///Called by gov to vote on a proposal
    VoteProposal {

    }
}

#[cosmwasm_schema::cw_serde]
pub struct IntersyncMigrateMsg {}

/// App query messages
#[cosmwasm_schema::cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
#[impl_into(QueryMsg)]
pub enum IntersyncQueryMsg {
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
