#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, from_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, StdError, Uint128, Addr,
        CosmosMsg, WasmMsg, Order, Decimal};
use cw2::set_contract_version;
use cw20::{Cw20ReceiveMsg};

use cw_storage_plus::{Bound};

use zerosum::referral::{ExecuteMsg as ReferralExecuteMsg};
use zerosum::asset::{Asset, AssetInfo, token_asset};
use zerosum::round::{RoundInfo, get_round};
use zerosum::reward::{reward_msg};

use crate::error::ContractError;
use zerosum::house::{PoolResponse, ExecuteMsg, InstantiateMsg, QueryMsg, Cw20HookMsg};
use crate::state::{State, STATE, PoolInfo, POOLS, GameInfo, GAMES, DEPOSITS, DepositInfo, ROUNDS};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:house";
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
        terraswap_contract: msg.terraswap_contract.unwrap_or(Addr::unchecked("")),
        collector_contract: msg.collector_contract.unwrap_or(Addr::unchecked("")),
        referral_contract: msg.referral_contract.unwrap_or(Addr::unchecked("")),
        distributor_contract: msg.distributor_contract.unwrap_or(Addr::unchecked("")),
        reward_contract: msg.reward_contract.unwrap_or(Addr::unchecked("")),
        max_output_rate: msg.max_output_rate.unwrap_or_default(),
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
            terraswap_contract,
            collector_contract,
            referral_contract,
            distributor_contract,
            reward_contract,
            max_output_rate,
        } => {
            execute_update(
                deps, 
                info, 
                gov_contract, 
                zerosum_token,
                terraswap_contract,
                collector_contract,
                referral_contract,
                distributor_contract,
                reward_contract,
                max_output_rate,
            )
        }
        ExecuteMsg::CreatePool { asset, swap_contract, reward_weight } => 
            execute_create_pool(deps, env, info, asset, swap_contract, reward_weight),
        ExecuteMsg::UpdatePool { asset, swap_contract, reward_weight } => 
            execute_update_pool(deps, env, info, asset, swap_contract, reward_weight),
        ExecuteMsg::Deposit {} => {
            // Coin을 다시 Asset으로 바꾸고 execute_deposit에서 다시 AssetInfo를 체크해서 
            // 처리하는게 과연 효율적인가.. 따로 Coin을 바로 처리하는 함수가 있으면 되지않을까?
            // 가스비는 차이가 얼마일까? 그런걸 따져봐야 할듯.
            let deposit_asset = Asset::from(info.funds[0].clone());
            execute_deposit(deps, env, deposit_asset, info.sender)
        },
        ExecuteMsg::Withdraw { asset_info } => execute_withdraw(deps, env, info, asset_info),
        ExecuteMsg::Claim { asset_info } => execute_claim(deps, env, info, asset_info),
        ExecuteMsg::Settle { player, output } => {
            let coin = info.funds[0].clone();
            let asset_info = AssetInfo::NativeToken { denom: coin.denom };
            execute_settle(deps, env, info.sender, player, coin.amount, output, asset_info)
        },
        ExecuteMsg::AddGame { name, description, url, address, creator } => {
            execute_add_game(deps, info, name, description, url, address, creator)
        },
        ExecuteMsg::RemoveGame { address } => {
            execute_remove_game(deps, info, address)
        },
        ExecuteMsg::Collect {} => {
            execute_collect(deps, env)
        }
    }
}

fn execute_update(
    deps: DepsMut, 
    info: MessageInfo,
    gov_contract: Option<Addr>,
    zerosum_token: Option<Addr>,
    terraswap_contract: Option<Addr>,
    collector_contract: Option<Addr>,
    referral_contract: Option<Addr>,
    distributor_contract: Option<Addr>,
    reward_contract: Option<Addr>,
    max_output_rate: Option<Decimal>,
) -> Result<Response, ContractError> {
    let mut state: State = STATE.load(deps.storage)?;
    if state.gov_contract != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    if gov_contract.is_some() {
        state.gov_contract = gov_contract.unwrap();
    }
    if zerosum_token.is_some() {
        state.zerosum_token = zerosum_token.unwrap();
    }
    if terraswap_contract.is_some() {
        state.terraswap_contract = terraswap_contract.unwrap();
    }
    if collector_contract.is_some() {
        state.collector_contract = collector_contract.unwrap();
    }
    if referral_contract.is_some() {
        state.referral_contract = referral_contract.unwrap();
    }
    if distributor_contract.is_some() {
        state.distributor_contract = distributor_contract.unwrap();
    }
    if reward_contract.is_some() {
        state.reward_contract = reward_contract.unwrap();
    }
    if max_output_rate.is_some() {
        state.max_output_rate = max_output_rate.unwrap();
    }
    STATE.save(deps.storage, &state)?;
    Ok(Response::new().add_attribute("method", "update_state"))
}

fn receive_cw20(deps: DepsMut, env: Env, info: MessageInfo, cw20_msg: Cw20ReceiveMsg) -> Result<Response, ContractError> {
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Deposit { }) => {
            let deposit_asset = token_asset(info.sender, cw20_msg.amount);
            execute_deposit(deps, env, deposit_asset, Addr::unchecked(cw20_msg.sender))
        },
        Ok(Cw20HookMsg::Settle { player, output }) => {
            let asset_info = AssetInfo::Token { contract_addr: info.sender };
            execute_settle(deps, env, Addr::unchecked(cw20_msg.sender), player, cw20_msg.amount, output, asset_info)
        },
        Ok(Cw20HookMsg::AddRound { key, round }) => {
            execute_add_round(deps, env, info, key, round, Addr::unchecked(cw20_msg.sender), cw20_msg.amount)
        }
        Err(err) => Err(ContractError::Std(err)),
    }
}
fn execute_collect(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    let mut msgs: Vec<CosmosMsg> = vec![];
    POOLS.range(deps.storage, None, None, Order::Ascending).for_each(|pool| {
        let (_key, pool_info) : (_, PoolInfo) = pool.unwrap();
        let pool_balance = pool_info.asset_info.query_balance(&deps.querier, env.contract.address.clone()).unwrap_or_default();
        if pool_balance > pool_info.total_supply.checked_add(Uint128::from(1000000u64)).unwrap() {
            let collect_asset = Asset {
                info: pool_info.asset_info,
                amount: pool_balance - pool_info.total_supply,
            };
            msgs.push(collect_asset.into_msg(&deps.querier, state.collector_contract.clone()).unwrap())
        }
    });

    Ok(Response::new().add_messages(msgs).add_attribute("method", "collect"))
}

fn execute_create_pool(
    deps: DepsMut, 
    _env: Env, 
    info: MessageInfo, 
    asset_info: AssetInfo,
    swap_contract: Option<Addr>,
    reward_weight: Option<Decimal>,
) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    if state.gov_contract != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let key = asset_info.clone().to_string();
    let symbol = asset_info.clone().query_symbol(&deps.querier)?;
    POOLS.update(deps.storage, key.clone(), |old| {
        match old {
            Some(_) => Err(ContractError::AlreadyExist {}),
            None => {
                Ok(PoolInfo {
                    name: symbol,
                    asset_info: asset_info.clone(),
                    total_supply: Uint128::zero(),
                    swap_contract: swap_contract.unwrap_or(Addr::unchecked("")),
                    reward_weight: reward_weight.unwrap_or(Decimal::one())
                })
            }
        }
    })?;
    Ok(Response::new()
        .add_attribute("method", "create_pool")
        .add_attribute("pool", key))
}

fn execute_update_pool(
    deps: DepsMut, 
    _env: Env, 
    info: MessageInfo, 
    asset_info: AssetInfo,
    swap_contract: Option<Addr>,
    reward_weight: Option<Decimal>,
) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    if state.gov_contract != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let key = asset_info.clone().to_string();
    POOLS.update(deps.storage, key.clone(), |prev| {
        match prev {
            Some(mut pool) => {
                if reward_weight.is_some() {
                    pool.reward_weight = reward_weight.unwrap();
                }
                if swap_contract.is_some() {
                    pool.swap_contract = swap_contract.unwrap();
                }
                Ok(pool)
            },
            None => Err(ContractError::NotExist {}), 
        }
    })?;
    Ok(Response::new()
        .add_attribute("method", "create_pool")
        .add_attribute("pool", key))
}

fn execute_deposit(deps: DepsMut, env: Env,  asset: Asset, sender: Addr) -> Result<Response, ContractError> {
    let key = asset.clone().info.to_string();
    let mut msgs: Vec<CosmosMsg> = vec![];
    let state: State = STATE.load(deps.storage)?;
    POOLS.update(deps.storage, key.clone(), |old| {
        match old {
            Some(mut pool) => {
                pool.total_supply = pool.total_supply + asset.amount;
                Ok(pool)
            },
            None => Err(ContractError::NotExist {})
        }
    })?;

    let current_round = get_round(env.block.height);

    let may_deposit_info = DEPOSITS.may_load(deps.storage, (sender.clone(), key.clone()))?;
    let mut deposit_info: DepositInfo;
    if may_deposit_info.is_some() {
        deposit_info = may_deposit_info.unwrap();
        deposit_info.amount = deposit_info.amount + asset.amount;

        // ROUND 가 시작됬을 경우에만 리워드를 지급해야함.
        if current_round.is_some() {
            let mut reward = Uint128::zero();
            let start = if deposit_info.last_claim_round.is_some() { 
                Some(Bound::ExclusiveRaw(deposit_info.last_claim_round.unwrap().to_be_bytes().to_vec())) 
            } else {
                None
            };
            let end = Some(Bound::ExclusiveRaw(current_round.unwrap().to_be_bytes().to_vec()));

            ROUNDS.range(deps.storage, start, end, Order::Ascending).for_each(|result| {
                let (_, round) = result.unwrap();
                reward = reward + (deposit_info.amount * round.reward_ratio);
            });
            deposit_info.last_claim_round = Some(current_round.unwrap() - 1);

            if !reward.is_zero() {
                msgs.push(reward_msg(state.reward_contract, sender.clone(), reward)?);
            }
        }
    } else {
        deposit_info = DepositInfo {
            amount: asset.amount,
            last_claim_round: if current_round.is_some() { Some(current_round.unwrap()) } else { None },
        };
    }

    DEPOSITS.save(deps.storage, (sender.clone(), key.clone()), &deposit_info)?;

    Ok(Response::new().add_messages(msgs)
        .add_attribute("method", "deposit")
        .add_attribute("amount", asset.amount))
}

fn execute_withdraw(
    deps: DepsMut, 
    env: Env,
    info: MessageInfo,
    asset_info: AssetInfo, 
) -> Result<Response, ContractError> {
    let key = asset_info.clone().to_string();
    let state: State = STATE.load(deps.storage)?;
    let mut msgs: Vec<CosmosMsg> = vec![];
    let deposit_info: DepositInfo = DEPOSITS.load(deps.storage, (info.sender.clone(), key.clone()))?;
    POOLS.update(deps.storage, key.clone(), |old| {
        match old {
            Some(mut pool) => {
                if pool.total_supply < deposit_info.amount {
                    return Err(ContractError::NotEnoughToken {});
                }
                pool.total_supply = pool.total_supply - deposit_info.amount;
                Ok(pool)
            },
            None => Err(ContractError::NotExist {})
        }
    })?;

    let current_round = get_round(env.block.height);
    if current_round.is_some() {
        let mut reward = Uint128::zero();
        let start = Some(Bound::ExclusiveRaw(deposit_info.last_claim_round.unwrap().to_be_bytes().to_vec()));
        let end = Some(Bound::ExclusiveRaw(current_round.unwrap().to_be_bytes().to_vec()));
        ROUNDS.prefix(key.clone()).range(deps.storage, start, end, Order::Ascending).for_each(|result| {
            let (_, round) = result.unwrap();
            reward = reward + (deposit_info.amount * round.reward_ratio);
        });
        if !reward.is_zero() {
            msgs.push(reward_msg(state.reward_contract, info.sender.clone(), reward)?);
        }
    }

    DEPOSITS.remove(deps.storage, (info.sender.clone(), key.clone()));
    msgs.push(Asset {
        info: asset_info,
        amount: deposit_info.amount,
    }.into_msg(&deps.querier, info.sender)?);

    Ok(Response::new().add_messages(msgs))
}

fn execute_claim(
    deps: DepsMut, 
    env: Env,
    info: MessageInfo,
    asset_info: AssetInfo, 
) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    let key = asset_info.clone().to_string();
    let current_round = get_round(env.block.height);
    if current_round.is_none() {
        return Err(ContractError::NotExist {});
    }
    let mut deposit_info: DepositInfo = DEPOSITS.load(deps.storage, (info.sender.clone(), key.clone()))?;
    let mut reward = Uint128::zero();

    let start = if deposit_info.last_claim_round.is_some() {
        Some(Bound::ExclusiveRaw(deposit_info.last_claim_round.unwrap().to_be_bytes().to_vec()))
    } else {
        None
    };
    let end = Some(Bound::ExclusiveRaw(current_round.unwrap().to_be_bytes().to_vec()));
    ROUNDS.prefix(key.clone()).range(deps.storage, start, end, Order::Ascending).for_each(|result| {
        let (_, round) = result.unwrap();
        reward = reward + (deposit_info.amount * round.reward_ratio);
    });
    deposit_info.last_claim_round = Some(current_round.unwrap() - 1);
    DEPOSITS.save(deps.storage, (info.sender.clone(), key.clone()), &deposit_info)?;
    let mut msgs = vec![];

    if !reward.is_zero() {
        msgs.push(reward_msg(state.reward_contract, info.sender, reward)?);
    }

    Ok(Response::new().add_messages(msgs)
        .add_attribute("method", "execute_claim")
        .add_attribute("reward", reward))
}

fn execute_settle(
    deps: DepsMut,
    env: Env,
    game_contract: Addr, 
    player: Addr, 
    input: Uint128, 
    output: Uint128, 
    asset_info: AssetInfo
) -> Result<Response, ContractError> {
    let key = asset_info.clone().to_string();
    let _pool: PoolInfo = POOLS.may_load(deps.storage, key)?.expect("Not Exist Pool");
    // if !GAMES.has(deps.storage, game_contract) {
    //     return Err(ContractError::Unauthorized{}); 
    // }
    let _game: GameInfo = GAMES.may_load(deps.storage, game_contract)?.expect("Not Allowed Contract");
    let state: State = STATE.load(deps.storage)?;
    let mut msgs: Vec<CosmosMsg> = vec![];

    if !output.is_zero() {
        let pool_amount = asset_info.query_balance(&deps.querier, env.contract.address.clone())?;

        let max_output_amount = pool_amount * state.max_output_rate;
        let mut output_amount = output; 
        
        if output_amount > max_output_amount {
            output_amount = max_output_amount;
        }

        let output_asset = Asset {
            info: asset_info.clone(),
            amount: output_amount,
        };
        msgs.push(output_asset.into_msg(&deps.querier, player.clone())?);
    }

    // 함수로 뺄수 있는지 찾아보기. 없으면 말고 ㅡㅡ..ㅋ Reward Save
    let current_round = get_round(env.block.height);

    if current_round.is_some() {
        let share = input;
        // let share = if asset_info.is_ust() {
        //     input
        // } else {
        //     token_to_ust(&deps.querier, state.terraswap_contract, input)?
        // };

        // UST 가격으로 변환후 
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute{
                contract_addr: state.referral_contract.to_string(),
                funds: vec![],
                msg: to_binary(&ReferralExecuteMsg::AddShare {
                    address: player,
                    amount: share,
                })?
        }));
    }
    
    Ok(Response::new().add_messages(msgs)
        .add_attribute("method", "settle")
        .add_attribute("input", input)
        .add_attribute("output", output)
        .add_attribute("asset", asset_info.to_string()))
}

pub fn execute_add_round(deps: DepsMut, _env: Env, info: MessageInfo, key: Option<String>, round: u64, sender: Addr, reward_amount: Uint128, ) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    let mut msgs: Vec<CosmosMsg> = vec![];
    if info.sender != state.zerosum_token {
        return Err(ContractError::Unauthorized {});
    }
    if sender != state.distributor_contract {
        return Err(ContractError::Unauthorized {});
    }

    let pool_key = key.unwrap();
    let pool_info: PoolInfo = POOLS.load(deps.storage, pool_key.clone())?;
    if !pool_info.total_supply.is_zero() {
        ROUNDS.update(deps.storage, (pool_key, round), |prev| {
            match prev {
                Some(_) => Err(ContractError::AlreadyExist{}),
                None => Ok(RoundInfo {
                    reward_ratio: Decimal::from_ratio(reward_amount, pool_info.total_supply)
                })
            }
        })?;
        msgs.push(token_asset(state.zerosum_token, reward_amount).into_msg(&deps.querier, state.reward_contract)?);
    }

    Ok(Response::new().add_messages(msgs)
        .add_attribute("method", "execute_stake")
        .add_attribute("round", round.to_string()))
}

fn execute_add_game(
    deps: DepsMut,
    info: MessageInfo,
    name: String,
    description: String,
    url: String,
    address: Addr,
    creator: Addr,
) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    if info.sender != state.gov_contract {
        return Err(ContractError::Unauthorized {});
    }
    if GAMES.has(deps.storage, address.clone()) {
        return Err(ContractError::AlreadyExist {});
    }
    GAMES.save(deps.storage, address.clone(), &GameInfo {
        name: name.clone(),
        description: description,
        url: url,
        address: address,
        creator: creator
    })?;

    Ok(Response::new()
        .add_attribute("method", "add_game")
        .add_attribute("name", name)
        .add_attribute("address", "address"))
}

fn execute_remove_game(
    deps: DepsMut,
    info: MessageInfo,
    address: Addr,
) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    if info.sender != state.gov_contract {
        return Err(ContractError::Unauthorized {});
    }
    if !GAMES.has(deps.storage, address.clone()) {
        return Err(ContractError::NotExist {});
    }
    GAMES.remove(deps.storage, address.clone());

    Ok(Response::new().add_attribute("method", "remove_game").add_attribute("address", address))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::State {} => to_binary(&query_state(deps)?),
        QueryMsg::Pool { asset_info } => to_binary(&query_pool(deps, env, asset_info)?),
        QueryMsg::DepositInfo { asset_info, address } => to_binary(&query_deposit_info(deps, env, asset_info, address)?),
        QueryMsg::ClaimableReward { asset_info, address } => to_binary(&query_claimable(deps, env, asset_info, address)?),
        QueryMsg::Game { contract_addr } => to_binary(&query_game(deps, env, contract_addr)?),
        QueryMsg::Pools {} => to_binary(&query_pools(deps, env)?),
        QueryMsg::Games {} => to_binary(&query_games(deps, env)?),
        QueryMsg::CurrentRound {} => to_binary(&query_current_round(deps, env)?),
        QueryMsg::RoundInfo { key, round } => to_binary(&query_round_info(deps, env, key, round)?),
    }
}

fn query_state(deps: Deps) -> StdResult<State> {
    let state = STATE.load(deps.storage)?;
    Ok(state)
}

fn query_pool(deps: Deps, env: Env, asset_info: AssetInfo) -> StdResult<PoolResponse> {
    let key = asset_info.clone().to_string();
    let pool: PoolInfo = POOLS.load(deps.storage, key)?;

    Ok(PoolResponse { 
        name: pool.name,
        asset: Asset {
            info: asset_info,
            amount: pool.asset_info.query_balance(&deps.querier, env.contract.address.clone())?
        },
        total_supply: pool.total_supply,
    })
}

fn query_deposit_info(deps: Deps, _env: Env, asset_info: AssetInfo, address: Addr) -> StdResult<DepositInfo> {
    let key = asset_info.clone().to_string();
    let deposit_info = DEPOSITS.load(deps.storage, (address, key))?;
    Ok(deposit_info)
}

fn query_claimable(deps: Deps, env: Env, asset_info: AssetInfo, address: Addr) -> StdResult<Uint128> {
    let key = asset_info.clone().to_string();
    let current_round = get_round(env.block.height);
    if current_round.is_none() {
        return Ok(Uint128::zero());
    }
    let deposit_info: DepositInfo = DEPOSITS.load(deps.storage, (address.clone(), key.clone()))?;
    let mut reward = Uint128::zero();

    let start = if deposit_info.last_claim_round.is_some() {
        Some(Bound::ExclusiveRaw(deposit_info.last_claim_round.unwrap().to_be_bytes().to_vec()))
    } else {
        None
    };
    let end = Some(Bound::ExclusiveRaw(current_round.unwrap().to_be_bytes().to_vec()));
    ROUNDS.prefix(key.clone()).range(deps.storage, start, end, Order::Ascending).for_each(|result| {
        let (_, round) = result.unwrap();
        reward = reward + (deposit_info.amount * round.reward_ratio);
    });
    Ok(reward)
}

fn query_game(deps: Deps, _env: Env, contract_addr: Addr) -> StdResult<GameInfo> {
    let game_info = GAMES.load(deps.storage, contract_addr)?;
    Ok(game_info)
}

fn query_pools(deps: Deps, env: Env) -> StdResult<Vec<PoolResponse>> {
    let pools: Vec<PoolResponse> = POOLS.range(deps.storage, None, None, Order::Ascending).map(|pool| {
        let (_key, asset) = pool.unwrap();
        query_pool(deps, env.clone(), asset.asset_info).unwrap()
    }).collect();
    Ok(pools)
}

fn query_games(deps: Deps, _env: Env) -> StdResult<Vec<GameInfo>> {
    let games: Vec<GameInfo> = GAMES.range(deps.storage, None, None, Order::Ascending).map(|item| {
        let (_, game) = item.unwrap();
        game
    }).collect();
    Ok(games)
}

fn query_round_info(deps: Deps, env: Env, key: String, round: u64) -> StdResult<RoundInfo> {
    Ok(ROUNDS.load(deps.storage, (key, round))?)
}

fn query_current_round(_deps: Deps, env: Env) -> StdResult<Option<u64>> {
    Ok(get_round(env.block.height))
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
//     use cosmwasm_std::{coins, from_binary, BlockInfo, ContractInfo, Timestamp};
//     use cw_storage_plus::{Item, Map, U64Key};

//     fn mock_env_height(height: u64) -> Env {
//         let mut env = mock_env();
//         env.block.height = height;
//         env
//     }

//     #[test]
//     fn proper_initialization() {
//         let mut deps = mock_dependencies(&[]);

//         // let msg = InstantiateMsg {
//         //     zerosum_token: Some(Addr::unchecked("token")),
//         //     referral_contract: Some(Addr::unchecked("referral")),
//         //     terraswap_contract: Some(Addr::unchecked("terraswap")),
//         //     collector_contract: Some(Addr::unchecked("collector")),
//         //     max_output_rate: Some(Decimal::percent(1)),
//         //     distributor_contract: Some(Addr::unchecked("distributor")),
//         // };

//         // let anyone_info = mock_info("anyone", &coins(1000000, "uusd"));
//         // let _res = execute(deps.as_mut(), mock_env_height(10000), anyone_info.clone(), ExecuteMsg::Deposit {});

//         // let res = query(
//         //     deps.as_ref(), 
//         //     mock_env_height(11000), 
//         //     QueryMsg::DepositInfo { 
//         //         asset_info: AssetInfo::NativeToken{ denom: "uusd".to_string() }, 
//         //         address: Addr::unchecked("anyone") }
//         //     ).unwrap();
//         // let deposit_info: DepositInfo = from_binary(&res).unwrap();

//         let key = "uusd".to_string();
//         let current_round = Some(20u64);
//         let deposit_info = DepositInfo {
//             amount: Uint128::from(100u128),
//             last_claim_round: Some(10u64),
//         };
//         let mut reward = Uint128::zero();

//         let start = if deposit_info.last_claim_round.is_some() {
//             Some(Bound::exclusive(deposit_info.last_claim_round.unwrap().to_be_bytes()))
//         } else {
//             None
//         };
//         let end = Some(Bound::exclusive(current_round.unwrap().to_be_bytes()));

//         let ROUNDS2: Map<(String, U64Key), RoundInfo> = Map::new("test");
//         ROUNDS2.save(deps.as_mut().storage, (key.clone(), U64Key::from(10)), &RoundInfo {
//             reward_ratio: Decimal::one(),
//         }).unwrap();
//         ROUNDS2.save(deps.as_mut().storage, (key.clone(), U64Key::from(11)), &RoundInfo {
//             reward_ratio: Decimal::from_ratio(Uint128::from(100u128), Uint128::from(10u128)),
//         }).unwrap();
//         ROUNDS2.save(deps.as_mut().storage, (key.clone(), U64Key::from(13)), &RoundInfo {
//             reward_ratio: Decimal::one(),
//         }).unwrap();


//         assert_eq!(deposit_info.amount, Uint128::from(100u128));
//         let mut test= Uint128::zero();
//         ROUNDS2.prefix(key.clone()).range(deps.as_mut().storage, start, end, Order::Ascending).for_each(|result| {
//             let (_, round) = result.unwrap();
//             assert_eq!(deposit_info.amount, Uint128::from(1020u128));
//             test = deposit_info.amount;
//             reward = reward + (deposit_info.amount * round.reward_ratio);
//             assert_eq!(reward, Uint128::from(82u128));
//         });

//         assert_eq!(test, Uint128::from(100u128));



//         // let info = mock_info("creator", &coins(1000, "earth"));
//         // // we can just call .unwrap() to assert this was a success
//         // let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
//         // assert_eq!(0, res.messages.len());

//         // // it worked, let's query the state
//         // let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();
//         // let state: State = from_binary(&res).unwrap();
//         // assert_eq!(Addr::unchecked("candy"), state.token_contract);
//         // assert_eq!(Decimal::percent(1), state.max_output_rate);
//         // assert_eq!(10000u64, state.round_period);
//         // assert_eq!(42131u64, state.start_block);
//     }

//     // #[test]
//     // fn update_state() {
//     //     let mut deps = mock_dependencies(&vec![]);

//     //     let owner_info = mock_info("creator", &vec![]);
//     //     let anyone_info = mock_info("anyone", &vec![]);

//     //     let msg = InstantiateMsg {
//     //         zerosum_token: Some(Addr::unchecked("candy")),
//     //         referral_contract: Some(Addr::unchecked("referral")),
//     //         terraswap_contract: Some(Addr::unchecked("terraswap")),
//     //         collector_contract: Some(Addr::unchecked("collector")),
//     //         max_output_rate: Some(Decimal::percent(1)),
//     //         round_period: Some(10000u64),
//     //         start_block: Some(42131u64),
//     //     };

//     //     let _res = instantiate(deps.as_mut(), mock_env(), owner_info.clone(), msg).unwrap();

//     //     // beneficiary can release it
//     //     let msg = ExecuteMsg::UpdateState {
//     //         gov_contract: None,
//     //         token_contract: None,
//     //         referral_contract: None,
//     //         terraswap_contract: None,
//     //         collector_contract: None,
//     //         max_output_rate: Some(Decimal::percent(2)),
//     //         start_block: None,
//     //         round_period: None,
//     //     };
//     //     let res = execute(deps.as_mut(), mock_env(), anyone_info.clone(), msg.clone());
//     //     match res {
//     //         Err(_) => {},
//     //         _ => { panic!("must return error"); }
//     //     }
//     //     let _res = execute(deps.as_mut(), mock_env(), owner_info.clone(), msg).unwrap();

//     //     // should increase counter by 1
//     //     let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();
//     //     let state: State = from_binary(&res).unwrap();
//     //     assert_eq!(Addr::unchecked("candy"), state.token_contract);
//     //     assert_eq!(Decimal::percent(2), state.max_output_rate);
//     //     assert_eq!(10000u64, state.round_period);
//     //     assert_eq!(42131u64, state.start_block);
//     // }
// }
