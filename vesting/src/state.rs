use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: Addr,
    pub zerosum_token: Addr,
    pub distributor_contract: Addr,
    pub vestings: Vec<Vesting>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Vesting {
    pub address: Addr,
    pub description: String,
    pub share: Decimal,
    pub unlock_round: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Reward {
    pub total_reward: Uint128,
    pub claimable_reward: Uint128,
}

// pool, round
pub const STATE: Item<State> = Item::new("state");
pub const REWARD: Map<Addr, Reward> = Map::new("reward");
pub const REST_REWARD: Item<Uint128> = Item::new("rest_reward");