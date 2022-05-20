#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Addr, Uint128, CosmosMsg, WasmMsg};
use cw2::set_contract_version;
use cw20::{Cw20ReceiveMsg, Cw20ExecuteMsg};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, Cw20HookMsg};
use crate::state::{State, STATE, Reward, REWARD, REST_REWARD, Vesting};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:vesting";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        owner: info.sender.clone(),
        zerosum_token: msg.zerosum_token.unwrap_or(Addr::unchecked("")),
        distributor_contract: msg.distributor_contract.unwrap_or(Addr::unchecked("")),
        vestings: msg.vestings.unwrap_or_default(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
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
        ExecuteMsg::UpdateState { owner, zerosum_token, distributor_contract, vestings } => execute_update_state(deps, info, owner, zerosum_token, distributor_contract, vestings),
        ExecuteMsg::Claim {} => execute_claim(deps, env, info),
        ExecuteMsg::Send { amount, recipient } => execute_send(deps, env, info, amount, recipient),
    }
}

fn receive_cw20(deps: DepsMut, env: Env, info: MessageInfo, cw20_msg: Cw20ReceiveMsg) -> Result<Response, ContractError> {
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::AddRound { key, round }) => {
            execute_add_round(deps, env, info, round, Addr::unchecked(cw20_msg.sender), cw20_msg.amount)
        },
        Err(err) => Err(ContractError::Std(err)),
    }
}

fn execute_update_state(
    deps: DepsMut, 
    info: MessageInfo,
    owner: Option<Addr>,
    distributor_contract: Option<Addr>,
    zerosum_token: Option<Addr>,
    vestings: Option<Vec<Vesting>>,
) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| {
        if state.owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }
        if owner.is_some() {
            state.owner = owner.unwrap();
        }
        if zerosum_token.is_some() {
            state.zerosum_token = zerosum_token.unwrap();
        }
        if distributor_contract.is_some() {
            state.distributor_contract = distributor_contract.unwrap();
        }
        if vestings.is_some() {
            state.vestings = vestings.unwrap();
        }
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "update_state"))
}

fn execute_add_round(deps: DepsMut, env: Env, info: MessageInfo, round: u64, sender: Addr, amount: Uint128) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if state.zerosum_token != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let mut rest_amount = amount;
    for vesting in state.vestings.iter() {
        let reward_amount = amount * vesting.share;
        rest_amount = rest_amount - reward_amount;
        REWARD.update(deps.storage, vesting.address.clone(), |prev| -> Result<_, ContractError> {
            match prev {
                Some(mut prev_reward) => {
                    prev_reward.total_reward = prev_reward.total_reward + reward_amount;
                    prev_reward.total_reward = prev_reward.total_reward + reward_amount;
                    Ok(prev_reward)
                },
                None => {
                    Ok(Reward {
                        total_reward: reward_amount,
                        claimable_reward: reward_amount,
                    })
                }
            }
        })?;
    }
    if !rest_amount.is_zero() {
        REST_REWARD.update(deps.storage, |prev| -> Result<Uint128, ContractError> {
            Ok(prev + rest_amount)
        })?;
    }
    Ok(Response::new())
}

fn execute_claim(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    let mut msgs: Vec<CosmosMsg> = vec![];
    REWARD.update(deps.storage, info.sender.clone(), |prev| {
        match prev {
            Some(mut prev_reward) => {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: state.zerosum_token.to_string(),
                    funds: vec![],
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        amount: prev_reward.claimable_reward,
                        recipient: info.sender.to_string(),
                    }).unwrap(),
                }));
                prev_reward.claimable_reward = Uint128::zero();
                Ok(prev_reward)
            },
            None => Err(ContractError::Unauthorized {}),
        }
    })?;
    Ok(Response::new().add_messages(msgs).add_attribute("method", "claim"))
}

fn execute_send(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
    recipient: Addr,
) -> Result<Response, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    let mut msgs: Vec<CosmosMsg> = vec![];
    if state.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    REST_REWARD.update(deps.storage, |prev_reward| {
        if prev_reward < amount {
            return Err(ContractError::Unauthorized {});
        }
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: state.zerosum_token.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                amount: amount,
                recipient: recipient.to_string(),
            }).unwrap(),
        }));
        Ok(prev_reward - amount)
    })?;
    Ok(Response::new().add_messages(msgs).add_attribute("method", "send"))
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetState {} => to_binary(&query_state(deps)?),
        QueryMsg::GetReward { addr } => to_binary(&query_reward(deps, addr)?),
        QueryMsg::GetRestReward {} => to_binary(&query_rest_reward(deps)?),
    }
}

fn query_state(deps: Deps) -> StdResult<State> {
    let state = STATE.load(deps.storage)?;
    Ok(state)
}

fn query_reward(deps: Deps, addr: Addr) -> StdResult<Reward> {
    let reward = REWARD.load(deps.storage, addr)?;
    Ok(reward)
}

fn query_rest_reward(deps: Deps) -> StdResult<Uint128> {
    let rest_reward = REST_REWARD.load(deps.storage)?;
    Ok(rest_reward)
}

// fn query_count(deps: Deps) -> StdResult<CountResponse> {
//     let state = STATE.load(deps.storage)?;
//     Ok(CountResponse { count: state.count })
// }

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
