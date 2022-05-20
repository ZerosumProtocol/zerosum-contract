#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Addr, Uint128, CosmosMsg, WasmMsg, Order, Decimal};
use cw2::set_contract_version;
use zerosum::asset::{token_asset_info};
use zerosum::round::{Cw20HookMsg as RoundHookMsg, get_round, get_period};
use cw_storage_plus::{Bound};
use cw20::{Cw20ExecuteMsg};


use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, DistributionDetail};
use crate::state::{State, STATE, DistributionAmount, HISTORY, DistributionHistory, LAST_ROUND};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:distributor";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const VESTING_CONTRACT: &str = "asdsadad";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        gov_contract: info.sender,
        zerosum_token: msg.zerosum_token.unwrap_or(Addr::unchecked("")),
        trigger_address: msg.trigger_address.unwrap_or(Addr::unchecked("")),
        distribute_amount: msg.distribute_amount.unwrap_or_default(),
        distributions: msg.distributions.unwrap_or_default(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;
    LAST_ROUND.save(deps.storage, &0u64)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateState {
            gov_contract,
            zerosum_token,
            trigger_address,
            distribute_amount,
            distributions
        } => execute_update_state(
                deps, 
                info, 
                gov_contract,
                zerosum_token,
                trigger_address,
                distribute_amount,
                distributions
            ),
        ExecuteMsg::Distribute { round } => execute_distribute(deps, env, info, round),
    }
}

pub fn execute_update_state(
    deps: DepsMut,
    info: MessageInfo,
    gov_contract: Option<Addr>,
    zerosum_token: Option<Addr>,
    trigger_address: Option<Addr>,
    distribute_amount: Option<Uint128>,
    distributions: Option<Vec<DistributionDetail>>
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
        if trigger_address.is_some() {
            state.trigger_address = trigger_address.unwrap();
        }
        if distribute_amount.is_some() {
            state.distribute_amount = distribute_amount.unwrap();
        }
        if distributions.is_some() {
            state.distributions = distributions.unwrap();
        }
        Ok(state)
    })?;

    Ok(Response::new().add_attribute("method", "update_state"))
}
pub fn execute_distribute(deps: DepsMut, env: Env, info: MessageInfo, round: u64) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    if state.trigger_address != info.sender && state.gov_contract != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let last_round = LAST_ROUND.load(deps.storage)?;
    if last_round >= round {
        return Err(ContractError::AreadyExist {});
    }
    let mut msgs: Vec<CosmosMsg> = vec![];
    let current_period = get_period(round);
    let mut distribute_amount;
    if current_period.is_some() {
        let total_distributor_amount = current_period.unwrap().distribute_amount_per_round;
        distribute_amount = total_distributor_amount * Decimal::percent(80);
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute{
            contract_addr: state.zerosum_token.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                amount: distribute_amount,
                recipient: env.contract.address.to_string(),
            }).unwrap()
        }));
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: state.zerosum_token.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: VESTING_CONTRACT.to_string(),
                amount: total_distributor_amount * Decimal::percent(20),
                msg: to_binary(&RoundHookMsg::AddRound {
                    key: None,
                    round: round,
                }).unwrap()
            }).unwrap()
        }));
    } else {
        distribute_amount = token_asset_info(state.zerosum_token.clone()).query_balance(&deps.querier, env.contract.address)?;
    }
    let mut history: Vec<DistributionAmount> = vec![];
    let contract_addr = state.zerosum_token.to_string();

    state.distributions.into_iter().for_each(|d : DistributionDetail| {
        let amount = distribute_amount * d.ratio;
        // token_asset(state.zerosum_token, distribute_amount).into_msg(&deps.querier, d.address).unwrap()
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.clone(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: d.address.to_string(),
                amount: amount,
                msg: to_binary(&RoundHookMsg::AddRound {
                    key: d.key.clone(),
                    round: round,
                }).unwrap()
            }).unwrap()
        }));
        history.push(DistributionAmount {
            key: d.key,
            address: d.address,
            amount: amount,
        })
    });
    HISTORY.update(deps.storage, round, |prev| {
        match prev {
            Some(_) => Err(ContractError::AreadyExist {}),
            None => Ok(DistributionHistory {
                round: round,
                distribute_amount: distribute_amount,
                distributions: history,
            })
        }
    })?;
    LAST_ROUND.save(deps.storage, &round)?;
    Ok(Response::new().add_messages(msgs).add_attribute("method", "reset"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetState {} => to_binary(&query_state(deps)?),
        QueryMsg::GetRound {} => to_binary(&query_round(env)?),
        QueryMsg::GetLastRound {} => to_binary(&query_last_round(deps)?),
        QueryMsg::GetDistributeHistory { round } => to_binary(&query_distribute_history(deps, round)?),
        QueryMsg::GetDistributeHistories { start_round, limit } => to_binary(&query_distribute_histories(deps, start_round, limit)?),
    }
}

fn query_state(deps: Deps) -> StdResult<State> {
    let state = STATE.load(deps.storage)?;
    Ok(state)
}

fn query_last_round(deps: Deps) -> StdResult<u64> {
    Ok(LAST_ROUND.load(deps.storage)?)
}

fn query_round(env: Env) -> StdResult<Option<u64>> {
    Ok(get_round(env.block.height))
}

fn query_distribute_history(deps: Deps, round: u64) -> StdResult<DistributionHistory> {
    let history = HISTORY.load(deps.storage, round)?;
    Ok(history)
}

fn query_distribute_histories(deps: Deps, start_round: Option<u64>, limit: Option<u64>) -> StdResult<Vec<DistributionHistory>> {
    let start = Some(Bound::InclusiveRaw(start_round.unwrap_or_default().to_be_bytes().to_vec()));
    let count = limit.unwrap_or(20).min(20);
    let histories: StdResult<Vec<DistributionHistory>> = 
        HISTORY.range(deps.storage, start, None, Order::Ascending).take(count as usize).map(|item| {
            let (_key, history) = item.unwrap(); 
            Ok(history)
        }).collect();
    Ok(histories.unwrap())
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
