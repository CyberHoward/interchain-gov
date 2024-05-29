use abstract_adapter::objects::chain_name::ChainName;
use abstract_adapter::sdk::AbstractResponse;
use abstract_adapter::std::ibc::ModuleIbcMsg;
use cosmwasm_std::{from_json, DepsMut, Env};

use crate::contract::{AdapterResult, InterchainGov};
use crate::msg::InterchainGovIbcMsg;

use crate::state::{Vote, ALLOW_JOINING_GOV, MEMBERS_STATE_SYNC, PROPOSAL_STATE_SYNC};
use crate::{InterchainGovError, MY_ADAPTER_ID};

pub fn module_ibc_handler(
    deps: DepsMut,
    env: Env,
    app: InterchainGov,
    ibc_msg: ModuleIbcMsg,
) -> AdapterResult {
    println!("module_ibc_handler 1 : {:?}", ibc_msg);

    // First check that we received the message from the gov contract
    if ibc_msg.source_module.id().ne(MY_ADAPTER_ID) {
        println!("UnauthorizedIbcModule: {:?}", ibc_msg.source_module.clone());
        return Err(InterchainGovError::UnauthorizedIbcModule(
            ibc_msg.source_module.clone(),
        ));
    };

    let ibc_msg: InterchainGovIbcMsg = from_json(&ibc_msg.msg)?;

    println!("parsed_msg: {:?}", ibc_msg);

    let our_chain = ChainName::new(&env);

    match ibc_msg {
        InterchainGovIbcMsg::JoinGov { members } => {
            // Check that the data has been finalized before.
            MEMBERS_STATE_SYNC.assert_finalized(deps.storage)?;

            let allowed_gov = ALLOW_JOINING_GOV.load(deps.storage)?;

            // assert that the governance members are the whitelisted members
            if members != allowed_gov {
                return Err(InterchainGovError::UnauthorizedIbcMessage {});
            }

            MEMBERS_STATE_SYNC.finalize_members(deps.storage, Some(members))?;

            Ok(app.response("module_ibc"))
        }
        InterchainGovIbcMsg::ProposeProposal {
            prop_hash,
            prop,
            chain,
        } => {
            // check that we were supposed to receive the message
            if chain != our_chain {
                return Err(InterchainGovError::WrongChain {
                    expected: our_chain,
                    actual: chain,
                });
            }

            // update proposal state to "proposed". Member will vote `NoVote` on the proposal by default
            PROPOSAL_STATE_SYNC.propose_kv_state(deps.storage, prop_hash, (prop, Vote::NoVote))?;

            Ok(app
                .response("module_ibc")
                .add_attribute("action", "propose"))
        }
        InterchainGovIbcMsg::FinalizeProposal { prop_hash: prop_id } => {
            PROPOSAL_STATE_SYNC.finalize_kv_state(deps.storage, prop_id, None)?;

            Ok(app
                .response("module_ibc")
                .add_attribute("action", "finalize"))
        }
        _ => Err(InterchainGovError::UnauthorizedIbcMessage {}),
    }
}
