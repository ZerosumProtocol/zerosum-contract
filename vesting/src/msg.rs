use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Addr, Uint128};
use cw20::{Cw20ReceiveMsg};

use crate::state::{Vesting};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub zerosum_token: Option<Addr>,
    pub distributor_contract: Option<Addr>,
    pub vestings: Option<Vec<Vesting>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    UpdateState {
        owner: Option<Addr>,
        zerosum_token: Option<Addr>,
        distributor_contract: Option<Addr>,
        vestings: Option<Vec<Vesting>>
    },
    Claim {},
    Send { amount: Uint128, recipient: Addr },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    AddRound { key: Option<String>, round: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetState {},
    GetReward { addr: Addr },
    GetRestReward {},
}