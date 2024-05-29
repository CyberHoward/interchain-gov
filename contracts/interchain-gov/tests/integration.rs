use abstract_adapter::objects::chain_name::ChainName;
use interchain_gov::{contract::interface::InterchainGovInterface, msg::{ConfigResponse, InterchainGovInstantiateMsg, InterchainGovQueryMsgFns}, InterchainGovExecuteMsg, MY_NAMESPACE, MY_ADAPTER_ID};

use abstract_adapter::std::{adapter, adapter::AdapterRequestMsg, objects::namespace::Namespace};
use abstract_adapter::std::manager::ExecuteMsgFns;
use abstract_client::{AbstractClient, Account, Application, Environment, Publisher};
use abstract_cw_orch_polytone::Polytone;
// Use prelude to get all the necessary imports
use cw_orch::{anyhow, prelude::*};
// use cw_orch_interchain::{prelude::*};
use speculoos::prelude::*;

use abstract_interchain_tests::setup::ibc_connect_polytone_and_abstract;
use cw_orch::mock::cw_multi_test::{AppResponse};
use interchain_gov::state::{DataState, ProposalAction, ProposalId, ProposalMsg};
// use cw_orch_interchain::MockBech32InterchainEnv;

const A_CHAIN_ID: &str = "neutron-1";
const B_CHAIN_ID: &str = "harpoon-1";
const A_CHAIN_ADDR: &str = "neutron18k2uq7srsr8lwrae6zr0qahpn29rsp7tu2m2ea";
const B_CHAIN_ADDR: &str = "kujira18k2uq7srsr8lwrae6zr0qahpn29rsp7tfassws";

struct TestEnv<Env: CwEnv> {
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
        let publisher = abs_client.publisher_builder(namespace).install_on_sub_account(false).build()?;
        publisher.publish_adapter::<InterchainGovInstantiateMsg, InterchainGovInterface<_>>(
            InterchainGovInstantiateMsg {},
        )?;
        // Enable IBC on the account
        publisher.account().as_ref().manager.update_settings(Some(true))?;

        let adapter = publisher
            .account()
            .install_adapter::<InterchainGovInterface<_>>(&[])?;

        // Make it so we can execute on the adapter directly
        publisher.account().as_ref().manager.execute_on_module(MY_ADAPTER_ID, adapter::ExecuteMsg::<Empty>::Base(
            adapter::BaseExecuteMsg {
                proxy_address: None,
                msg: adapter::AdapterBaseMsg::UpdateAuthorizedAddresses {
                    to_add: vec![sender.to_string()],
                    to_remove: vec![],
                },
            },
        ))?;

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

}

impl<Chain: CwEnv> Environment<Chain> for TestEnv<Chain> {
    fn environment(&self) -> Chain {
        self.abs.environment()
    }
}

impl TestEnv<MockBech32> {
    pub fn execute_gov_for(&self, request: InterchainGovExecuteMsg, for_acc: Option<&Account<MockBech32>>) -> anyhow::Result<AppResponse> {
        let acc = for_acc.unwrap_or(self.gov.account());
        Ok(self.gov.call_as(&acc.manager()?).execute(&AdapterRequestMsg {
            proxy_address: Some(acc.proxy()?.to_string()),
            request,
        }.into(), None)?)
    }

    pub fn execute_gov(&self, request: InterchainGovExecuteMsg) -> anyhow::Result<AppResponse> {
       self.execute_gov_for(request, None)
    }

    fn propose_proposal(&self, title: &str, actions: Vec<ProposalAction>) -> anyhow::Result<(AppResponse, ProposalId)> {
        let res = self.execute_gov(InterchainGovExecuteMsg::Propose {
            proposal: test_proposal(title, actions)
        })?;

        let props = self.gov.list_proposals()?.proposals;
        let prop_id = props.into_iter().find(|p| p.prop.title == title).unwrap().prop_id;

        Ok((res, prop_id))
    }

    fn finalize_proposal(&self, prop_id: ProposalId) -> anyhow::Result<AppResponse> {
        let res = self.execute_gov(InterchainGovExecuteMsg::Finalize {
            prop_id
        })?;

        Ok(res)
    }

    fn assert_prop_state(&self, prop_id: ProposalId, expected_state: DataState) -> anyhow::Result<()> {
        let state = self.gov.proposal(prop_id)?.state;
        assert_that!(state).is_equal_to(expected_state);
        Ok(())
    }
}

fn test_proposal(title: impl Into<String>, actions: Vec<ProposalAction>) -> ProposalMsg {
    ProposalMsg {
        title: title.into(),
        description: "This is a test proposal".to_string(),
        min_voting_period: None,
        expiration: Default::default(),
        actions
    }
}

#[test]
fn successful_install() -> anyhow::Result<()> {
    let mock = MockBech32::new("mock");
    let env = TestEnv::setup(mock)?;
    let adapter = env.gov.clone();

    let config = adapter.config()?;
    assert_eq!(config, ConfigResponse {});

    // Check single member
    let members = adapter.members()?;
    assert_eq!(members.members.len(), 1);
    // check that the single member is the current chain
    let current_chain_id = env.chain_name();
    assert_that!(members.members[0]).is_equal_to(current_chain_id);

    Ok(())
}


mod propose {
    
    use super::*;

    #[test]
    fn happy_propose() -> anyhow::Result<()> {
        let interchain = MockBech32InterchainEnv::new(vec![
            (A_CHAIN_ID, A_CHAIN_ADDR),
            (
                B_CHAIN_ID,
                B_CHAIN_ADDR
            ),
        ]);

        let a_env = TestEnv::setup(interchain.chain(A_CHAIN_ID)?)?;
        let b_env = TestEnv::setup(interchain.chain(B_CHAIN_ID)?)?;

        a_env.enable_ibc()?;
        b_env.enable_ibc()?;
        ibc_connect_polytone_and_abstract(&interchain, A_CHAIN_ID, B_CHAIN_ID)?;

        let a_gov = a_env.gov.clone();
        let b_gov = b_env.gov.clone();

        a_env.execute_gov(InterchainGovExecuteMsg::TestAddMembers {
            members: vec![b_env.chain_name()],
        })?;

        b_env.execute_gov(InterchainGovExecuteMsg::TestAddMembers {
            members: vec![a_env.chain_name()],
        })?;

        // Create proposal
        let (res, prop_id) = a_env.propose_proposal("happy_propose", vec![])?;

        interchain.wait_ibc(A_CHAIN_ID, res)?;

        let proposals = a_gov.list_proposals()?;
        assert_that!(proposals.proposals).has_length(1);

        // check the state.
        // should be proposed on A and pending on B
        a_env.assert_prop_state(prop_id.clone(), DataState::Proposed)?;
        b_env.assert_prop_state(prop_id, DataState::Initiated)?;

        Ok(())
    }
}


mod finalize {
    
    use super::*;

    #[test]
    fn happy_finalize() -> anyhow::Result<()> {
        let interchain = MockBech32InterchainEnv::new(vec![
            (A_CHAIN_ID, A_CHAIN_ADDR),
            (
                B_CHAIN_ID,
                B_CHAIN_ADDR
            ),
        ]);

        let a_env = TestEnv::setup(interchain.chain(A_CHAIN_ID)?)?;
        let b_env = TestEnv::setup(interchain.chain(B_CHAIN_ID)?)?;

        a_env.enable_ibc()?;
        b_env.enable_ibc()?;
        ibc_connect_polytone_and_abstract(&interchain, A_CHAIN_ID, B_CHAIN_ID)?;

        let a_gov = a_env.gov.clone();
        let b_gov = b_env.gov.clone();

        a_env.execute_gov(InterchainGovExecuteMsg::TestAddMembers {
            members: vec![b_env.chain_name()],
        })?;

        b_env.execute_gov(InterchainGovExecuteMsg::TestAddMembers {
            members: vec![a_env.chain_name()],
        })?;

        // Propose a proposal
        let (res, prop_id) = a_env.propose_proposal("happy_finalize", vec![])?;
        interchain.wait_ibc(A_CHAIN_ID, res)?;

        a_env.assert_prop_state(prop_id.clone(), DataState::Proposed)?;
        b_env.assert_prop_state(prop_id.clone(), DataState::Initiated)?;

        // Finalize a proposal
        let res = a_env.finalize_proposal(prop_id.clone())?;
        let analysis = interchain.wait_ibc(A_CHAIN_ID, res)?;

        a_env.assert_prop_state(prop_id.clone(), DataState::Finalized)?;
        b_env.assert_prop_state(prop_id, DataState::Finalized)?;


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