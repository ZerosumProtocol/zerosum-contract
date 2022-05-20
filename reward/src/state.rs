use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub gov_contract: Addr,
    pub zerosum_token: Addr,
}

pub const SPENDER: Map<Addr, bool> = Map::new("spender");
pub const STATE: Item<State> = Item::new("state");