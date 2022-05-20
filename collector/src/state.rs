use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128, Timestamp, Storage};
use cw_storage_plus::{Item, Map};

use zerosum::asset::{AssetInfo, Asset};

use crate::error::ContractError;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub gov_contract: Addr,
    pub zerosum_token: Addr,
    pub trigger_address: Addr,
    pub distributor_contract: Addr,
    pub total_distribute_amount: Uint128,
    pub total_burn_amount: Uint128,
    pub total_lp_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Profit {
    pub asset_info: AssetInfo,
    pub swap_contract: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SwapHistory {
    pub time: Timestamp,
    pub asset: Asset,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ActionHistory {
    pub time: Timestamp,
    pub amount: Uint128,
}

pub fn next_action_index(store: &mut dyn Storage) -> u64 {
    ACTION_INDEX.update(store, |prev| -> Result<u64, ContractError> {
        Ok(prev + 1)
    }).unwrap()
}

pub fn next_swap_index(store: &mut dyn Storage) -> u64 {
    SWAP_INDEX.update(store, |prev| -> Result<u64, ContractError> {
        Ok(prev + 1)
    }).unwrap()
}

pub const PROFITS: Map<String, Profit> = Map::new("profits");
pub const STATE: Item<State> = Item::new("state");
pub const HARVEST_CONTRACTS: Map<Addr, bool> = Map::new("harvest_contracts");

pub const SWAP_INDEX: Item<u64> = Item::new("swap_history_index");
pub const ACTION_INDEX: Item<u64> = Item::new("action_history_index");

pub const SWAPS: Map<u64, SwapHistory> = Map::new("swaps");
pub const ACTIONS: Map<u64, ActionHistory> = Map::new("actions");