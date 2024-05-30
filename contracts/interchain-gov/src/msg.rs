use crate::{
    contract::InterchainGov,
    state::{Members},
};

use abstract_adapter::objects::chain_name::ChainName;
use cosmwasm_schema::QueryResponses;
use ibc_sync_state::{DataState, StateChange};

// This is used for type safety and re-exporting the contract endpoint structs.
abstract_adapter::adapter_msg_types!(InterchainGov, InterchainGovExecuteMsg, InterchainGovQueryMsg);
use crate::state::{Governance, GovernanceVote, Proposal, ProposalId, ProposalMsg, Vote};

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
    /// Execute the proposal state
    #[fn_name("execute_proposal")]
    Execute {
        prop_id: ProposalId,
    },
    ///Called by gov to vote on a proposal
    VoteProposal {
        prop_id: String,
        governance: Governance,
        vote: Vote,
    },
    /// Request results from remote chains
    RequestVoteResults {
        prop_id: String,
    },
    RequestGovVoteDetails {
        prop_id: String
    },
    SetAcceptGovInvite {
        /// only accept invites for groups with these members
        members: Members,
    },
    TallyProposal {
        prop_id: String,
    },
    TestAddMembers {
        members: Members
    },
    TemporaryRegisterRemoteGovModuleAddrs {
        modules: Vec<(ChainName, String)>
    },
}

#[cosmwasm_schema::cw_serde]
pub struct InterchainGovMigrateMsg {}

#[cosmwasm_schema::cw_serde]
pub enum InterchainGovIbcMsg {
    /// Called when this contract is being asked to join a Government
    JoinGov {
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
    JoinGov {
        proposed_to: ChainName,
    },
    FinalizeProposal {
        prop_hash: String,
        proposed_to: ChainName,
    },
    ProposeProposal {
        prop_hash: String,
        proposed_to: ChainName,
    },
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
    Proposals { proposal_ids: Vec<ProposalId> },
    #[returns(ProposalsResponse)]
    ListProposals {},
    #[returns(ProposalStateResponse)]
    ListProposalStates {},

    #[returns(ProposalResponse)]
    Proposal { prop_id: ProposalId },
    #[returns(Option<MapState>)]
    ProposalState { prop_id: ProposalId },
    /// Get the local vote
    #[returns(VoteResponse)]
    Vote {
        prop_id: ProposalId,
        // chain: Option<ChainName>
    },

    #[returns(VoteResultsResponse)]
    VoteResults {
        prop_id: ProposalId,
    },

}

/// App sudo messages
#[cosmwasm_schema::cw_serde]
pub enum InterchainGovSudoMsg {
    #[serde(rename = "kv_query_result")]
    KVQueryResult {
        query_id: u64,
    }
}


#[cosmwasm_schema::cw_serde]
pub struct ConfigResponse {}

#[cosmwasm_schema::cw_serde]
pub struct MembersResponse {
    pub members: Members,
}

#[cosmwasm_schema::cw_serde]
pub struct ProposalsResponse {
    pub proposals: Vec<ProposalResponse>,
}

#[cosmwasm_schema::cw_serde]
pub struct ProposalResponse {
    pub prop_id: ProposalId,
    pub prop: Proposal,
    pub state: Option<DataState>,
}

#[cosmwasm_schema::cw_serde]
pub struct ProposalStateResponse {
    pub state: Vec<MapState>,
}

#[cosmwasm_schema::cw_serde]
pub struct MapState {
    pub namespace: String,
    pub proposal_id: String,
    pub state: DataState,
    pub change: StateChange,
}

#[cosmwasm_schema::cw_serde]
pub struct VoteResponse {
    // TODO: maybe this field should not be included
    pub prop_id: ProposalId,
    pub chain: ChainName,
    pub governance: Governance,
    pub vote: Vote,
}

#[cosmwasm_schema::cw_serde]
pub struct VoteResultsResponse {
    pub prop_id: ProposalId,
    pub results: Vec<(ChainName, Option<GovernanceVote>)>
}