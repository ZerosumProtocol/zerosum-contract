use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    QueryRequest, StdResult, Uint128, WasmQuery, Decimal,
};

const START_HEIGHT: u64 = 4;
const ROUND_PERIOD: u64 = 10;
const DISTRIBUTE_PERIOD: [(u8, u64, u64, u128); 4] = [
    (1u8, 0u64, 365u64, 400_000_000u128),
    (2u8, 365u64, 730u64, 300_000_000u128),
    (3u8, 730u64, 1095u64, 200_000_000u128),
    (4u8, 1095u64, 1460u64, 100_000_000u128)];

// id, start_round, end_round, total_mint
// const DISTRIBUTE_PERIOD: Vec<(u8, u64, u64, u128)> = vec![
//     (1u8, 0u64, 365u64, 400_000_000u128),
//     (2u8, 365u64, 730u64, 300_000_000u128),
//     (3u8, 730u64, 1095u64, 200_000_000u128),
//     (4u8, 1095u64, 1460u64, 100_000_000u128),
// ];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    AddRound {
        key: Option<String>,
        round: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RoundInfo {
    pub reward_ratio: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Period {
    pub id: u8,
    pub start_round: u64,
    pub end_round: u64,
    pub total_distribute_amount: Uint128,
    pub distribute_amount_per_round: Uint128,
}

pub fn get_round(height: u64) -> Option<u64> {
    if height < START_HEIGHT {
        None
    } else {
        Some((height - START_HEIGHT) / ROUND_PERIOD)
    }
}

pub fn get_period(height: u64) -> Option<Period> {
    let round = get_round(height);
    if round.is_some() {
        let current_round = round.unwrap();
        for period in DISTRIBUTE_PERIOD.iter() {
            let (id, start_round, end_round, total_distribute_amount) = period;
            if start_round < &current_round && end_round >= &current_round {
                let total_distribute_amount =  Uint128::from(*total_distribute_amount);
                return Some(Period {
                    id: *id,
                    start_round: *start_round,
                    end_round: *end_round,
                    total_distribute_amount: total_distribute_amount,
                    distribute_amount_per_round: total_distribute_amount / Uint128::from(end_round - start_round),
                });
            }
        }
    }
    return None;
}