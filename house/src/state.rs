use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use zerosum::asset::{AssetInfo};
use zerosum::round::{RoundInfo};

use cosmwasm_std::{Addr, Uint128, Decimal};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub gov_contract: Addr,
    pub zerosum_token: Addr,
    pub referral_contract: Addr,
    pub terraswap_contract: Addr, 
    pub collector_contract: Addr,
    pub distributor_contract: Addr,
    pub reward_contract: Addr,
    pub max_output_rate: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GameInfo {
    pub name: String,
    pub description: String,
    pub url: String,
    pub address: Addr,
    pub creator: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolInfo {
    pub name: String,
    pub asset_info: AssetInfo,
    pub total_supply: Uint128,
    pub swap_contract: Addr,
    pub reward_weight: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DepositInfo {
    pub amount: Uint128,
    pub last_claim_round: Option<u64>,
}

pub const STATE: Item<State> = Item::new("state");
pub const GAMES: Map<Addr, GameInfo> = Map::new("games");
pub const POOLS: Map<String, PoolInfo> = Map::new("pools");

// human_address, pool
pub const DEPOSITS: Map<(Addr, String), DepositInfo> = Map::new("deposits");

// pool, round
pub const ROUNDS: Map<(String, u64), RoundInfo> = Map::new("rounds");