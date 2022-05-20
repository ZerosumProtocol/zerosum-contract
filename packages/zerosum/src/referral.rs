use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw20::{Cw20ReceiveMsg};
use cosmwasm_std::{Uint128, Addr, Decimal};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub zerosum_token: Option<Addr>,
    pub house_coutract: Option<Addr>,
    pub distributor_contract: Option<Addr>,
    pub register_referrer_fee: Option<Uint128>,
    pub referral_ratio: Option<Vec<Decimal>>,
    pub collector_contract: Option<Addr>,
    pub reward_contract: Option<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    UpdateState {
        gov_contract: Option<Addr>,
        zerosum_token: Option<Addr>,
        house_coutract: Option<Addr>,
        distributor_contract: Option<Addr>,
        register_referrer_fee: Option<Uint128>,
        referral_ratio: Option<Vec<Decimal>>,
        collector_contract: Option<Addr>,
        reward_contract: Option<Addr>,
    },
    AddShare {
        address: Addr,
        amount: Uint128
    },
    Claim {
        start_round: Option<u64>
    },
    ClaimReferral {},
    AddFollowing { address: Addr },
    Collect {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    RegisterReferral {
        addr: Option<Addr>,
        name: Option<String>,
        description: Option<String>,
    },
    AddRound { round: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetState {},
    GetReferral { addr: Addr },
    GetReferrals { start_after: Option<u64>, limit: Option<u64> },
    GetRound { round: u64 },
    GetRounds { start_after: Option<u64>, limit: Option<u64> },
    GetFollowing { addr: Addr },
    GetFollowers { reward_addr: Addr, target_addr: Addr },
    GetRewardShare { round: Option<u64>, addr: Addr },
    GetRewardShares { round: Option<u64> },
    GetReferralReward { addr: Addr },
    GetReward { addr: Addr },
    GetLastClaimRound { addr: Addr }
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserAmountInfo {
    pub address: Addr,
    pub amount: Uint128,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RoundShareInfo {
    pub total: Uint128,
    pub share: Uint128,
}
// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RoundShareInfos {
    pub total: Uint128,
    pub shares: Vec<UserShareInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserShareInfo {
    pub address: Addr,
    pub share: Uint128,
}