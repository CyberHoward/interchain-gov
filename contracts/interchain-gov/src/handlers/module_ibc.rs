use abstract_adapter::sdk::AbstractResponse;
use abstract_adapter::std::ibc::ModuleIbcMsg;
use cosmwasm_std::{from_json, DepsMut, Env};

use crate::contract::{AdapterResult, InterchainGov};
use crate::msg::InterchainGovIbcMsg;
use crate::state::members_sync_state::MembersSyncState;
use crate::state::ALLOW_JOINING_GOV;
use crate::{InterchainGovError, MY_ADAPTER_ID};

pub fn module_ibc_handler(
    deps: DepsMut,
    _env: Env,
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

    match ibc_msg {
        InterchainGovIbcMsg::JoinGovProposal { members } => {
            // Check that the data has been finalized before.
            MembersSyncState::new().assert_finalized(deps.storage)?;

            let allowed_gov = ALLOW_JOINING_GOV.load(deps.storage)?;

            // assert that the governance members are the whitelisted members
            if members != allowed_gov {
                return Err(InterchainGovError::UnauthorizedIbcMessage {});
            }

            MembersSyncState::new().finalize_members(deps.storage, Some(members))?;

            Ok(app.response("module_ibc").add_message(msg))
        }
        _ => Err(InterchainGovError::UnauthorizedIbcMessage {}),
    }
}
