use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Addr, Uint128, Decimal};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub zerosum_token: Option<Addr>,
    pub trigger_address: Option<Addr>,
    pub distribute_amount: Option<Uint128>,
    pub distributions: Option<Vec<DistributionDetail>>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateState {
        gov_contract: Option<Addr>,
        zerosum_token: Option<Addr>,
        trigger_address: Option<Addr>,
        distribute_amount: Option<Uint128>,
        distributions: Option<Vec<DistributionDetail>>
    },
    Distribute { round: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    GetState {},
    GetRound {},
    GetLastRound {},
    GetDistributeHistory { round: u64 },
    GetDistributeHistories {
        start_round: Option<u64>,
        limit: Option<u64>
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DistributionDetail {
    pub description: String,
    pub key: Option<String>,
    pub address: Addr,
    pub ratio: Decimal,
}
