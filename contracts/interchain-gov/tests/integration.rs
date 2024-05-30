use std::str::FromStr;

use abstract_adapter::objects::chain_name::ChainName;
use ibc_sync_state::DataState;
use interchain_gov::{
    contract::interface::InterchainGovInterface,
    msg::{InterchainGovInstantiateMsg, InterchainGovQueryMsgFns},
    state::Members,
    InterchainGovExecuteMsg, MY_ADAPTER_ID, MY_NAMESPACE,
};

use abstract_adapter::std::manager::ExecuteMsgFns;
use abstract_adapter::std::{adapter, adapter::AdapterRequestMsg, objects::namespace::Namespace};
use abstract_client::{AbstractClient, Account, Application, Environment, Publisher};
use abstract_cw_orch_polytone::Polytone;
// Use prelude to get all the necessary imports
use cw_orch_interchain::{ChannelCreator, MockBech32InterchainEnv, Starship};
use cw_orch_interchain::InterchainEnv;

use cw_orch::{anyhow, prelude::*};
// use cw_orch_interchain::{prelude::*};
use speculoos::prelude::*;

use abstract_interchain_tests::setup::ibc_connect_polytone_and_abstract;
use cw_orch::mock::cw_multi_test::AppResponse;
use cw_orch::tokio::runtime::Runtime;
use cw_utils::Expiration;
use interchain_gov::state::{ProposalAction, ProposalId, ProposalMsg};
// use cw_orch_interchain::MockBech32InterchainEnv;

const A_CHAIN_ID: &str = "neutron-1";
const B_CHAIN_ID: &str = "juno-1";
const C_CHAIN_ID: &str = "stargaze-1";
const A_CHAIN_ADDR: &str = "neutron18k2uq7srsr8lwrae6zr0qahpn29rsp7tu2m2ea";
const B_CHAIN_ADDR: &str = "juno18k2uq7srsr8lwrae6zr0qahpn29rsp7tw83nyx";
const C_CHAIN_ADDR: &str = "stars14cl2dthqamgucg9sfvv4relp3aa83e40pxn500";

struct TestEnv<Env: CwEnv> {
    // FEEDBACK: the trait `Clone` is not implemented for `Publisher<Env>`
    publisher: Publisher<Env>,
    abs: AbstractClient<Env>,
    gov: Application<Env, InterchainGovInterface<Env>>,
}

impl<Env: CwEnv> TestEnv<Env> {
    /// Set up the test environment with an Account that has the Adapter installed
    fn setup(env: Env) -> anyhow::Result<TestEnv<Env>> {
        // Create a sender and mock env
        let sender = env.sender();
        let namespace = Namespace::new(MY_NAMESPACE)?;

        // You can set up Abstract with a builder.
        let abs_client = AbstractClient::builder(env).build()?;

        // Publish the adapter
        let publisher = abs_client
            .publisher_builder(namespace)
            .install_on_sub_account(false)
            .build()?;
        publisher.publish_adapter::<InterchainGovInstantiateMsg, InterchainGovInterface<_>>(
            InterchainGovInstantiateMsg {
                accept_proposal_from_gov: Members {
                    members: vec![
                        ChainName::from_chain_id(A_CHAIN_ID),
                        ChainName::from_chain_id(B_CHAIN_ID),
                    ],
                },
            },
        )?;
        // Enable IBC on the account
        publisher
            .account()
            .as_ref()
            .manager
            .update_settings(Some(true))?;

        let adapter = publisher
            .account()
            .install_adapter::<InterchainGovInterface<_>>(&[])?;

        // Make it so we can execute on the adapter directly
        publisher.account().as_ref().manager.execute_on_module(
            MY_ADAPTER_ID,
            adapter::ExecuteMsg::<Empty>::Base(adapter::BaseExecuteMsg {
                proxy_address: None,
                msg: adapter::AdapterBaseMsg::UpdateAuthorizedAddresses {
                    to_add: vec![sender.to_string()],
                    to_remove: vec![],
                },
            }),
        )?;

        Ok(TestEnv {
            abs: abs_client,
            publisher,
            gov: adapter,
        })
    }

    pub fn chain_id(&self) -> String {
        self.environment().env_info().chain_id
    }

    pub fn chain_name(&self) -> ChainName {
        let chain_id = self.chain_id();
        ChainName::from_chain_id(&chain_id)
    }

    fn enable_ibc(&self) -> anyhow::Result<()> {
        Polytone::deploy_on(self.abs.environment().clone(), None)?;
        Ok(())
    }

    // helpers
    fn wait_blocks(&self, amount: u64) -> anyhow::Result<()> {
        Ok(self.environment().wait_blocks(amount)?)
    }
}

impl<Chain: CwEnv> Environment<Chain> for TestEnv<Chain> {
    fn environment(&self) -> Chain {
        self.abs.environment()
    }
}

const TEST_PROP_LEN: u64 = 1000;

impl<Env: CwEnv> TestEnv<Env> {
    pub fn execute_gov_for(
        &self,
        request: InterchainGovExecuteMsg,
        for_acc: Option<&Account<Env>>,
    ) -> anyhow::Result<Env::Response> {
        // let acc = for_acc.unwrap_or(self.gov.account());
        // Ok(self.gov.call_as(&acc.manager()?).execute(&AdapterRequestMsg {
        // // Ok(self.gov.call_as(&acc.manager()?).execute(&AdapterRequestMsg {
        //     proxy_address: Some(acc.proxy()?.to_string()),
        //     request,
        // }.into(), None)?)
        let acc = for_acc.unwrap_or(self.gov.account());
        Ok(acc.as_ref().manager.execute_on_module(
            MY_ADAPTER_ID,
            adapter::ExecuteMsg::<InterchainGovExecuteMsg, Empty>::Module(
                adapter::AdapterRequestMsg {
                    proxy_address: None,
                    request,
                },
            ),
        )?)
        // Ok(self.gov.execute(&AdapterRequestMsg {
        //     // Ok(self.gov.call_as(&acc.manager()?).execute(&AdapterRequestMsg {
        //     proxy_address: Some(acc.proxy()?.to_string()),
        //     request,
        // }.into(), None)?)
    }

    pub fn execute_gov(&self, request: InterchainGovExecuteMsg) -> anyhow::Result<Env::Response> {
        self.execute_gov_for(request, None)
    }

    fn propose_proposal(
        &self,
        title: &str,
        action: ProposalAction,
    ) -> anyhow::Result<(Env::Response, ProposalId)> {
        let test_prop = test_proposal(
            title,
            action,
            self.environment().block_info()?.height + TEST_PROP_LEN,
        );
        let res = self.execute_gov(InterchainGovExecuteMsg::Propose {
            proposal: test_prop.clone(),
        })?;

        let id: String = test_prop.hash();

        let props = self.gov.list_proposal_states()?.state;
        let prop_id = props
            .into_iter()
            .find(|p| p.proposal_id == id)
            .unwrap()
            .proposal_id;
        Ok((res, prop_id))
    }

    // Propose a member change
    fn propose_first_member_proposal(
        &self,
        title: &str,
        action: ProposalAction,
    ) -> anyhow::Result<(Env::Response, ProposalId)> {
        let test_prop = test_proposal(
            title,
            action,
            self.environment().block_info()?.height + TEST_PROP_LEN,
        );
        let res = self.execute_gov(InterchainGovExecuteMsg::Propose {
            proposal: test_prop.clone(),
        })?;

        let id: String = test_prop.hash();

        let props = self.gov.list_proposals()?.proposals;
        let prop_id = props.into_iter().find(|p| p.prop_id == id).unwrap().prop_id;
        Ok((res, prop_id))
    }

    fn finalize_proposal(&self, prop_id: ProposalId) -> anyhow::Result<Env::Response> {
        let res = self.execute_gov(InterchainGovExecuteMsg::Finalize { prop_id })?;

        Ok(res)
    }

    fn assert_prop_state(
        &self,
        prop_id: ProposalId,
        expected_state: Option<DataState>,
    ) -> anyhow::Result<()> {
        let state = self.gov.proposal_state(prop_id)?.map(|f| f.state);
        assert_that!(state).is_equal_to(expected_state);
        Ok(())
    }

    fn test_register_gov_modules(
        &self,
        govs: Vec<(ChainName, InterchainGovInterface<MockBech32>)>,
    ) -> anyhow::Result<()> {
        let modules = govs
            .into_iter()
            .map(|(chain_name, gov)| (chain_name, gov.address().unwrap().to_string()))
            .collect();
        self.execute_gov(
            InterchainGovExecuteMsg::TemporaryRegisterRemoteGovModuleAddrs { modules },
        )?;
        Ok(())
    }
}

fn test_proposal(
    title: impl Into<String>,
    action: ProposalAction,
    exp_height: u64,
) -> ProposalMsg {
    ProposalMsg {
        title: title.into(),
        description: "This is a test proposal".to_string(),
        min_voting_period: None,
        expiration: Expiration::AtHeight(exp_height),
        action,
    }
}

mod propose {

    use super::*;

    #[test]
    fn happy_propose() -> anyhow::Result<()> {
        let interchain = MockBech32InterchainEnv::new(vec![
            (A_CHAIN_ID, A_CHAIN_ADDR),
            (B_CHAIN_ID, B_CHAIN_ADDR),
        ]);

        let a_env = TestEnv::setup(interchain.chain(A_CHAIN_ID)?)?;
        let b_env = TestEnv::setup(interchain.chain(B_CHAIN_ID)?)?;

        a_env.enable_ibc()?;
        b_env.enable_ibc()?;
        ibc_connect_polytone_and_abstract(&interchain, A_CHAIN_ID, B_CHAIN_ID)?;

        let a_gov = a_env.gov.clone();
        let b_gov = b_env.gov.clone();

        a_env.execute_gov(InterchainGovExecuteMsg::TestAddMembers {
            members: vec![b_env.chain_name(), a_env.chain_name()].into(),
        })?;

        b_env.execute_gov(InterchainGovExecuteMsg::TestAddMembers {
            members: vec![a_env.chain_name(), b_env.chain_name()].into(),
        })?;

        // Create proposal
        let (res, prop_id) = a_env.propose_proposal("happy_propose", ProposalAction::Signal)?;

        interchain.wait_ibc(A_CHAIN_ID, res)?;

        let proposals = a_gov.list_proposal_states()?;
        assert_that!(proposals.state.len()).is_equal_to(0);

        let proposals = b_gov.list_proposal_states()?;
        assert_that!(proposals.state.len()).is_equal_to(1);

        // check the state.
        // should be proposed on A and pending on B
        a_env.assert_prop_state(prop_id.clone(), None)?;
        b_env.assert_prop_state(prop_id, Some(DataState::Proposed))?;

        Ok(())
    }
}

mod query_vote_results {
    use super::*;
    use interchain_gov::state::{Governance, Vote};

    #[test]
    fn happy_query() -> anyhow::Result<()> {
        let interchain = MockBech32InterchainEnv::new(vec![
            (A_CHAIN_ID, A_CHAIN_ADDR),
            (B_CHAIN_ID, B_CHAIN_ADDR),
        ]);

        let a_env = TestEnv::setup(interchain.chain(A_CHAIN_ID)?)?;
        let b_env = TestEnv::setup(interchain.chain(B_CHAIN_ID)?)?;

        a_env.enable_ibc()?;
        b_env.enable_ibc()?;
        ibc_connect_polytone_and_abstract(&interchain, A_CHAIN_ID, B_CHAIN_ID)?;
        a_env.test_register_gov_modules(vec![(b_env.chain_name(), b_env.gov.clone())])?;

        let a_gov = a_env.gov.clone();
        let b_gov = b_env.gov.clone();

        a_env.execute_gov(InterchainGovExecuteMsg::TestAddMembers {
            members: vec![b_env.chain_name()].into(),
        })?;

        b_env.execute_gov(InterchainGovExecuteMsg::TestAddMembers {
            members: vec![a_env.chain_name()].into(),
        })?;

        // Create proposal
        let (res, prop_id) = a_env.propose_proposal("happy_propose", ProposalAction::Signal)?;
        interchain.wait_ibc(A_CHAIN_ID, res)?;
        // Finalize a proposal
        let res = a_env.finalize_proposal(prop_id.clone())?;
        interchain.wait_ibc(A_CHAIN_ID, res)?;

        // Vote on the proposal
        a_env.execute_gov(InterchainGovExecuteMsg::VoteProposal {
            prop_id: prop_id.clone(),
            governance: Governance::Manual {},
            vote: Vote::Yes,
        })?;
        let a_vote = a_gov.vote(prop_id.clone())?;
        assert_that!(a_vote.vote).is_equal_to(Vote::Yes);

        b_env.execute_gov(InterchainGovExecuteMsg::VoteProposal {
            prop_id: prop_id.clone(),
            governance: Governance::Manual {},
            vote: Vote::Yes,
        })?;
        let vote = b_gov.vote(prop_id.clone())?;
        assert_that!(vote.vote).is_equal_to(Vote::Yes);

        // Wait the test blocks after voting
        a_env.wait_blocks(TEST_PROP_LEN + 1)?;
        b_env.wait_blocks(TEST_PROP_LEN + 1)?;

        // Request vote results
        let res = a_env.execute_gov(InterchainGovExecuteMsg::RequestVoteResults {
            prop_id: prop_id.clone(),
        })?;

        let analysis = interchain.wait_ibc(A_CHAIN_ID, res)?;

        let vote_results = a_gov.vote_results(prop_id.clone())?;
        println!("Vote results: {:?}", vote_results);

        Ok(())
    }
}

mod finalize {

    use super::*;

    #[test]
    fn happy_finalize() -> anyhow::Result<()> {
        let interchain = MockBech32InterchainEnv::new(vec![
            (A_CHAIN_ID, A_CHAIN_ADDR),
            (B_CHAIN_ID, B_CHAIN_ADDR),
        ]);

        let a_env = TestEnv::setup(interchain.chain(A_CHAIN_ID)?)?;
        let b_env = TestEnv::setup(interchain.chain(B_CHAIN_ID)?)?;

        a_env.enable_ibc()?;
        b_env.enable_ibc()?;
        ibc_connect_polytone_and_abstract(&interchain, A_CHAIN_ID, B_CHAIN_ID)?;

        let _a_gov = a_env.gov.clone();
        let _b_gov = b_env.gov.clone();

        a_env.execute_gov(InterchainGovExecuteMsg::TestAddMembers {
            members: vec![b_env.chain_name(), a_env.chain_name()].into(),
        })?;

        b_env.execute_gov(InterchainGovExecuteMsg::TestAddMembers {
            members: vec![a_env.chain_name(), b_env.chain_name()].into(),
        })?;

        // Propose a proposal
        let (res, prop_id) = a_env.propose_proposal("happy_finalize", ProposalAction::Signal)?;
        interchain.wait_ibc(A_CHAIN_ID, res)?;

        a_env.assert_prop_state(prop_id.clone(), None)?;
        b_env.assert_prop_state(prop_id.clone(), Some(DataState::Proposed))?;

        // Finalize a proposal
        let res = a_env.finalize_proposal(prop_id.clone())?;
        let _analysis = interchain.wait_ibc(A_CHAIN_ID, res)?;
        // TODO: this errs
        a_env.assert_prop_state(prop_id.clone(), None)?;
        b_env.assert_prop_state(prop_id, None)?;

        Ok(())
    }
}

#[test]
fn starship_test() -> anyhow::Result<()> {
    let rt = Runtime::new()?;
    let starship = Starship::new(rt.handle(), Some("http://65.108.1.234:8081"))?;

    let interchain = starship.interchain_env();

    let a_env = TestEnv::setup(interchain.chain(A_CHAIN_ID)?)?;
    let b_env = TestEnv::setup(interchain.chain(B_CHAIN_ID)?)?;

    a_env.enable_ibc()?;
    b_env.enable_ibc()?;
    ibc_connect_polytone_and_abstract(&interchain, A_CHAIN_ID, B_CHAIN_ID)?;

    let a_gov = a_env.gov.clone();
    let b_gov = b_env.gov.clone();

    a_env.execute_gov(InterchainGovExecuteMsg::TestAddMembers {
        members: vec![b_env.chain_name()].into(),
    })?;

    b_env.execute_gov(InterchainGovExecuteMsg::TestAddMembers {
        members: vec![a_env.chain_name()].into(),
    })?;

    // Propose a proposal
    let (res, prop_id) = a_env.propose_proposal("happy_finalize", ProposalAction::Signal)?;
    interchain.wait_ibc(A_CHAIN_ID, res)?;

    a_env.assert_prop_state(prop_id.clone(), Some(DataState::Proposed))?;
    b_env.assert_prop_state(prop_id.clone(), Some(DataState::Initiated))?;

    // Finalize a proposal
    let res = a_env.finalize_proposal(prop_id.clone())?;
    let analysis = interchain.wait_ibc(A_CHAIN_ID, res)?;

    a_env.assert_prop_state(prop_id.clone(), None)?;
    b_env.assert_prop_state(prop_id, None)?;

    Ok(())
}

mod members {

    use std::env;

    use abstract_interchain_tests::setup::ibc_connect_abstract;
    use cw_orch_interchain::MockBech32InterchainEnv;
    use interchain_gov::msg::InterchainGovExecuteMsgFns;

    use super::*;

    #[test]
    fn happy_member_add() -> anyhow::Result<()> {
        env::set_var("RUST_LOG", "debug");
        env_logger::init();

        let interchain = MockBech32InterchainEnv::new(vec![
            (A_CHAIN_ID, A_CHAIN_ADDR),
            (B_CHAIN_ID, B_CHAIN_ADDR),
        ]);

        let a_env = TestEnv::setup(interchain.chain(A_CHAIN_ID)?)?;
        let b_env = TestEnv::setup(interchain.chain(B_CHAIN_ID)?)?;

        a_env.enable_ibc()?;
        b_env.enable_ibc()?;
        ibc_connect_polytone_and_abstract(&interchain, A_CHAIN_ID, B_CHAIN_ID)?;

        let a_gov = a_env.gov.clone();
        let b_gov = b_env.gov.clone();

        // Propose a proposal
        let (res, prop_id) = a_env.propose_first_member_proposal(
            "happy_finalize",
            ProposalAction::UpdateMembers {
                members: vec![b_env.chain_name(), a_env.chain_name()].into(),
            },
        )?;
        let res = interchain.wait_ibc(A_CHAIN_ID, res)?;
        dbg!(&res.packets[0].outcome);
        a_env.assert_prop_state(prop_id.clone(), None)?;
        b_env.assert_prop_state(prop_id.clone(), None)?;

        let a_members = dbg!(a_gov.members()?);
        assert_eq!(a_members.members.members.len(), 2);
        let b_members = dbg!(b_gov.members()?);
        assert_eq!(b_members.members.members.len(), 2);
        Ok(())
    }

    #[test]
    fn happy_member_add_3() -> anyhow::Result<()> {
        env::set_var("RUST_LOG", "info");
        env_logger::init();

        let interchain = MockBech32InterchainEnv::new(vec![
            (A_CHAIN_ID, A_CHAIN_ADDR),
            (B_CHAIN_ID, B_CHAIN_ADDR),
            (C_CHAIN_ID, C_CHAIN_ADDR),
        ]);
        use cw_orch_interchain::InterchainEnv;
        let a_env = TestEnv::setup(interchain.chain(A_CHAIN_ID)?)?;
        let b_env = TestEnv::setup(interchain.chain(B_CHAIN_ID)?)?;
        let c_env = TestEnv::setup(interchain.chain(C_CHAIN_ID)?)?;

        a_env.enable_ibc()?;
        b_env.enable_ibc()?;
        c_env.enable_ibc()?;

        // ibc_connect_polytone_and_abstract(&interchain, A_CHAIN_ID, B_CHAIN_ID)?;
        // ibc_connect_polytone_and_abstract(&interchain, A_CHAIN_ID, C_CHAIN_ID)?;
        // ibc_connect_polytone_and_abstract(&interchain, B_CHAIN_ID, A_CHAIN_ID)?;
        // ibc_connect_polytone_and_abstract(&interchain, B_CHAIN_ID, C_CHAIN_ID)?;
        // ibc_connect_polytone_and_abstract(&interchain, C_CHAIN_ID, A_CHAIN_ID)?;
        // ibc_connect_polytone_and_abstract(&interchain, C_CHAIN_ID, B_CHAIN_ID)?;
        
        ibc_connect_abstract(&interchain, A_CHAIN_ID, B_CHAIN_ID)?;
        ibc_connect_abstract(&interchain, A_CHAIN_ID, C_CHAIN_ID)?;
        ibc_connect_abstract(&interchain, B_CHAIN_ID, A_CHAIN_ID)?;
        ibc_connect_abstract(&interchain, B_CHAIN_ID, C_CHAIN_ID)?;
        ibc_connect_abstract(&interchain, C_CHAIN_ID, A_CHAIN_ID)?;
        ibc_connect_abstract(&interchain, C_CHAIN_ID, B_CHAIN_ID)?;


        let a_gov = a_env.gov.clone();
        let b_gov = b_env.gov.clone();

        // Propose a proposal
        let (res, prop_id) = a_env.propose_first_member_proposal(
            "happy_finalize",
            ProposalAction::UpdateMembers {
                members: vec![b_env.chain_name(), a_env.chain_name()].into(),
            },
        )?;
        let res = interchain.wait_ibc(A_CHAIN_ID, res)?;
        dbg!(&res.packets[0].outcome);

        let a_members = dbg!(a_gov.members()?);
        assert_eq!(a_members.members.members.len(), 2);
        let b_members = dbg!(b_gov.members()?);
        assert_eq!(b_members.members.members.len(), 2);

        a_env.assert_prop_state(prop_id.clone(), None)?;
        b_env.assert_prop_state(prop_id.clone(), None)?;

        // A and B are friends now. 
        // Propose to add C

        let (res, prop_id) = a_env.propose_proposal(
            "happy_finalize 2",
            ProposalAction::UpdateMembers {
                members: vec![b_env.chain_name(), a_env.chain_name(), c_env.chain_name()].into(),
            },
        )?;

        let res = interchain.wait_ibc(A_CHAIN_ID, res)?;
        dbg!(&res.packets[0].outcome);

        a_env.assert_prop_state(prop_id.clone(), None)?;
        b_env.assert_prop_state(prop_id.clone(), Some(DataState::Proposed))?;
        c_env.assert_prop_state(prop_id.clone(), Some(DataState::Proposed))?;
    
        a_gov.finalize(prop_id.clone())?;

        a_env.assert_prop_state(prop_id.clone(), None)?;
        b_env.assert_prop_state(prop_id.clone(), None)?;
        c_env.assert_prop_state(prop_id.clone(), None)?;
    
        Ok(())
    }
}

// #[test]
// fn update_members() -> anyhow::Result<()> {
//     let env = TestEnv::setup()?;
//     let adapter = env.gov;
//
//     // Executing it on publisher account
//     // Note that it's not a requirement to have it installed in this case
//     let publisher_account = env
//         .abs
//         .publisher_builder(Namespace::new(MY_NAMESPACE).unwrap())
//         .build()?;
//
//     env.execute_gov(InterchainGovExecuteMsg::UpdateConfig {}, None)?;
//
//     let config = adapter.config()?;
//     let expected_response = interchain_gov::msg::ConfigResponse {};
//     assert_eq!(config, expected_response);
//
//     // Adapter installed on sub-account of the publisher so this should error
//     let err = adapter
//         .execute(
//             &AdapterRequestMsg {
//                 proxy_address: Some(adapter.account().proxy()?.to_string()),
//                 request: InterchainGovExecuteMsg::UpdateConfig {},
//             }
//             .into(),
//             None,
//         )
//         .unwrap_err();
//     assert_eq!(err.root().to_string(), "Unauthorized");
//
//     Ok(())
// }
