use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateState { gov_contract: Addr },
    AddFeeder { address: Addr },
    RemoveFeeder { address: Addr },
    Feed { height: Option<u64>, seed: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    Seed { height: u64 },
    RandomOne { 
        height: u64,
        entropy: Option<Vec<u8>>,
        max_value: u32,
    },
    RandomBetween { 
        height: u64,
        entropy: Option<Vec<u8>>,
        min_value: u32,
        max_value: u32,
    },
}
