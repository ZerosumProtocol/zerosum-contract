use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Addr};

use crate::asset::{AssetInfo, Asset};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub zerosum_token: Option<Addr>,
    pub trigger_address: Option<Addr>,
    pub distributor_contract: Option<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CollectorMsg {
    Collect {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateState {
        gov_contract: Option<Addr>,
        zerosum_token: Option<Addr>,
        trigger_address: Option<Addr>,
        distributor_contract: Option<Addr>,
    },
    Swap { asset_info: AssetInfo },
    SwapAll {},
    AddProfit { asset: AssetInfo, swap_contract: Option<Addr> },
    UpdateProfit { asset: AssetInfo, swap_contract: Addr },
    AddHarvestContract { addr: Addr },
    RemoveHarvestContract { addr: Addr },
    Action {},
    Collect {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    State {},
    Profit { asset: AssetInfo },
    Profits {},
    GetSwaps {
        start_after: Option<u64>,
        limit: Option<u64>,
    },
    GetActions {
        start_after: Option<u64>,
        limit: Option<u64>,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProfitResponse {
    pub asset: Asset,
    pub swap_contract: Addr,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    Distribute = 0,
    Burn = 1,
}