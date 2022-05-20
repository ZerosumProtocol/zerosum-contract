#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, StdError, Addr, Uint128, Order};
use cw2::set_contract_version;
use zerosum::asset::{token_asset_info, token_asset};
use zerosum::reward::{ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::error::ContractError;
use crate::state::{State, STATE, SPENDER};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:reward";
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
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateState { gov_contract, zerosum_token } => execute_update_state(deps, info, gov_contract, zerosum_token), 
        ExecuteMsg::AddSpender { addr } => execute_add_spender(deps, info, addr),
        ExecuteMsg::RemoveSpender { addr } => execute_remove_spender(deps, info, addr),
        ExecuteMsg::Spend { addr, amount } => execute_spend(deps, info, addr, amount),
    }
}

pub fn execute_update_state(deps: DepsMut, info: MessageInfo, gov_contract: Option<Addr>, zerosum_token: Option<Addr>) -> Result<Response, ContractError> {
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
        Ok(state)
    })?;

    Ok(Response::new().add_attribute("method", "update_state"))
}

pub fn execute_add_spender(deps: DepsMut, info: MessageInfo, addr: Addr) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if info.sender != state.gov_contract {
        return Err(ContractError::Unauthorized {});
    }
    SPENDER.save(deps.storage, addr, &true)?;
    Ok(Response::new().add_attribute("method", "add_spender"))
}

pub fn execute_remove_spender(deps: DepsMut, info: MessageInfo, addr: Addr) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if info.sender != state.gov_contract {
        return Err(ContractError::Unauthorized {});
    }
    SPENDER.remove(deps.storage, addr);
    Ok(Response::new().add_attribute("method", "remove_spender"))
}

pub fn execute_spend(deps: DepsMut, info: MessageInfo, addr: Addr, amount: Uint128) -> Result<Response, ContractError> {
    let spender = SPENDER.may_load(deps.storage, info.sender.clone())?;
    if spender.is_none() {
        return Err(ContractError::Unauthorized {});
    }
    let state = STATE.load(deps.storage)?;
    let msg = token_asset(state.zerosum_token, amount).into_msg(&deps.querier, addr)?;
    Ok(Response::new().add_message(msg).add_attribute("method", "spend"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetSpenders {} => to_binary(&query_spenders(deps)?),
        QueryMsg::GetBalance {} => to_binary(&query_balance(deps, env)?),
    }
}

fn query_spenders(deps: Deps) -> StdResult<Vec<Addr>> {
    let spenders: Vec<Addr> = SPENDER.range(deps.storage, None, None, Order::Ascending).map(|spender| {
        let (harvest_address, _) = spender.unwrap();
        // let harvest_address = deps.api.addr_validate(String::from_utf8(key).unwrap().as_str()).unwrap();
        harvest_address
    }).collect();
    Ok(spenders)
}

fn query_balance(deps: Deps, env: Env) -> StdResult<Uint128> {
    let state = STATE.load(deps.storage)?;
    token_asset_info(state.zerosum_token).query_balance(&deps.querier, env.contract.address)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg { zerosum_token: None };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetSpenders {}).unwrap();
        let value: Vec<Addr> = from_binary(&res).unwrap();
        assert_eq!(vec![] as Vec<Addr>, value);

        let msg = ExecuteMsg::AddSpender { addr: Addr::unchecked("creator") };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);

        // let res = query(deps.as_ref(), mock_env(), QueryMsg::GetSpenders {}).unwrap();
        // let value: Vec<Addr> = from_binary(&res).unwrap();
        // assert_eq!(vec![] as Vec<Addr>, value);

        let info = mock_info("terra1dcegyrekltswvyy0xy69ydgxn9x8x32zdtapd8", &coins(1000, "earth"));
        let msg = ExecuteMsg::Spend { addr: Addr::unchecked("test"), amount: Uint128::from(100u64) };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        // match res {
        //     Err(_) => {},
        //     _ => { panic!("Must return error") }
        // }
    }
}
