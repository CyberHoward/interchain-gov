use crate::contract::InterchainGov;

use abstract_adapter::objects::chain_name::ChainName;
use cosmwasm_schema::QueryResponses;
use cosmwasm_std::Binary;

// This is used for type safety and re-exporting the contract endpoint structs.
abstract_adapter::adapter_msg_types!(InterchainGov, InterchainGovExecuteMsg, InterchainGovQueryMsg);
use crate::state::{DataState, Proposal, ProposalId, ProposalMsg};

/// App instantiate message
#[cosmwasm_schema::cw_serde]
pub struct InterchainGovInstantiateMsg {
}

/// App execute messages
#[cosmwasm_schema::cw_serde]
#[derive(cw_orch::ExecuteFns)]
#[impl_into(ExecuteMsg)]
pub enum InterchainGovExecuteMsg {
    /// Called by gov when a chain wants to create a proposal
    Propose {
        proposal: ProposalMsg,
    },
    /// Finalize the proposal state
    Finalize {
        prop_id: ProposalId,
    },
    ///Called by gov to vote on a proposal
    VoteProposal {
        prop_hash: String,
        vote: bool,
    },
    /// Can be called by any chain to trigger tallying
    TallyProposal {
        prop_hash: String,
    },
    // TODO: remove, just propose state change
    InviteMember {
        member: ChainName
    }
}

#[cosmwasm_schema::cw_serde]
pub struct InterchainGovMigrateMsg {}

#[cosmwasm_schema::cw_serde]
pub enum InterchainGovIbcMsg {
    UpdateMembers {
        members: Vec<ChainName>
    },
    ProposeProposal {
        prop_hash: String,
        prop: Proposal,
        /// information used to identify the recipient in the callback
        chain: ChainName,
    },
    FinalizeProposal {
        prop_hash: String,
        /// information used to identify the recipient in the callback
        chain: ChainName,
    }
    // SyncState {
    //     key: String,
    //     value: Binary
    // }
}

// #[non_exhaustive]
// #[cosmwasm_schema::cw_serde]
// pub enum InterchainGovIbcCallbackMsg {
//     ProposedProposal {
//         prop_hash: String,
//         chain: ChainName
//     }
// }

/// App query messages
#[cosmwasm_schema::cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
#[impl_into(QueryMsg)]
pub enum InterchainGovQueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(MembersResponse)]
    Members {}
}

#[cosmwasm_schema::cw_serde]
pub struct ConfigResponse {}

#[cosmwasm_schema::cw_serde]
pub struct MembersResponse {
    pub members: Vec<ChainName>,
    pub state: DataState,
}