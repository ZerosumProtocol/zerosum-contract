#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Addr, Order, Decimal};
use cw_storage_plus::{Bound};
use cw2::set_contract_version;

use crate::error::ContractError;
use zerosum::random::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{State, STATE, FEEDERS, SEEDS};
use crate::rand::{Prng};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:random";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        gov_contract: info.sender.clone(),
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
        ExecuteMsg::UpdateState { gov_contract } => execute_update_state(deps, info, gov_contract),
        ExecuteMsg::AddFeeder { address } => execute_add_feeder(deps, info, address),
        ExecuteMsg::RemoveFeeder { address } => execute_remove_feeder(deps, info, address),
        ExecuteMsg::Feed { height, seed } => execute_feed(deps, env, info, height, seed)
    }
}

pub fn execute_update_state(deps: DepsMut, info: MessageInfo, address: Addr) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;
    if info.sender != state.gov_contract {
        state.gov_contract = address.clone();
    }
    STATE.save(deps.storage, &state)?;
    Ok(Response::new()
        .add_attribute("method", "update_state")
        .add_attribute("gov_contract", address.to_string()))
}

pub fn execute_add_feeder(deps: DepsMut, info: MessageInfo, address: Addr) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if info.sender != state.gov_contract {
        return Err(ContractError::Unauthorized{});
    }
    FEEDERS.save(deps.storage, address.clone(), &true)?;
    Ok(Response::new()
        .add_attribute("method", "add_feeder")
        .add_attribute("address", address.to_string()))
}

pub fn execute_remove_feeder(deps: DepsMut, info: MessageInfo, address: Addr) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if info.sender != state.gov_contract {
        return Err(ContractError::Unauthorized {});
    }
    FEEDERS.remove(deps.storage, address.clone());
    Ok(Response::new()
        .add_attribute("method", "remove_feeder")
        .add_attribute("address", address.to_string()))
}

pub fn execute_feed(deps: DepsMut, env: Env, info: MessageInfo, height: Option<u64>, seed: String) -> Result<Response, ContractError> {    
    // 이해가 안되네 expect가 왜이렇게 동작하지?
    /////////////////// 안됨
    // let feeder = FEEDERS.may_load(deps.storage, info.sender)?.expect("you are not feeder");

    /////////////////// 됨
    // let feeder = FEEDERS.may_load(deps.storage, info.sender)?; //.expect("you are not feeder");
    // if !feeder.is_some() {
    //     return Err(ContractError::NotFeeder {});
    // }
    let height_key = height.unwrap_or(env.block.height);

    if !FEEDERS.has(deps.storage, info.sender) {
        return Err(ContractError::NotFeeder {})
    }
    if SEEDS.has(deps.storage, height_key) {
        return Err(ContractError::AlreadyExist {})
    }

    SEEDS.save(deps.storage, height_key, &seed)?;
    Ok(Response::new()
        .add_attribute("method", "feed")
        .add_attribute("height", height_key.to_string())
        .add_attribute("seed", seed))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Seed { height } => to_binary(&query_seed(deps, height)?),
        QueryMsg::RandomOne { height, entropy, max_value } => to_binary(&query_random(deps, height, entropy, 0, max_value)?),
        QueryMsg::RandomBetween { height, entropy, min_value, max_value } => to_binary(&query_random(deps, height, entropy, min_value, max_value)?),
    }
}

fn query_seed(deps: Deps, height: u64) -> StdResult<Option<String>> {
    // Bound::exclusive_int(height);
    let start = Some(Bound::InclusiveRaw(height.to_be_bytes().to_vec()));
    let seed: String = SEEDS.range(deps.storage, start, None, Order::Ascending).take(1).map(|seed| {
        let (_, value) = seed.unwrap();
        value
    }).collect();
    if seed == "" {
        return Ok(None);
    }
    Ok(Some(seed))
}

fn query_random(deps: Deps, height: u64, entropy: Option<Vec<u8>>, min_value: u32, max_value: u32) -> StdResult<Option<u32>> {
    let start = Some(Bound::InclusiveRaw(height.to_be_bytes().to_vec()));
    let seed: String = SEEDS.range(deps.storage, start, None, Order::Ascending).take(1).map(|seed| {
        let (_, value) = seed.unwrap();
        value
    }).collect();
    if seed == String::default() {
        return Ok(None);
    }
    let seed = seed.as_bytes();
    let mut entropy_vec = height.to_be_bytes().to_vec();
    entropy_vec.extend(entropy.unwrap_or_default());
    let mut rng: Prng = Prng::new(&seed, &entropy_vec.as_slice());
    Ok(Some(rng.random_between(min_value, max_value)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, BlockInfo, ContractInfo, Timestamp};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn add_feeder() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &vec![]);
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("anyone", &vec![]);
        let msg = ExecuteMsg::AddFeeder { address: Addr::unchecked("feeder") };
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        match res {
            Err(_) => {}
            _ => panic!("Must return error")
        }

        // beneficiary can release it
        let info = mock_info("creator", &vec![]);
        let msg = ExecuteMsg::AddFeeder { address: Addr::unchecked("feeder") };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    #[test]
    fn remove_feeder() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &vec![]);
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("creator", &vec![]);
        let msg = ExecuteMsg::AddFeeder { address: Addr::unchecked("feeder") };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("anyone", &vec![]);
        let msg = ExecuteMsg::RemoveFeeder { address: Addr::unchecked("creator") };
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        match res {
            Err(_) => {}
            _ => panic!("Must return error")
        }

        let info = mock_info("creator", &vec![]);
        let msg = ExecuteMsg::RemoveFeeder { address: Addr::unchecked("feeder") };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    #[test]
    fn feed() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &vec![]);
        let _res = instantiate(deps.as_mut(), mock_env_height(0), info, msg).unwrap();

        let info = mock_info("creator", &vec![]);
        let msg = ExecuteMsg::AddFeeder { address: Addr::unchecked("feeder") };
        let _res = execute(deps.as_mut(), mock_env_height(0), info, msg).unwrap();

        let info = mock_info("anyone", &vec![]);
        let msg = ExecuteMsg::Feed { height: None, seed: String::from("0xSEED___________00___________") };
        let res = execute(deps.as_mut(), mock_env_height(0), info, msg);
        match res {
            Err(_) => {}
            _ => panic!("Must return error")
        }

        let info = mock_info("feeder", &vec![]);
        let msg = ExecuteMsg::Feed { height: None, seed: String::from("0xSEED___________00___________") };
        let _res = execute(deps.as_mut(), mock_env_height(0), info, msg).unwrap();


        let info = mock_info("feeder", &vec![]);
        let msg = ExecuteMsg::Feed { height: None, seed: String::from("0xSEED___________10___________") };
        let res = execute(deps.as_mut(), mock_env_height(0), info, msg);
        match res {
            Err(_) => {}
            _ => panic!("Must return error")
        }
    }


    #[test]
    fn seed() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &vec![]);
        let _res = instantiate(deps.as_mut(), mock_env_height(0), info, msg).unwrap();

        let info = mock_info("creator", &vec![]);
        let msg = ExecuteMsg::AddFeeder { address: Addr::unchecked("feeder") };
        let _res = execute(deps.as_mut(), mock_env_height(0), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("feeder", &vec![]);
        let msg = ExecuteMsg::Feed { height: None, seed: String::from("0xSEED___________00___________") };
        let _res = execute(deps.as_mut(), mock_env_height(0), info, msg).unwrap();

        let info = mock_info("feeder", &vec![]);
        let msg = ExecuteMsg::Feed { height: None, seed: String::from("0xSEED___________01___________") };
        let _res = execute(deps.as_mut(), mock_env_height(1), info, msg).unwrap();

        let info = mock_info("feeder", &vec![]);
        let msg = ExecuteMsg::Feed { height: None, seed: String::from("0xSEED___________02___________") };
        let _res = execute(deps.as_mut(), mock_env_height(2), info, msg).unwrap();

        let info = mock_info("feeder", &vec![]);
        let msg = ExecuteMsg::Feed { height: None, seed: String::from("0xSEED___________10___________") };
        let _res = execute(deps.as_mut(), mock_env_height(10), info, msg).unwrap();


        let res = query(deps.as_ref(), mock_env_height(2), QueryMsg::Seed { height: 0 }).unwrap();
        let value: String = from_binary(&res).unwrap();
        assert_eq!("0xSEED___________01___________", value);


        let res = query(deps.as_ref(), mock_env_height(2), QueryMsg::Seed { height: 1 }).unwrap();
        let value: String = from_binary(&res).unwrap();
        assert_eq!("0xSEED___________02___________", value);

        let res = query(deps.as_ref(), mock_env_height(2), QueryMsg::Seed { height: 7 }).unwrap();
        let value: String = from_binary(&res).unwrap();
        assert_eq!("0xSEED___________10___________", value);

        let res = query(deps.as_ref(), mock_env_height(2), QueryMsg::Seed { height: 100 }).unwrap();
        let value: String = from_binary(&res).unwrap_or("NOT".to_string());
        assert_eq!("NOT".to_string(), value);
    }

    #[test]
    fn random() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &vec![]);
        let _res = instantiate(deps.as_mut(), mock_env_height(0), info, msg).unwrap();

        let info = mock_info("creator", &vec![]);
        let msg = ExecuteMsg::AddFeeder { address: Addr::unchecked("feeder") };
        let _res = execute(deps.as_mut(), mock_env_height(0), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("feeder", &vec![]);
        let msg = ExecuteMsg::Feed { height: None, seed: String::from("0xSEED___________00___________") };
        let _res = execute(deps.as_mut(), mock_env_height(0), info, msg).unwrap();

        let info = mock_info("feeder", &vec![]);
        let msg = ExecuteMsg::Feed { height: None, seed: String::from("0xSEED___________01___________") };
        let _res = execute(deps.as_mut(), mock_env_height(1), info, msg).unwrap();

        let info = mock_info("feeder", &vec![]);
        let msg = ExecuteMsg::Feed { height: None, seed: String::from("0xSEED___________02___________") };
        let _res = execute(deps.as_mut(), mock_env_height(2), info, msg).unwrap();

        let info = mock_info("feeder", &vec![]);
        let msg = ExecuteMsg::Feed { height: None, seed: String::from("0xSEED___________10___________") };
        let _res = execute(deps.as_mut(), mock_env_height(10), info, msg).unwrap();


        let res = query(deps.as_ref(), mock_env_height(1), QueryMsg::RandomOne { height: 0, max_value: 99, entropy: None }).unwrap();
        let value: u32 = from_binary(&res).unwrap();
        assert_eq!(23u32, value);

        let res = query(deps.as_ref(), mock_env_height(2), QueryMsg::RandomOne { height: 1, max_value: 99, entropy: None }).unwrap();
        let value: u32 = from_binary(&res).unwrap();
        assert_eq!(26u32, value);

        let res = query(deps.as_ref(), mock_env_height(3), QueryMsg::RandomOne { height: 2, max_value: 99, entropy: None }).unwrap();
        let value: u32 = from_binary(&res).unwrap();
        assert_eq!(55u32, value);
        
        let res = query(deps.as_ref(), mock_env_height(4), QueryMsg::RandomOne { height: 4, max_value: 99, entropy: None }).unwrap();
        let value: u32 = from_binary(&res).unwrap();
        assert_eq!(83u32, value);

        let res = query(deps.as_ref(), mock_env_height(5), QueryMsg::RandomOne { height: 5, max_value: 99, entropy: None }).unwrap();
        let value: u32 = from_binary(&res).unwrap();
        assert_eq!(83u32, value);

        let res = query(deps.as_ref(), mock_env_height(6), QueryMsg::RandomOne { height: 6, max_value: 99, entropy: None }).unwrap();
        let value: u32 = from_binary(&res).unwrap();
        assert_eq!(67u32, value);

        let res = query(deps.as_ref(), mock_env_height(6), QueryMsg::RandomOne { height: 100, max_value: 99, entropy: None }).unwrap();
        let value: u32 = from_binary(&res).unwrap_or(1000u32);
        assert_eq!(1000u32, value);

        let res = query(deps.as_ref(), mock_env_height(6), QueryMsg::RandomOne { height: 300, max_value: 99, entropy: None }).unwrap();
        let value: u32 = from_binary(&res).unwrap_or(1000u32);
        assert_eq!(1000u32, value);

        let res = query(deps.as_ref(), mock_env_height(6), QueryMsg::RandomOne { height: 400, max_value: 99, entropy: None }).unwrap();
        let value: u32 = from_binary(&res).unwrap_or(1000u32);
        assert_eq!(1000u32, value);
    }

    pub fn mock_env_height(height: u64) -> Env {
        Env {
            block: BlockInfo {
                height: height,
                time: Timestamp::from_nanos(1_571_797_419_879_305_533),
                chain_id: "cosmos-testnet-14002".to_string(),
            },
            contract: ContractInfo {
                address: Addr::unchecked("contract"),
            },
            transaction: None,
        }
    }
}
