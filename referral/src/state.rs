use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128, Decimal};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub gov_contract: Addr,
    pub zerosum_token: Addr,
    pub house_coutract: Addr,
    pub distributor_contract: Addr,
    pub register_referrer_fee: Uint128,
    pub referral_ratio: Vec<Decimal>,
    pub collector_contract: Addr,
    pub reward_contract: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RoundInfo {
    pub reward_ratio: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Referral {
    pub name: String,
    pub description: String,
    // pub total_reward: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Reward {
    pub claimable_reward: Uint128,
    pub total_reward: Uint128,
}

pub const STATE: Item<State> = Item::new("state");
pub const REFERRALS: Map<Addr, Referral> = Map::new("referrals");

pub const FOLLOWING: Map<Addr, Addr> = Map::new("following");
pub const FOLLOWERS: Map<(Addr, u64), Addr> = Map::new("followers");

pub const FOLLOWER_IDX: Map<Addr, u64> = Map::new("follower_idx");

pub const LAST_CLAIM_ROUND: Map<Addr, u64> = Map::new("last_claim_round");
// 해당 라운드의 특정유저의 Reward 지분
pub const REWARD_SHARE: Map<(u64, Addr), Uint128> = Map::new("reward_share");
// 해당 라운드의 전체 Reward 전체지분
pub const TOTAL_REWARD_SHARE: Map<u64, Uint128> = Map::new("total_reward_share");
// 레퍼럴로 받은 리워드
pub const REWARDS: Map<Addr, Reward> = Map::new("rewards");
// 레퍼럴로 누가 누구에게 얼마나 리워드를 지급했는지 팔로잉, 팔로워 순서
pub const REFERRAL_HISTORY: Map<(Addr, Addr), Uint128> = Map::new("refferal_history");

pub const ROUNDS: Map<u64, RoundInfo> = Map::new("rounds");