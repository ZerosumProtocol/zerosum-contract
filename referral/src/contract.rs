#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, from_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Addr, Uint128, Order, Decimal, CosmosMsg, WasmMsg};
use cw2::set_contract_version;
use cw20::{Cw20ReceiveMsg, Cw20ExecuteMsg};
use cw_storage_plus::{Bound};

use zerosum::referral::{ExecuteMsg, InstantiateMsg, QueryMsg, Cw20HookMsg, UserAmountInfo, RoundShareInfo, RoundShareInfos, UserShareInfo};
use zerosum::round::{get_round};
use zerosum::asset::{token_asset, token_asset_info};
use zerosum::reward::{reward_msg};

use crate::error::ContractError;
use crate::state::{State, STATE, RoundInfo, Referral, REFERRALS, REWARD_SHARE, TOTAL_REWARD_SHARE, REFERRAL_HISTORY, 
    REWARDS, ROUNDS, LAST_CLAIM_ROUND, FOLLOWING, FOLLOWERS, FOLLOWER_IDX, Reward};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:referral";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        gov_contract: info.sender.clone(),
        zerosum_token: msg.zerosum_token.unwrap_or(Addr::unchecked("")),
        house_coutract: msg.house_coutract.unwrap_or(Addr::unchecked("")),
        distributor_contract: msg.distributor_contract.unwrap_or(Addr::unchecked("")),
        register_referrer_fee: msg.register_referrer_fee.unwrap_or_default(),
        referral_ratio: msg.referral_ratio.unwrap_or_default(),
        collector_contract: msg.collector_contract.unwrap_or(Addr::unchecked("")),
        reward_contract: msg.reward_contract.unwrap_or(Addr::unchecked("")),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("gov_contract", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::UpdateState {
            gov_contract,
            zerosum_token,
            house_coutract,
            distributor_contract,
            register_referrer_fee,
            referral_ratio,
            collector_contract,
            reward_contract,
        } => execute_update_state(
            deps,
            info,
            gov_contract,
            zerosum_token,
            house_coutract,
            distributor_contract,
            register_referrer_fee,
            referral_ratio,
            collector_contract,
            reward_contract,
        ),
        ExecuteMsg::AddFollowing { address } => execute_add_following(deps, info, address), 
        ExecuteMsg::AddShare { address, amount } => execute_add_share(deps, env, info, address, amount),
        ExecuteMsg::Claim { start_round } => execute_claim(deps, env, info, start_round),
        ExecuteMsg::ClaimReferral {} => execute_claim_referral(deps, info),
        ExecuteMsg::Collect {} => execute_collect(deps, env),
    }
}

fn receive_cw20(deps: DepsMut, _env: Env, info: MessageInfo, cw20_msg: Cw20ReceiveMsg) -> Result<Response, ContractError> {
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::RegisterReferral { addr, name, description }) => {
            execute_register_referral(deps, info, addr, name, description, Addr::unchecked(cw20_msg.sender), cw20_msg.amount)
        },
        Ok(Cw20HookMsg::AddRound { round }) =>
            execute_add_round(deps, info, round, Addr::unchecked(cw20_msg.sender), cw20_msg.amount),
        Err(err) => Err(ContractError::Std(err)),
    }
}

pub fn execute_update_state(
    deps: DepsMut,
    info: MessageInfo,
    gov_contract: Option<Addr>,
    zerosum_token: Option<Addr>,
    house_coutract: Option<Addr>,
    distributor_contract: Option<Addr>,
    register_referrer_fee: Option<Uint128>,
    referral_ratio: Option<Vec<Decimal>>,
    collector_contract: Option<Addr>,
    reward_contract: Option<Addr>,
) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if state.gov_contract != info.sender {
            return Err(ContractError::Unauthorized {});
        }
        if gov_contract.is_some() {
            state.gov_contract = gov_contract.unwrap(); 
        }
        if zerosum_token.is_some() {
            state.zerosum_token = zerosum_token.unwrap(); 
        }
        if house_coutract.is_some() {
            state.house_coutract = house_coutract.unwrap(); 
        }
        if distributor_contract.is_some() {
            state.distributor_contract = distributor_contract.unwrap(); 
        }
        if register_referrer_fee.is_some() {
            state.register_referrer_fee = register_referrer_fee.unwrap(); 
        }
        if referral_ratio.is_some() {
            state.referral_ratio = referral_ratio.unwrap(); 
        }
        if collector_contract.is_some() {
            state.collector_contract = collector_contract.unwrap(); 
        }
        if reward_contract.is_some() {
            state.reward_contract = reward_contract.unwrap();
        }
        Ok(state)
    })?;

    Ok(Response::new().add_attribute("method", "try_increment"))
}

pub fn execute_add_following(deps: DepsMut, info: MessageInfo, address: Addr) -> Result<Response, ContractError> {
    if address == info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let mut next_address = address.clone();
    loop {
        let result = FOLLOWING.may_load(deps.storage, next_address)?;
        if result.is_some() {
            next_address = result.unwrap();
            if next_address == info.sender {
                return Err(ContractError::Unauthorized {});
            }
        } else {
            break;
        }
    }
    if !REFERRALS.has(deps.storage, address.clone()) {
        return Err(ContractError::NotReferral {});
    }

    FOLLOWING.update(deps.storage, info.sender.clone(), |prev| {
        match prev {
            Some(_) => Err(ContractError::AlreadyExist {}),
            None => Ok(address.clone()),
        }
    })?;
    let follower_idx = FOLLOWER_IDX.update(deps.storage, address.clone(), |prev| -> Result<u64, ContractError> {
        match prev {
            Some(idx) => Ok(idx + 1),
            None => Ok(0),
        }
    })?;
    FOLLOWERS.save(deps.storage, (address, follower_idx), &info.sender)?;
    Ok(Response::new()
        .add_attribute("method", "add_following"))
}

pub fn execute_register_referral(deps: DepsMut, info: MessageInfo, addr: Option<Addr>, name: Option<String>, description: Option<String>, sender: Addr, amount: Uint128) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    if state.zerosum_token != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    if amount < state.register_referrer_fee {
        return Err(ContractError::NotEnough {});
    }
    REFERRALS.update(deps.storage, addr.unwrap_or(sender), |prev| {
        match prev {
            Some(_) => Err(ContractError::AlreadyExist {}),
            None => Ok(Referral {
                name: name.unwrap_or_default(),
                description: description.unwrap_or_default(),
            })
        }
    })?;
    Ok(Response::new()
        .add_attribute("method", "register_referral"))
}

pub fn execute_add_round(deps: DepsMut, info: MessageInfo, round: u64, sender: Addr, amount:Uint128) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    let mut msgs: Vec<CosmosMsg> = vec![];
    if state.zerosum_token != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    if state.distributor_contract != sender {
        return Err(ContractError::Unauthorized {});
    }
    if ROUNDS.has(deps.storage, round) {
        return Err(ContractError::AlreadyExist {});
    }
    let total_share = TOTAL_REWARD_SHARE.may_load(deps.storage, round)?.unwrap_or_default();
    if !total_share.is_zero() {
        let reward_ratio = Decimal::from_ratio(amount, total_share);
        ROUNDS.save(deps.storage, round, &RoundInfo {
            reward_ratio
        })?;
        msgs.push(token_asset(state.zerosum_token, amount).into_msg(&deps.querier, state.reward_contract)?);
    }
    
    Ok(Response::new().add_messages(msgs)
        .add_attribute("method", "add_round")
        .add_attribute("round", round.to_string())
        .add_attribute("amount", amount)
        .add_attribute("total_share", total_share.to_string()))
}

pub fn execute_add_share(deps: DepsMut, env: Env, info: MessageInfo, address: Addr, amount: Uint128) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    if state.house_coutract != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let current_round = get_round(env.block.height);
    if current_round.is_some() {
        REWARD_SHARE.update(deps.storage, (current_round.unwrap(), address), |prev| -> Result<Uint128, ContractError> {
            match prev {
                Some(prev_amount) => Ok(prev_amount + amount),
                None => Ok(amount),
            }
        })?;
        TOTAL_REWARD_SHARE.update(deps.storage, current_round.unwrap(), |prev| -> Result<Uint128, ContractError> {
            match prev {
                Some(prev_amount) => Ok(prev_amount + amount),
                None => Ok(amount),
            }
        })?;
    }
    Ok(Response::new().add_attribute("method", "add_share"))
}

pub fn execute_claim_referral(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut reward: Uint128 = Uint128::zero();
    let state: State = STATE.load(deps.storage)?;
    REWARDS.update(deps.storage, info.sender.clone(), |prev| {
        match prev {
            Some(mut prev_reward) => {
                reward = prev_reward.claimable_reward;
                prev_reward.claimable_reward = Uint128::zero();
                Ok(prev_reward)
            },
            None => Err(ContractError::NotEnough {})
        }
    })?;
    if !reward.is_zero() {
        msgs.push(reward_msg(state.reward_contract, info.sender, reward)?);
    }
    
    Ok(Response::new().add_messages(msgs)
        .add_attribute("method", "claim_referral")
        .add_attribute("reward", reward))
}

pub fn execute_claim(deps: DepsMut, env: Env, info: MessageInfo, start_round_param: Option<u64>) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    let current_round = get_round(env.block.height).expect("NOT STARTED ROUND");
    let last_claim_round = LAST_CLAIM_ROUND.may_load(deps.storage, info.sender.clone())?;
    let start_round = if last_claim_round.is_some() {
        last_claim_round.unwrap() + 1
    } else {
        start_round_param.unwrap_or_default()
    };
    LAST_CLAIM_ROUND.save(deps.storage, info.sender.clone(), &(current_round - 1))?;
    
    let start = Some(Bound::InclusiveRaw(start_round.to_be_bytes().to_vec()));
    let end = Some(Bound::ExclusiveRaw(current_round.to_be_bytes().to_vec()));
    
    let mut reward = Uint128::zero();

    ROUNDS.range(deps.storage, start, end, Order::Ascending).for_each(|item| {
        let (key, round_info) = item.unwrap();
        let my_share = REWARD_SHARE.may_load(deps.storage, (key, info.sender.clone())).unwrap();
        if my_share.is_some() {
            reward = reward + (my_share.unwrap() * round_info.reward_ratio);
        }
    });

    let mut msgs = vec![];
    if !reward.is_zero() {
        let mut iter = state.referral_ratio.iter();
        let return_amount = reward * *iter.next().unwrap();
        let mut rest_reward = reward - return_amount;
        let mut target_address = info.sender.clone();
        loop {
            let ratio = iter.next();
            if ratio.is_some() {
                let reward_amount = reward * *ratio.unwrap();
                rest_reward = rest_reward - reward_amount;
                REWARDS.update(deps.storage, target_address.clone(), |old| -> Result<Reward, ContractError> {
                    match old {
                        Some(mut prev_reward) => {
                            prev_reward.total_reward += reward_amount;
                            prev_reward.claimable_reward += reward_amount;
                            Ok(prev_reward)
                        },
                        None => Ok(Reward {
                            claimable_reward: reward_amount,
                            total_reward: reward_amount
                        }),
                    }
                })?;
                REFERRAL_HISTORY.update(deps.storage, (target_address.clone(), info.sender.clone()), |prev| -> Result<Uint128, ContractError> {
                    match prev {
                        Some(prev_amount) => Ok(prev_amount + reward_amount),
                        None => Ok(reward_amount)
                    }
                })?;

                let next_addr = FOLLOWING.may_load(deps.storage, target_address.clone()).unwrap();
                if next_addr.is_some() {
                    target_address = next_addr.unwrap();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        if !rest_reward.is_zero() {
            // 남은 리워드 다시 반환 (referral_contract의 수익금)
            msgs.push(reward_msg(state.reward_contract.clone(), env.contract.address, rest_reward)?);
        }
        msgs.push(reward_msg(state.reward_contract, info.sender, return_amount)?);
    }
    Ok(Response::new().add_messages(msgs).add_attribute("method", "claim"))
}

fn execute_collect(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    let mut msgs: Vec<CosmosMsg> = vec![];
    let balance = token_asset_info(state.zerosum_token.clone()).query_balance(&deps.querier, env.contract.address)?;
    if !balance.is_zero() {
        msgs.push(token_asset(state.zerosum_token, balance).into_msg(&deps.querier, state.collector_contract).unwrap())
    }
    Ok(Response::new().add_messages(msgs).add_attribute("method", "collect"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetState {} => to_binary(&query_state(deps)?),
        QueryMsg::GetReferral { addr } => to_binary(&query_referral(deps, addr)?),
        QueryMsg::GetReferrals { start_after, limit } => to_binary(&query_referrals(deps, start_after, limit)?),
        QueryMsg::GetRound { round } => to_binary(&query_round(deps, round)?),
        QueryMsg::GetRounds { start_after, limit } => to_binary(&query_rounds(deps, start_after, limit)?),
        QueryMsg::GetFollowing { addr } => to_binary(&query_following(deps, addr)?),
        QueryMsg::GetFollowers { reward_addr, target_addr } => to_binary(&query_followers(deps, reward_addr, target_addr)?),
        QueryMsg::GetRewardShare { round, addr } => to_binary(&query_reward_share(deps, env, round, addr)?),
        QueryMsg::GetRewardShares { round } => to_binary(&query_reward_shares(deps, env, round)?),
        QueryMsg::GetReferralReward { addr } => to_binary(&query_referral_reward(deps, env, addr)?),
        QueryMsg::GetReward { addr } => to_binary(&query_reward(deps, env, addr)?),
        QueryMsg::GetLastClaimRound { addr } => to_binary(&query_last_claim_round(deps, env, addr)?),
    }
}

fn query_state(deps: Deps) -> StdResult<State> {
    let state = STATE.load(deps.storage)?;
    Ok(state)
}

fn query_referral(deps: Deps, addr: Addr) -> StdResult<Referral> {
    Ok(REFERRALS.load(deps.storage, addr)?)
}

fn query_referrals(deps: Deps, start_after: Option<u64>, limit: Option<u64>) -> StdResult<Vec<Referral>> {
    let start = if start_after.is_some() {
        Some(Bound::ExclusiveRaw(start_after.unwrap().to_be_bytes().to_vec()))
    } else {
        None
    };
    let referrals: Vec<Referral> = REFERRALS.range(deps.storage, start, None, Order::Ascending).take(limit.unwrap_or(20) as usize).map(|item| {
        let (_, referral) = item.unwrap();
        referral
    }).collect();

    Ok(referrals)
}

fn query_round(deps: Deps, round: u64) -> StdResult<RoundInfo> {
    Ok(ROUNDS.load(deps.storage, round)?)
}

fn query_rounds(deps: Deps, start_after: Option<u64>, limit: Option<u64>) -> StdResult<Vec<RoundInfo>> {
    let start = if start_after.is_some() {
        Some(Bound::ExclusiveRaw(start_after.unwrap().to_be_bytes().to_vec()))
    } else {
        None
    };
    let rounds: Vec<RoundInfo> = ROUNDS.range(deps.storage, start, None, Order::Ascending).take(limit.unwrap_or(20) as usize).map(|item| {
        let (_, round) = item.unwrap();
        round
    }).collect();

    Ok(rounds)
}

fn query_following(deps: Deps, addr: Addr) -> StdResult<Addr> {
    Ok(FOLLOWING.load(deps.storage, addr)?)
}

fn query_followers(deps: Deps, reward_addr: Addr, target_addr: Addr) -> StdResult<Vec<UserAmountInfo>> {
    let followers: Vec<UserAmountInfo> = FOLLOWERS.prefix(target_addr.clone()).range(deps.storage, None, None, Order::Ascending).map(|item| {
        let (_key, follower) = item.unwrap();
        let amount = REFERRAL_HISTORY.may_load(deps.storage, (reward_addr.clone(), follower.clone())).unwrap();
        UserAmountInfo {
            address: follower,
            amount: amount.unwrap_or_default()
        }
    }).collect();
    Ok(followers)
}

fn query_reward_share(deps: Deps, env: Env, round: Option<u64>, addr: Addr) -> StdResult<RoundShareInfo> {
    let real_round = round.unwrap_or(get_round(env.block.height).unwrap());
    let total = TOTAL_REWARD_SHARE.may_load(deps.storage, real_round)?.unwrap_or_default();
    let share = REWARD_SHARE.may_load(deps.storage, (real_round, addr))?.unwrap_or_default();
    Ok(RoundShareInfo {
        total,
        share
    })
}

fn query_reward_shares(deps: Deps, env: Env, round: Option<u64>) -> StdResult<RoundShareInfos> {
    let real_round = round.unwrap_or(get_round(env.block.height).unwrap());
    let total = TOTAL_REWARD_SHARE.may_load(deps.storage, real_round)?.unwrap_or_default();
    let shares: Vec<UserShareInfo> = REWARD_SHARE.prefix(real_round).range(deps.storage, None, None, Order::Ascending).map(|item| {
        let (address, share) = item.unwrap();
        UserShareInfo {
            address,
            share: share,
        }
    }).collect();
    Ok(RoundShareInfos {
        total,
        shares
    })
}

fn query_referral_reward(deps: Deps, _env: Env, addr: Addr) -> StdResult<Reward> {
    Ok(REWARDS.load(deps.storage, addr)?)
}

fn query_reward(deps: Deps, env: Env, addr: Addr) -> StdResult<Uint128> {
    let state: State = STATE.load(deps.storage)?;
    let last_claim_round = LAST_CLAIM_ROUND.may_load(deps.storage, addr.clone())?;
    let start_round = if last_claim_round.is_some() {
        last_claim_round.unwrap() + 1
    } else {
        0
    };
    let current_round = get_round(env.block.height).unwrap();
    let start = Some(Bound::InclusiveRaw(start_round.to_be_bytes().to_vec()));
    let end = Some(Bound::ExclusiveRaw(current_round.to_be_bytes().to_vec()));

    let mut reward = Uint128::zero();
    let ratio = state.referral_ratio[0];
    ROUNDS.range(deps.storage, start, end, Order::Ascending).for_each(|item| {
        let (key, round_info) = item.unwrap();
        let my_share = REWARD_SHARE.may_load(deps.storage, (key, addr.clone())).unwrap();
        if my_share.is_some() {
            reward = reward + (my_share.unwrap() * round_info.reward_ratio * ratio);
        }
    });
    Ok(reward)
}

fn query_last_claim_round(deps: Deps, _env: Env, addr: Addr) -> StdResult<u64> {
    Ok(LAST_CLAIM_ROUND.load(deps.storage, addr)?)
}