use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::DistributionDetail;

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub gov_contract: Addr,
    pub zerosum_token: Addr,
    pub trigger_address: Addr,
    pub distribute_amount: Uint128,
    pub distributions: Vec<DistributionDetail>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DistributionHistory {
    pub round: u64,
    pub distribute_amount: Uint128,
    pub distributions: Vec<DistributionAmount>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DistributionAmount {
    pub key: Option<String>,
    pub address: Addr,
    pub amount: Uint128,
}

pub const STATE: Item<State> = Item::new("state");
pub const LAST_ROUND: Item<u64> = Item::new("last_round");
pub const HISTORY: Map<u64, DistributionHistory> = Map::new("history");
