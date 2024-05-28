use abstract_adapter::sdk::AbstractResponse;
use abstract_adapter::std::ibc::ModuleIbcMsg;
use cosmwasm_std::{from_json, DepsMut, Env};

use crate::{InterchainGovError, MY_ADAPTER_ID};
use crate::contract::{AdapterResult, InterchainGov};
use crate::msg::InterchainGovIbcMsg;
use crate::state::MEMBERS;

pub fn module_ibc_handler(
    deps: DepsMut,
    _env: Env,
    app: InterchainGov,
    ibc_msg: ModuleIbcMsg,
) -> AdapterResult {
    println!("module_ibc_handler 1 : {:?}", ibc_msg);

    // First check that we received the message from the server
    if ibc_msg.source_module.id().ne(MY_ADAPTER_ID) {
        println!("UnauthorizedIbcModule: {:?}", ibc_msg.source_module.clone());
        return Err(InterchainGovError::UnauthorizedIbcModule(
            ibc_msg.source_module.clone(),
        ));
    };

    // check that the sender

    let ibc_msg: InterchainGovIbcMsg = from_json(&ibc_msg.msg)?;

    println!("parsed_msg: {:?}", ibc_msg);

    match ibc_msg {
        InterchainGovIbcMsg::UpdateMembers { members } => {
            // check that the data has been finalized before
            let members_state = MEMBERS.load(deps.storage)?.1;
            if !members_state.is_finalized() {
                return Err(InterchainGovError::DataNotFinalized {
                    key: "members".to_string(),
                    state: members_state,
                });
            }

            Ok(app.response("module_ibc").add_message(msg))
        }
        _ => Err(InterchainGovError::UnauthorizedIbcMessage {}),
    }
}
