#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Addr, Order, Uint128,
    CosmosMsg, WasmMsg, Coin};
use cw2::set_contract_version;
use cw_storage_plus::{Bound};
use zerosum::collector::{ExecuteMsg, InstantiateMsg, QueryMsg, ProfitResponse, CollectorMsg};
use zerosum::terraswap::{ExecuteMsg as SwapExecuteMsg};
use zerosum::round::{get_period};

use crate::error::ContractError;
use crate::state::{State, STATE, Profit, PROFITS, SWAPS, ACTIONS, ACTION_INDEX, SWAP_INDEX, next_action_index, next_swap_index, HARVEST_CONTRACTS, SwapHistory, ActionHistory};

use cw20::{Cw20ExecuteMsg};

use zerosum::asset::{AssetInfo, Asset, token_asset_info};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:collector";
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
        trigger_address: msg.trigger_address.unwrap_or(Addr::unchecked("")),
        distributor_contract: msg.distributor_contract.unwrap_or(Addr::unchecked("")),
        total_distribute_amount: Uint128::zero(),
        total_burn_amount: Uint128::zero(),
        total_lp_amount: Uint128::zero(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;
    ACTION_INDEX.save(deps.storage, &0u64)?;
    SWAP_INDEX.save(deps.storage, &0u64)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("distributor_contract", state.distributor_contract.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateState { gov_contract, zerosum_token, trigger_address, distributor_contract} => 
            execute_update_state(deps, info, gov_contract, zerosum_token, trigger_address, distributor_contract),
        ExecuteMsg::Swap { asset_info } => execute_swap(deps, env, info, asset_info),
        ExecuteMsg::SwapAll {} => execute_swap_all(deps, env, info),
        ExecuteMsg::AddProfit { asset, swap_contract } => execute_add_profit(deps, info, asset, swap_contract),
        ExecuteMsg::UpdateProfit { asset, swap_contract } => execute_update_profit(deps, info, asset, swap_contract),
        ExecuteMsg::AddHarvestContract { addr } => execute_add_harvest_contract(deps, info, addr),
        ExecuteMsg::RemoveHarvestContract { addr } => execute_remove_harvest_contract(deps, info, addr),
        ExecuteMsg::Action {} => execute_action(deps, env, info),
        ExecuteMsg::Collect {} => execute_collect(deps, env, info),
    }
}

pub fn execute_update_state(
    deps: DepsMut, 
    info: MessageInfo, 
    gov_contract: Option<Addr>, 
    zerosum_token: Option<Addr>, 
    trigger_address: Option<Addr>,
    distributor_contract: Option<Addr>,
) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if info.sender != state.gov_contract {
            return Err(ContractError::Unauthorized {});
        }
        if gov_contract.is_some() {
            state.gov_contract = gov_contract.unwrap();
        }
        if zerosum_token.is_some() {
            state.zerosum_token = zerosum_token.unwrap();
        }
        if trigger_address.is_some() {
            state.trigger_address = trigger_address.unwrap();
        }
        if distributor_contract.is_some() {
            state.distributor_contract = distributor_contract.unwrap();
        }
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "update_state"))
}

pub fn execute_swap(deps: DepsMut, env: Env, info: MessageInfo, asset_info: AssetInfo) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    if state.trigger_address != info.sender && state.gov_contract != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let mut msgs: Vec<CosmosMsg> = vec![];
    let profit: Profit = PROFITS.load(deps.storage, (&asset_info).to_string())?;

    let all_balance = asset_info.query_balance(&deps.querier, env.contract.address)?;
    let swap_asset = Asset {
        info: asset_info.clone(),
        amount: all_balance,
    };

    match asset_info {
        AssetInfo::Token { contract_addr } => {
            if contract_addr == state.zerosum_token {
                return Err(ContractError::Unauthorized{});
            }

            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: profit.swap_contract.to_string(),
                    amount: swap_asset.amount,
                    msg: to_binary(&SwapExecuteMsg::Swap {
                        offer_asset: swap_asset.clone(),
                        belief_price: None,
                        max_spread: None,
                        to: None,
                    })?
                })?
            }));
        },
        AssetInfo::NativeToken { denom } => {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: profit.swap_contract.to_string(),
                funds: vec![Coin::new(swap_asset.amount.u128(), denom)],
                msg: to_binary(&SwapExecuteMsg::Swap {
                    offer_asset: swap_asset.clone(),
                    belief_price: None,
                    max_spread: None,
                    to: None,
                })?
            }));
        },
    }

    let swap_idx = next_swap_index(deps.storage);
    SWAPS.save(deps.storage, swap_idx, &SwapHistory {
        time: env.block.time,
        asset: swap_asset,
    })?;

    // 이거 가지고 스왑하는 메시지 생성.
    Ok(Response::new().add_messages(msgs).add_attribute("method", "execute_swap"))
}

pub fn execute_swap_all(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    if state.trigger_address != info.sender && state.gov_contract != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut swap_histories: Vec<SwapHistory> = vec![];

    PROFITS.range(deps.storage, None, None, Order::Ascending).for_each(|item| {
        let (_, profit): (_, Profit) = item.unwrap();

        let all_balance = profit.asset_info.query_balance(&deps.querier, env.contract.address.clone()).unwrap();
        let swap_asset = Asset {
            info: profit.asset_info.clone(),
            amount: all_balance,
        };

        match profit.asset_info {
            AssetInfo::Token { contract_addr } => {
                if contract_addr != state.zerosum_token {

                    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: contract_addr.to_string(),
                        funds: vec![],
                        msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                            spender: profit.swap_contract.to_string(),
                            amount: all_balance,
                            expires: None,
                        }).unwrap(),
                    }));
                    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: contract_addr.to_string(),
                        funds: vec![],
                        msg: to_binary(&SwapExecuteMsg::Swap {
                            offer_asset: swap_asset.clone(),
                            belief_price: None,
                            max_spread: None,
                            to: None,
                        }).unwrap()
                    }));
                    swap_histories.push(SwapHistory {
                        time: env.block.time,
                        asset: swap_asset,
                    });
                }
            },
            AssetInfo::NativeToken { denom } => {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: profit.swap_contract.to_string(),
                    funds: vec![Coin::new(swap_asset.amount.u128(), denom)],
                    msg: to_binary(&SwapExecuteMsg::Swap {
                        offer_asset: swap_asset.clone(),
                        belief_price: None,
                        max_spread: None,
                        to: None,
                    }).unwrap()
                }));
                swap_histories.push(SwapHistory {
                    time: env.block.time,
                    asset: swap_asset,
                });
            },
        }
    });

    swap_histories.into_iter().for_each(|swap_history| {
        let swap_idx = next_swap_index(deps.storage);
        SWAPS.save(deps.storage, swap_idx, &swap_history).unwrap();
    });

    Ok(Response::new().add_messages(msgs).add_attribute("method", "execute_swap_all"))
}

pub fn execute_add_profit(deps: DepsMut, info: MessageInfo, asset_info: AssetInfo, swap_contract: Option<Addr>) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    let key = asset_info.clone().to_string();
    if state.gov_contract != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    if PROFITS.has(deps.storage, key.clone()) {
        return Err(ContractError::AlreadyExist{});
    }
    PROFITS.save(deps.storage, key.clone(), &Profit {
        asset_info: asset_info,
        swap_contract: swap_contract.unwrap_or(Addr::unchecked("")),
    })?;
    Ok(Response::new()
        .add_attribute("method", "add_profit_asset")
        .add_attribute("asset", key.clone()))
}

pub fn execute_update_profit(deps: DepsMut, info: MessageInfo, asset: AssetInfo, swap_contract: Addr) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    let key = asset.to_string();
    if state.gov_contract != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    if !PROFITS.has(deps.storage, key.clone()) {
        return Err(ContractError::NotExist{});
    }
    PROFITS.update(deps.storage, key.clone(), |result| -> Result<_, ContractError> {
        let mut profit: Profit = result.unwrap();
        profit.swap_contract = swap_contract;
        Ok(profit)
    })?;
    Ok(Response::new()
        .add_attribute("method", "update_profit_asset")
        .add_attribute("asset", key))
}

pub fn execute_add_harvest_contract(deps: DepsMut, info: MessageInfo, addr: Addr) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    if state.gov_contract != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    HARVEST_CONTRACTS.save(deps.storage, addr, &true)?;
    Ok(Response::new()
        .add_attribute("method", "add_harvest_contract"))
}

pub fn execute_remove_harvest_contract(deps: DepsMut, info: MessageInfo, addr: Addr) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    if state.gov_contract != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    HARVEST_CONTRACTS.remove(deps.storage, addr);
    Ok(Response::new()
        .add_attribute("method", "remove_harvest_contract"))
}

pub fn execute_action(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let mut state: State = STATE.load(deps.storage)?;
    if state.gov_contract != info.sender && state.trigger_address != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let mut msgs: Vec<CosmosMsg> = vec![];

    let asset_info = token_asset_info(state.zerosum_token.clone());
    let amount: Uint128 = asset_info.query_balance(&deps.querier, env.contract.address)?;
    if !amount.is_zero() {
        let period = get_period(env.block.height);
        if period.is_some() {
            
            // 쿼리해서 LP Providing 위주로 조져야 할듯하다.
            // let res: Option<u32> = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            //     contract_addr: String::from(random_contract),
            //     msg: to_binary(&RandomQueryMsg::RandomOne {
            //         height: height,
            //         entropy: entropy,
            //         max_value: max_value,
            //     })?,
            // }))?;
            // deps.querier.query_wasm_smart(contract_addr: impl Into<String>, msg: &impl Serialize)

            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: state.zerosum_token.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: amount,
                }).unwrap()
            }));
            state.total_burn_amount = state.total_burn_amount.checked_add(amount).unwrap();
        } else {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: state.zerosum_token.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: state.distributor_contract.to_string(),
                    amount: amount,
                }).unwrap()
            }));
            state.total_distribute_amount = state.total_distribute_amount.checked_add(amount).unwrap();
        }
        STATE.save(deps.storage, &state)?;
        let action_idx = next_action_index(deps.storage);
        ACTIONS.save(deps.storage, action_idx, &ActionHistory {
            time: env.block.time,
            amount: amount,
        })?;
    }
    Ok(Response::new().add_messages(msgs).add_attribute("method", "action"))
}

pub fn execute_collect(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    if state.gov_contract != info.sender && state.trigger_address != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let mut msgs: Vec<CosmosMsg> = vec![];

    HARVEST_CONTRACTS.range(deps.storage, None, None, Order::Ascending).for_each(|contract| {
        let (harvest_address, _) = contract.unwrap();
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: harvest_address.to_string(),
            funds: vec![],
            msg: to_binary(&CollectorMsg::Collect {}).unwrap(),
        }))
    });
    Ok(Response::new().add_messages(msgs).add_attribute("method", "collect"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::State {} => to_binary(&query_state(deps)?),
        QueryMsg::Profit { asset } => to_binary(&query_profit(deps, &env, asset)?),
        QueryMsg::Profits {} => to_binary(&query_profits(deps, env)?),
        QueryMsg::GetSwaps { start_after, limit } => to_binary(&query_swaps(deps, env, start_after, limit)?),
        QueryMsg::GetActions { start_after, limit } => to_binary(&query_actions(deps, env, start_after, limit)?),
    }
}

fn query_state(deps: Deps) -> StdResult<State> {
    let state = STATE.load(deps.storage)?;
    Ok(state)
}

fn query_profit(deps: Deps, env: &Env, asset: AssetInfo) -> StdResult<ProfitResponse> {
    let profit: Profit = PROFITS.load(deps.storage, asset.to_string())?;
    let res = ProfitResponse {
        asset: Asset {
            info: profit.asset_info.clone(),
            amount: profit.asset_info.query_balance(&deps.querier, env.contract.address.clone())?,
        },
        swap_contract: profit.swap_contract,
    };
    Ok(res)
}

fn query_profits(deps: Deps, env: Env) -> StdResult<Vec<ProfitResponse>> {
    let res: Vec<ProfitResponse> = PROFITS.range(deps.storage, None, None, Order::Ascending).map(|result| {
        let (_, profit) = result.unwrap();
        query_profit(deps, &env, profit.asset_info).unwrap()
    }).collect();
    Ok(res)
}

fn query_actions(deps: Deps, _env: Env, start_after: Option<u64>, limit: Option<u64>) -> StdResult<Vec<ActionHistory>> {
    let start = if start_after.is_some() {
        Some(Bound::ExclusiveRaw(start_after.unwrap().to_be_bytes().to_vec()))
    } else {
        None
    };
    let action_histories = ACTIONS.range(deps.storage, start, None, Order::Ascending).take(limit.unwrap_or(20u64) as usize).map(|item| {
        let (_, action_history) = item.unwrap();
        action_history
    }).collect();
    Ok(action_histories)
}

fn query_swaps(deps: Deps, _env: Env, start_after: Option<u64>, limit: Option<u64>) -> StdResult<Vec<SwapHistory>> {
    let start = if start_after.is_some() {
        Some(Bound::ExclusiveRaw(start_after.unwrap().to_be_bytes().to_vec()))
    } else {
        None
    };
    let swap_histories = SWAPS.range(deps.storage, start, None, Order::Ascending).take(limit.unwrap_or(20u64) as usize).map(|item| {
        let (_, swap_history) = item.unwrap();
        swap_history
    }).collect();
    Ok(swap_histories)
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
//     use cosmwasm_std::{coins, from_binary};

//     #[test]
//     fn proper_initialization() {
//         let mut deps = mock_dependencies(&[]);

//         let msg = InstantiateMsg { count: 17 };
//         let info = mock_info("creator", &coins(1000, "earth"));

//         // we can just call .unwrap() to assert this was a success
//         let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
//         assert_eq!(0, res.messages.len());

//         // it worked, let's query the state
//         let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
//         let value: CountResponse = from_binary(&res).unwrap();
//         assert_eq!(17, value.count);
//     }

//     #[test]
//     fn increment() {
//         let mut deps = mock_dependencies(&coins(2, "token"));

//         let msg = InstantiateMsg { count: 17 };
//         let info = mock_info("creator", &coins(2, "token"));
//         let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

//         // beneficiary can release it
//         let info = mock_info("anyone", &coins(2, "token"));
//         let msg = ExecuteMsg::Increment {};
//         let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//         // should increase counter by 1
//         let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
//         let value: CountResponse = from_binary(&res).unwrap();
//         assert_eq!(18, value.count);
//     }

//     #[test]
//     fn reset() {
//         let mut deps = mock_dependencies(&coins(2, "token"));

//         let msg = InstantiateMsg { count: 17 };
//         let info = mock_info("creator", &coins(2, "token"));
//         let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

//         // beneficiary can release it
//         let unauth_info = mock_info("anyone", &coins(2, "token"));
//         let msg = ExecuteMsg::Reset { count: 5 };
//         let res = execute(deps.as_mut(), mock_env(), unauth_info, msg);
//         match res {
//             Err(ContractError::Unauthorized {}) => {}
//             _ => panic!("Must return unauthorized error"),
//         }

//         // only the original creator can reset the counter
//         let auth_info = mock_info("creator", &coins(2, "token"));
//         let msg = ExecuteMsg::Reset { count: 5 };
//         let _res = execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();

//         // should now be 5
//         let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
//         let value: CountResponse = from_binary(&res).unwrap();
//         assert_eq!(5, value.count);
//     }
// }
