use cosmos_anybuf::types::neutron::interchainqueries::KVKey;
use cosmwasm_std::StdResult;

/// Key for Proposals in the **gov** module's storage
/// <https://github.com/cosmos/cosmos-sdk/blob/35ae2c4c72d4aeb33447d5a7af23ca47f786606e/x/gov/types/keys.go#L41>
pub const PROPOSALS_KEY_PREFIX: u8 = 0x00;

/// Name of the standard **gov** Cosmos-SDK module
pub const GOV_STORE_KEY: &str = "gov";

/// Creates Cosmos-SDK governance key for proposal with specific id
/// <https://github.com/cosmos/cosmos-sdk/blob/35ae2c4c72d4aeb33447d5a7af23ca47f786606e/x/gov/types/keys.go#L41>
pub fn create_gov_proposal_key(proposal_id: u64) -> StdResult<Vec<u8>> {
    let mut key: Vec<u8> = vec![PROPOSALS_KEY_PREFIX];
    key.extend_from_slice(proposal_id.to_be_bytes().as_slice());

    Ok(key)
}

/// Creates Cosmos-SDK storage keys for list of proposals
pub fn create_gov_proposal_keys(proposals_ids: Vec<u64>) -> StdResult<Vec<KVKey>> {
    let mut kv_keys: Vec<KVKey> = Vec::with_capacity(proposals_ids.len());

    for id in proposals_ids {
        let kv_key = KVKey {
            path: GOV_STORE_KEY.to_string(),
            key: create_gov_proposal_key(id)?,
        };

        kv_keys.push(kv_key)
    }

    Ok(kv_keys)
}
