use crate::{contract::InterchainGov, state::Members};

use abstract_adapter::objects::chain_name::ChainName;
use cosmwasm_schema::QueryResponses;

// This is used for type safety and re-exporting the contract endpoint structs.
abstract_adapter::adapter_msg_types!(
    InterchainGov,
    InterchainGovExecuteMsg,
    InterchainGovQueryMsg
);
use crate::state::{Proposal, ProposalMsg};

/// App instantiate message
#[cosmwasm_schema::cw_serde]
pub struct InterchainGovInstantiateMsg {
    pub accept_proposal_from_gov: Members,
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
    /// Finalize the proposal state
    #[fn_name("execute_proposal")]
    Execute {
        prop_id: ProposalId,
    },
    ///Called by gov to vote on a proposal
    VoteProposal { prop_hash: String, vote: bool },
    /// Can be called by any chain to trigger tallying
    ProposeGovMembers {
        /// Propose these new members
        members: Members,
    },
    AcceptGovInvite {
        /// only accept invites for groups with these members
        members: Members,
    },
    TallyProposal {
        prop_id: String,
    },
}

#[cosmwasm_schema::cw_serde]
pub struct InterchainGovMigrateMsg {}

#[cosmwasm_schema::cw_serde]
pub enum InterchainGovIbcMsg {
    /// Called when this contract is being asked to join a Government
    JoinGovProposal {
        // All the members in the governance (including this chain)
        members: Members,
    },
    ProposeProposal {
        prop_hash: String,
        prop: Proposal,
        /// information used to identify the recipient in the callback
        chain: ChainName,
    },
    FinalizeProposal {
        prop_hash: String,
    },
}

#[non_exhaustive]
#[cosmwasm_schema::cw_serde]
pub enum InterchainGovIbcCallbackMsg {
    JoinGovProposal { proposed_to: ChainName },
    FinalizeProposal { prop_hash: String, proposed_to: ChainName },
    ProposeProposal { prop_hash: String, proposed_to: ChainName },
}

/// App query messages
#[cosmwasm_schema::cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
#[impl_into(QueryMsg)]
pub enum InterchainGovQueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(MembersResponse)]
    Members {},
    // #[returns(PendingProposalStates)]
    // PendingProposals {},
    #[returns(ProposalsResponse)]
    ListProposals {},
    #[returns(ProposalResponse)]
    Proposal {
        prop_id: ProposalId,
    }
}

#[cosmwasm_schema::cw_serde]
pub struct ConfigResponse {}

#[cosmwasm_schema::cw_serde]
pub struct MembersResponse {
    pub members: Members,
    pub state: DataState,
}

#[cosmwasm_schema::cw_serde]
pub struct ProposalsResponse {
    pub proposals: Vec<ProposalResponse>
}

#[cosmwasm_schema::cw_serde]
pub struct ProposalResponse {
    pub prop_id: ProposalId,
    pub prop: Proposal,
    pub state: DataState,
}
