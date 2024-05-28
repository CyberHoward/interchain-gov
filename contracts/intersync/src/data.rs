use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Data {
    name: String,

    age: i64,

    sex: String,
}
