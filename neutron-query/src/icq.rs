//! # Neutron Interchain Query (ICQ)
use crate::{QueryType, ICQ_PROTOCOL};
use abstract_adapter::objects::chain_name::ChainName;
use abstract_adapter::objects::UncheckedChannelEntry;
use abstract_adapter::std::AbstractError;
use abstract_sdk::features::AccountIdentification;
use abstract_sdk::{AbstractSdkError, AbstractSdkResult, TransferInterface};
use cosmos_anybuf::interfaces::TransactionFilterItem;
use cosmos_anybuf::types::neutron::interchainqueries::{KVKey, QueryResult, RegisteredQuery};
use cosmos_anybuf::{interfaces::InterChainQueries, neutron::Neutron};
use cosmwasm_schema::schemars::_serde_json::to_string;
use cosmwasm_std::{Addr, CosmosMsg, Deps, StdError};

/// An interface to the Neutron Interchain Query Module
pub trait IcqInterface: AccountIdentification + TransferInterface {
    /// API for accessing the Cosmos SDK FeeGrant module.
    /// The **granter** is the address of the user granting an allowance of their funds.
    /// By default, it is the proxy address of the Account.

    /// ```
    /// use abstract_sdk::prelude::*;
    /// # use cosmwasm_std::testing::mock_dependencies;
    /// # use abstract_sdk::mock_module::MockModule;
    /// # let module = MockModule::new();
    /// # let deps = mock_dependencies();

    /// let grant: FeeGranter = module.fee_granter(deps.as_ref(), None)?;
    /// ```
    fn neutron_icq<'a>(
        &'a self,
        deps: cosmwasm_std::Deps<'a>,
    ) -> AbstractSdkResult<NeutronIcq<Self>> {
        Ok(NeutronIcq { base: self, deps })
    }
}

impl<T> IcqInterface for T where T: AccountIdentification + TransferInterface {}

// impl<'a, T: IcqInterface> AbstractApi<T> for NeutronIcq<'a, T> {
//     fn base(&self) -> &T {
//         self.base
//     }
//     fn deps(&self) -> Deps {
//         self.deps
//     }
// }
//
// impl<'a, T: IcqInterface> ApiIdentification for NeutronIcq<'a, T> {
//     fn api_id() -> String {
//         "Modules".to_owned()
//     }
// }

/// This struct provides methods to grant fee allowances and interact with the feegrant module.
///
/// # Example
/// ```
/// use abstract_sdk::prelude::*;
/// # use cosmwasm_std::testing::mock_dependencies;
/// # use abstract_sdk::mock_module::MockModule;
/// # let module = MockModule::new();
///
/// let grant: NeutronIcq  = module.neutron_icq(deps.as_ref())?;
/// ```
/// */
#[derive(Clone)]
pub struct NeutronIcq<'a, T: AccountIdentification + TransferInterface> {
    base: &'a T,
    deps: Deps<'a>,
}

impl<'a, T: AccountIdentification + TransferInterface> NeutronIcq<'a, T> {
    pub fn connection_id(&self, chain: ChainName) -> AbstractSdkResult<String> {
        self.base
            .ans_host(self.deps)?
            .query_channel(
                &self.deps.querier,
                &UncheckedChannelEntry::new(chain.as_str(), ICQ_PROTOCOL).check()?,
            )
            .map_err(|e| AbstractSdkError::Abstract(AbstractError::AnsHostError(e)))
    }

    pub fn register_interchain_query(
        &self,
        sender: &Addr,
        chain: ChainName,
        query_type: QueryType,
        keys: Vec<KVKey>,
        transactions_filter: Vec<TransactionFilterItem>,
        update_period: u64,
    ) -> AbstractSdkResult<CosmosMsg> {
        let connection_id = self.connection_id(chain)?;

        let register_msg = Neutron::register_interchain_query(
            sender,
            query_type.to_string(),
            keys,
            to_string(&transactions_filter).map_err(|e| StdError::generic_err(e.to_string()))?,
            connection_id,
            update_period,
        );

        Ok(register_msg)
    }

    /// Remove an existing interchain query with the given id.
    ///
    /// # Arguments
    ///
    /// * `query_id` - The id of the query
    pub fn remove_interchain_query(
        &self,
        sender: &Addr,
        query_id: u64,
    ) -> AbstractSdkResult<CosmosMsg> {
        Ok(Neutron::remove_interchain_query(sender, query_id))
    }

    pub fn query_registered_query(&self, query_id: u64) -> AbstractSdkResult<RegisteredQuery> {
        Ok(Neutron::query_registered_query(&self.deps.querier, query_id)?.registered_query)
    }

    pub fn query_registered_query_result(&self, query_id: u64) -> AbstractSdkResult<QueryResult> {
        Ok(Neutron::query_registered_query_result(&self.deps.querier, query_id)?.result)
    }
}
