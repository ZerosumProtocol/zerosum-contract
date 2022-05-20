use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{to_binary, Addr, CosmosMsg, StdResult, Uint128, WasmMsg};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub zerosum_token: Option<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateState {
        gov_contract: Option<Addr>,
        zerosum_token: Option<Addr>
    },
    AddSpender { addr: Addr },
    RemoveSpender { addr: Addr },
    Spend { addr: Addr, amount: Uint128 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetSpenders {},
    GetBalance {},
}

pub fn reward_msg(contract: Addr, addr: Addr, amount: Uint128) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: to_binary(&ExecuteMsg::Spend {
            addr,
            amount,
        })?,
        funds: vec![],
    }))
}