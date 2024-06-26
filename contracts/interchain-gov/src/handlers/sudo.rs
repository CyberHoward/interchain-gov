use abstract_adapter::sdk::AbstractResponse;

use cosmwasm_std::{Binary, DepsMut, Env, StdError, Storage};

use neutron_query::icq::IcqInterface;

use crate::msg::InterchainGovSudoMsg;
use crate::state::{TallyResult, GOV_VOTE_QUERIES, PENDING_QUERIES};
use crate::{
    contract::{AdapterResult, InterchainGov},
    InterchainGovError,
};

pub fn sudo_handler(
    deps: DepsMut,
    _env: Env,
    app: InterchainGov,
    msg: InterchainGovSudoMsg,
) -> AdapterResult {
    match msg {
        InterchainGovSudoMsg::KVQueryResult { query_id } => {
            // Load the pending query details
            let query_details = PENDING_QUERIES.load(deps.storage, query_id)?;
            PENDING_QUERIES.remove(deps.storage, query_id);

            let query_res = app
                .neutron_icq(deps.as_ref())?
                .query_registered_query_result(query_id)?;

            // Map into the proper type
            let retyped_kv_results = query_res
                .kv_results
                .into_iter()
                .map(|kv| {
                    let key = kv.key;
                    let value = kv.value;
                    neutron_query::neutron_sdk::bindings::types::StorageValue {
                        storage_prefix: kv.storage_prefix,
                        key: Binary(key),
                        value: Binary(value),
                    }
                })
                .collect::<Vec<_>>();

            let gov_props: neutron_query::neutron_sdk::interchain_queries::v045::types::GovernmentProposal =
                neutron_query::neutron_sdk::interchain_queries::types::KVReconstruct::reconstruct(retyped_kv_results.as_slice()).map_err(|e| InterchainGovError::Std(StdError::generic_err(e.to_string())))?;

            // check the first proposal
            let prop = gov_props.proposals.first();
            if let Some(prop) = prop {
                if prop.final_tally_result.is_none() {
                    return Err(StdError::generic_err("No final tally result found").into());
                };

                let tally_result = prop.final_tally_result.clone().unwrap();

                let tally = TallyResult {
                    yes: tally_result.yes,
                    no: tally_result.no,
                    abstain: tally_result.abstain,
                    no_with_veto: tally_result.no_with_veto,
                };

                GOV_VOTE_QUERIES.save(
                    deps.storage,
                    (query_details.1.clone(), &query_details.0),
                    &Some(tally),
                )?;
            } else {
                unimplemented!("No proposals found")
            }
        }
    }

    Ok(app.response("instantiate"))
}
