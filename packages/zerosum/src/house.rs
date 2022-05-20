use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Uint128, Addr, Decimal};
use cw20::Cw20ReceiveMsg;

use crate::asset::{AssetInfo, Asset};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub zerosum_token: Option<Addr>,
    pub terraswap_contract: Option<Addr>, 
    pub collector_contract: Option<Addr>,
    pub distributor_contract: Option<Addr>,
    pub referral_contract: Option<Addr>,
    pub reward_contract: Option<Addr>,
    pub max_output_rate: Option<Decimal>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    UpdateState {
        gov_contract: Option<Addr>,
        zerosum_token: Option<Addr>,
        terraswap_contract: Option<Addr>,
        collector_contract: Option<Addr>,
        distributor_contract: Option<Addr>,
        referral_contract: Option<Addr>,
        reward_contract: Option<Addr>,
        max_output_rate: Option<Decimal>,
    },
    CreatePool {
        asset: AssetInfo,
        swap_contract: Option<Addr>,
        reward_weight: Option<Decimal>,
    },
    UpdatePool {
        asset: AssetInfo,
        swap_contract: Option<Addr>,
        reward_weight: Option<Decimal>,
    },
    Deposit {},
    Settle {
        player: Addr,
        output: Uint128,
    },
    AddGame {
        name: String,
        description: String,
        url: String,
        address: Addr,
        creator: Addr,
    },
    RemoveGame {
        address: Addr,
    },
    Withdraw {
        asset_info: AssetInfo,
    },
    Claim {
        asset_info: AssetInfo,
    },
    Collect {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    Deposit {},
    Settle {
        player: Addr,
        output: Uint128,
    },
    AddRound {
        key: Option<String>,
        round: u64,
    },
    
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    State {},
    Pool { asset_info: AssetInfo },
    DepositInfo { asset_info: AssetInfo, address: Addr },
    ClaimableReward { asset_info: AssetInfo, address: Addr },
    Game { contract_addr: Addr },
    Pools {},
    Games {},
    CurrentRound {},
    RoundInfo { key: String, round: u64  },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolResponse {
    pub name: String,
    pub asset: Asset,
    pub total_supply: Uint128,
}