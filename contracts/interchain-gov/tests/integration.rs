use std::str::FromStr;
use abstract_adapter::objects::chain_name::ChainName;
use interchain_gov::{
    contract::interface::InterchainGovInterface,
    msg::{ConfigResponse, ExecuteMsg, InterchainGovInstantiateMsg, InterchainGovQueryMsgFns},
    InterchainGovExecuteMsg, MY_ADAPTER_ID, MY_NAMESPACE,
};

use abstract_adapter::std::{adapter::AdapterRequestMsg, objects::namespace::Namespace};
use abstract_adapter::std::manager::ExecuteMsgFns;
use abstract_client::{AbstractClient, Account, Application, Environment, Publisher};
use cosmwasm_std::coins;
// Use prelude to get all the necessary imports
use cw_orch::{anyhow, prelude::*};
use cw_orch::mock::cw_multi_test::AppResponse;
use speculoos::prelude::*;

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

        Ok(TestEnv {
            abs: abs_client,
            publisher,
            gov: adapter,
        })
    }

    pub fn execute_gov(&self, request: InterchainGovExecuteMsg, as_acc: Option<&Account<Env>>) -> anyhow::Result<Env::Response> {
        Ok(self.gov.execute(&AdapterRequestMsg {
            proxy_address: Some(as_acc.unwrap_or(self.gov.account()).proxy()?.to_string()),
            request,
        }.into(), None)?)

    }

    pub fn chain_id(&self) -> String {
        self.abs.environment().env_info().chain_id
    }

    pub fn chain_name(&self) -> ChainName {
        let chain_id = self.chain_id();
        ChainName::from_chain_id(&chain_id)
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