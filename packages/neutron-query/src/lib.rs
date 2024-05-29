use cosmwasm_schema::cw_serde;

pub mod icq;
pub mod gov;

pub use neutron_sdk;

pub const ICQ_PROTOCOL: &str = "ics23";

#[cw_serde]
/// Describes possible interchain query types
pub enum QueryType {
    #[serde(rename = "kv")]
    /// **kv** is an interchain query type to query KV values from remote chain
    KV,

    /// **tx** is an interchain query type to query transactions from remote chain
    #[serde(rename = "tx")]
    TX,
}

impl ToString for QueryType {
    fn to_string(&self) -> String {
        match self {
            QueryType::KV => "kv".to_string(),
            QueryType::TX => "tx".to_string(),
        }
    }
}