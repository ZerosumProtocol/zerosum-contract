use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub gov_contract: Addr,
}

pub const FEEDERS: Map<Addr, bool> = Map::new("feeders");
pub const SEEDS: Map<u64, String> = Map::new("seeds");
pub const STATE: Item<State> = Item::new("state");
