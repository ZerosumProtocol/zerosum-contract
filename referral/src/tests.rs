#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, from_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Addr, Uint128, Order, Decimal, CosmosMsg, WasmMsg, Timestamp};
use cw2::set_contract_version;
use cw20::{Cw20ReceiveMsg, Cw20ExecuteMsg};
use cw_storage_plus::{Bound};

use zerosum::referral::{ExecuteMsg, InstantiateMsg, QueryMsg, Cw20HookMsg, RoundShareInfo, UserAmountInfo};
use zerosum::round::{get_round};

use crate::error::ContractError;
use crate::state::{State, STATE, RoundInfo, Referral, REFERRALS, REWARD_SHARE, REFERRAL_HISTORY, 
    REWARDS, ROUNDS, LAST_CLAIM_ROUND, FOLLOWING, FOLLOWERS, FOLLOWER_IDX};

use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coins};

use crate::contract::{execute, query, instantiate};

const CREATOR: &str = "creator";
const ZEROSUM_TOKEN: &str = "zerosum_token";
const HOUSE: &str = "house";
const DISTRIBUTOR: &str = "distributor";
const COLLECTOR: &str = "collector";


fn mock_instantiate(deps: DepsMut) {
    let msg = InstantiateMsg {
        zerosum_token: Some(Addr::unchecked(ZEROSUM_TOKEN)),
        house_coutract: Some(Addr::unchecked(HOUSE)),
        distributor_contract: Some(Addr::unchecked(DISTRIBUTOR)),
        register_referrer_fee: Some(Uint128::from(100000u64)),
        referral_ratio: Some(vec![Decimal::percent(75), Decimal::percent(5), Decimal::percent(8), Decimal::percent(5), Decimal::percent(3), Decimal::percent(2), Decimal::percent(1)]),
        collector_contract: Some(Addr::unchecked(COLLECTOR)),
        reward_contract: None,
    };

    let info = mock_info(CREATOR, &[]);
    let _res = instantiate(deps, mock_env(), info, msg)
        .expect("contract successfully handles InstantiateMsg");
}

fn mock_register_referral(deps: DepsMut, address: &str) {
    let info = mock_info(ZEROSUM_TOKEN, &vec![]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        amount: Uint128::from(10000000u64),
        sender: address.to_string(),
        msg: to_binary(&Cw20HookMsg::RegisterReferral {
            addr: None,
            name: Some("상위".to_string()),
            description: Some("상위 레퍼럴".to_string()),
        }).unwrap()
    });
    let _res = execute(deps, mock_env(), info.clone(), msg.clone()).unwrap();
}

fn mock_following(deps: DepsMut, me: &str, target: &str) {
    let info = mock_info(me, &vec![]);
    let msg = ExecuteMsg::AddFollowing {
        address: Addr::unchecked(target)
    };
    let _res = execute(deps, mock_env(), info.clone(), msg).unwrap();
}

fn mock_add_share(deps: DepsMut, height: u64, target: &str, amount: u64) {
    let info = mock_info(HOUSE, &vec![]);
    let msg = ExecuteMsg::AddShare {
        address: Addr::unchecked(target),
        amount: Uint128::from(amount),
    };
    let _res = execute(deps, mock_env_height(height), info.clone(), msg.clone()).unwrap();
}

fn mock_add_round(deps: DepsMut, round: u64, amount: u64) {
    let info = mock_info(ZEROSUM_TOKEN, &vec![]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        amount: Uint128::from(amount),
        sender: DISTRIBUTOR.to_string(),
        msg: to_binary(&Cw20HookMsg::AddRound {
            round: round,
        }).unwrap()
    });
    let res = execute(deps, mock_env(), info.clone(), msg.clone()).unwrap();
}

fn mock_env_height(height: u64) -> Env {
    let mut env = mock_env();
    env.block.height = height;
    env
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();
    mock_instantiate(deps.as_mut());

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetState {}).unwrap();
    let state: State = from_binary(&res).unwrap();
    
    assert_eq!(
        state,
        State {
            gov_contract: Addr::unchecked(CREATOR),
            zerosum_token: Addr::unchecked(ZEROSUM_TOKEN),
            house_coutract: Addr::unchecked(HOUSE),
            distributor_contract: Addr::unchecked("distributor"),
            register_referrer_fee: Uint128::from(100000u64),
            referral_ratio: vec![Decimal::percent(75), Decimal::percent(5), Decimal::percent(8), Decimal::percent(5), Decimal::percent(3), Decimal::percent(2), Decimal::percent(1)],
            collector_contract: Addr::unchecked("collector"),
            reward_contract: Addr::unchecked("reward"),
        }
    );
}

#[test]
fn update_state() {
    let mut deps = mock_dependencies();
    mock_instantiate(deps.as_mut());

    let info = mock_info(CREATOR, &vec![]);
    let msg = ExecuteMsg::UpdateState {
        gov_contract: Some(Addr::unchecked("gov_new")),
        zerosum_token: Some(Addr::unchecked("zerosum_new")),
        house_coutract: Some(Addr::unchecked("house_new")),
        distributor_contract: Some(Addr::unchecked("distributor_new")),
        register_referrer_fee: Some(Uint128::from(999999u64)),
        referral_ratio: Some(vec![Decimal::percent(75), Decimal::percent(5), Decimal::percent(8)]),
        collector_contract: Some(Addr::unchecked("collector_new")),
        reward_contract: None,
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetState {}).unwrap();
    let state: State = from_binary(&res).unwrap();
    
    assert_eq!(
        state,
        State {
            gov_contract: Addr::unchecked("gov_new"),
            zerosum_token: Addr::unchecked("zerosum_new"),
            house_coutract: Addr::unchecked("house_new"),
            distributor_contract: Addr::unchecked("distributor_new"),
            register_referrer_fee: Uint128::from(999999u64),
            referral_ratio: vec![Decimal::percent(75), Decimal::percent(5), Decimal::percent(8)],
            collector_contract: Addr::unchecked("collector_new"),
            reward_contract: Addr::unchecked("reward"),
        }
    );

    let info = mock_info(CREATOR, &vec![]);
    let msg = ExecuteMsg::UpdateState {
        gov_contract: None,
        zerosum_token: None,
        house_coutract: None,
        distributor_contract: None,
        register_referrer_fee: Some(Uint128::from(777777u64)),
        referral_ratio: Some(vec![Decimal::percent(60), Decimal::percent(30)]),
        collector_contract: None,
        reward_contract: None,
    };
    let result = execute(deps.as_mut(), mock_env(), info, msg.clone());
    match result {
        Ok(_) => panic!("must be unauth error"),
        Err(_) => {}
    }

    let info = mock_info("gov_new", &vec![]);
    let _result = execute(deps.as_mut(), mock_env(), info, msg);

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetState {}).unwrap();
    let state: State = from_binary(&res).unwrap();
    
    assert_eq!(
        state,
        State {
            gov_contract: Addr::unchecked("gov_new"),
            zerosum_token: Addr::unchecked("zerosum_new"),
            house_coutract: Addr::unchecked("house_new"),
            distributor_contract: Addr::unchecked("distributor_new"),
            register_referrer_fee: Uint128::from(777777u64),
            referral_ratio: vec![Decimal::percent(60), Decimal::percent(30)],
            collector_contract: Addr::unchecked("collector_new"),
            reward_contract: Addr::unchecked("reward"),
        }
    );
}

#[test]
fn register_referral() {
    let mut deps = mock_dependencies();
    mock_instantiate(deps.as_mut());

    // sender가 ZEROSUM_TOKEN이 아닐때
    let info = mock_info(CREATOR, &vec![]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        amount: Uint128::from(1000u64),
        sender: CREATOR.to_string(),
        msg: to_binary(&Cw20HookMsg::RegisterReferral {
            addr: None,
            name: Some("상위".to_string()),
            description: Some("상위 레퍼럴".to_string()),
        }).unwrap()
    });
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone());
    match res {
        Ok(_) => panic!("must be error"),
        Err(_) => {}
    }

    // register fee보다 돈을 더 적게 줬을때
    let info = mock_info(ZEROSUM_TOKEN, &vec![]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        amount: Uint128::from(100u64),
        sender: CREATOR.to_string(),
        msg: to_binary(&Cw20HookMsg::RegisterReferral {
            addr: None,
            name: Some("상위".to_string()),
            description: Some("상위 레퍼럴".to_string()),
        }).unwrap()
    });
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone());
    match res {
        Ok(_) => panic!("must be error"),
        Err(_) => {}
    }

    // 정상적으로 줬을때
    let info = mock_info(ZEROSUM_TOKEN, &vec![]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        amount: Uint128::from(10000000u64),
        sender: CREATOR.to_string(),
        msg: to_binary(&Cw20HookMsg::RegisterReferral {
            addr: None,
            name: Some("상위".to_string()),
            description: Some("상위 레퍼럴".to_string()),
        }).unwrap()
    });
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetReferral { addr: Addr::unchecked(CREATOR) }).unwrap();
    let referral: Referral = from_binary(&res).unwrap();
    
    assert_eq!(
        referral,
        Referral {
            name: "상위".to_string(),
            description: "상위 레퍼럴".to_string(),
        }
    )
}

#[test]
fn add_following() {
    let mut deps = mock_dependencies();
    mock_instantiate(deps.as_mut());

    // 레퍼럴이 아닌 유저를 Follow
    let info = mock_info(CREATOR, &vec![]);
    let msg = ExecuteMsg::AddFollowing {
        address: Addr::unchecked("user1")
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    match res {
        Ok(_) => panic!("must be error"),
        Err(_) => {}
    }

    mock_register_referral(deps.as_mut(), CREATOR);
    mock_register_referral(deps.as_mut(), "USER1");

    // 자기 자신을 Follow
    let msg = ExecuteMsg::AddFollowing {
        address: Addr::unchecked(CREATOR)
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    match res {
        Ok(_) => panic!("must be error"),
        Err(_) => {}
    }

    // 정상적인 사람 팔로잉
    let msg = ExecuteMsg::AddFollowing {
        address: Addr::unchecked("USER1")
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // 팔로잉 한 상태에서 또 팔로잉
    let msg = ExecuteMsg::AddFollowing {
        address: Addr::unchecked("USER2")
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    match res {
        Ok(_) => panic!("must be error"),
        Err(_) => {}
    }

    // 서로 팔로잉
    let info = mock_info("USER1", &vec![]);
    let msg = ExecuteMsg::AddFollowing {
        address: Addr::unchecked(CREATOR)
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    match res {
        Ok(_) => panic!("must be error"),
        Err(_) => {}
    }


    // 7명이서 서로 팔로잉
    mock_register_referral(deps.as_mut(), "USER1");
    mock_register_referral(deps.as_mut(), "USER2");
    mock_register_referral(deps.as_mut(), "USER3");
    mock_register_referral(deps.as_mut(), "USER4");
    mock_register_referral(deps.as_mut(), "USER5");
    mock_register_referral(deps.as_mut(), "USER6");
    mock_register_referral(deps.as_mut(), "USER7");
    mock_following(deps.as_mut(), "USER1", "USER2");
    mock_following(deps.as_mut(), "USER2", "USER3");
    mock_following(deps.as_mut(), "USER3", "USER4");
    mock_following(deps.as_mut(), "USER4", "USER5");
    mock_following(deps.as_mut(), "USER6", "USER7");
    let info = mock_info("USER7", &vec![]);
    let msg = ExecuteMsg::AddFollowing {
        address: Addr::unchecked("USER1")
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    match res {
        Ok(_) => panic!("must be error"),
        Err(_) => {}
    }
}

#[test]
fn add_share() {
    let mut deps = mock_dependencies();
    mock_instantiate(deps.as_mut());

    // sender가 HOUSE가 아닐때
    let info = mock_info(CREATOR, &vec![]);
    let msg = ExecuteMsg::AddShare {
        address: Addr::unchecked(CREATOR),
        amount: Uint128::from(10000u64),
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone());
    match res {
        Ok(_) => panic!("must be error"),
        Err(_) => {}
    }

    // 정상적으로 하우스가 보낼때
    let info = mock_info(HOUSE, &vec![]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetRewardShare { round: None, addr: Addr::unchecked(CREATOR) }).unwrap();
    let res: RoundShareInfo = from_binary(&res).unwrap();
    assert_eq!(res, RoundShareInfo {
        total: Uint128::from(10000u64),
        share: Uint128::from(10000u64),
    });

    mock_add_share(deps.as_mut(), 12_345, "USER4", 458123u64);
    mock_add_share(deps.as_mut(), 12_345, "USER5", 458123u64);

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetRewardShare { round: None, addr: Addr::unchecked(CREATOR) }).unwrap();
    let res: RoundShareInfo = from_binary(&res).unwrap();
    assert_eq!(res, RoundShareInfo {
        total: Uint128::from(926246u64),
        share: Uint128::from(10000u64),
    });
}

#[test]
fn add_round() {
    let mut deps = mock_dependencies();
    mock_instantiate(deps.as_mut());

    // sender가 distributor가 아닐때
    let info = mock_info(CREATOR, &vec![]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        amount: Uint128::from(1000u64),
        sender: CREATOR.to_string(),
        msg: to_binary(&Cw20HookMsg::AddRound {
            round: 20,
        }).unwrap()
    });
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone());
    match res {
        Ok(_) => panic!("must be error"),
        Err(_) => {}
    }

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetReferralReward { addr: Addr::unchecked(COLLECTOR) }).unwrap();
    let res: Uint128 = from_binary(&res).unwrap();
    assert_eq!(res, Uint128::from(0u64));

    // 아무도 그날 배팅하지 않아서 분배비율을 못정할때, 모든금액을 collector에게 넘김
    let info = mock_info(ZEROSUM_TOKEN, &vec![]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        amount: Uint128::from(100000u64),
        sender: DISTRIBUTOR.to_string(),
        msg: to_binary(&Cw20HookMsg::AddRound {
            round: 30,
        }).unwrap()
    });
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetReferralReward { addr: Addr::unchecked(COLLECTOR) }).unwrap();
    let res: Uint128 = from_binary(&res).unwrap();
    assert_eq!(res, Uint128::from(100000u64));


    // 정상적으로 AddRound
    mock_add_share(deps.as_mut(), 20_000, CREATOR, 100000);
    mock_add_share(deps.as_mut(), 20_000, "USER1", 100000);
    mock_add_share(deps.as_mut(), 20_000, "USER2", 100000);


    let info = mock_info(ZEROSUM_TOKEN, &vec![]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        amount: Uint128::from(3000000u64),
        sender: DISTRIBUTOR.to_string(),
        msg: to_binary(&Cw20HookMsg::AddRound {
            round: 19,
        }).unwrap()
    });
    let res = execute(deps.as_mut(), mock_env_height(20_000), info.clone(), msg.clone()).unwrap();

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetRound { round: 19 }).unwrap();
    let res: RoundInfo = from_binary(&res).unwrap();
    assert_eq!(res, RoundInfo {
        reward_ratio: Decimal::percent(1000),
    });
}

#[test]
fn claim() {
    let mut deps = mock_dependencies();
    mock_instantiate(deps.as_mut());

    mock_register_referral(deps.as_mut(), "USER1");
    mock_register_referral(deps.as_mut(), "USER10");
    mock_register_referral(deps.as_mut(), "USER11");
    mock_register_referral(deps.as_mut(), "USER12");
    mock_register_referral(deps.as_mut(), "USER2");
    mock_register_referral(deps.as_mut(), "USER3");
    mock_register_referral(deps.as_mut(), "USER4");
    mock_register_referral(deps.as_mut(), "USER5");
    mock_register_referral(deps.as_mut(), "USER6");
    mock_register_referral(deps.as_mut(), "USER7");
    mock_following(deps.as_mut(), "USER10", "USER1");
    mock_following(deps.as_mut(), "USER11", "USER1");
    mock_following(deps.as_mut(), "USER12", "USER1");
    mock_following(deps.as_mut(), "USER1", "USER2");
    mock_following(deps.as_mut(), "USER2", "USER3");
    mock_following(deps.as_mut(), "USER3", "USER4");
    mock_following(deps.as_mut(), "USER4", "USER5");
    mock_following(deps.as_mut(), "USER5", "USER6");


    mock_add_share(deps.as_mut(), 20_000, "USER10", 200_000);
    mock_add_share(deps.as_mut(), 20_000, "USER11", 100_000);
    mock_add_share(deps.as_mut(), 20_000, "USER12", 100_000);

    mock_add_share(deps.as_mut(), 20_000, "USER1", 100_000);
    mock_add_share(deps.as_mut(), 20_000, "USER2", 100_000);
    mock_add_share(deps.as_mut(), 20_000, "USER3", 100_000);
    mock_add_share(deps.as_mut(), 20_000, "USER4", 100_000);
    mock_add_share(deps.as_mut(), 20_000, "USER5", 100_000);
    mock_add_share(deps.as_mut(), 20_000, "USER6", 100_000);

    mock_add_round(deps.as_mut(), 19, 1_000_000);

    let info = mock_info("USER1", &vec![]);
    let msg = ExecuteMsg::Claim {
        start_round: None,
    };
    let res = execute(deps.as_mut(), mock_env_height(30_000), info.clone(), msg).unwrap();
    
    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetReferralReward { addr: Addr::unchecked("USER1") }).unwrap();
    let res: Uint128 = from_binary(&res).unwrap();
    assert_eq!(res, Uint128::from(5_000u64));

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetReferralReward { addr: Addr::unchecked("USER2") }).unwrap();
    let res: Uint128 = from_binary(&res).unwrap();
    assert_eq!(res, Uint128::from(8_000u64));

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetReferralReward { addr: Addr::unchecked("USER3") }).unwrap();
    let res: Uint128 = from_binary(&res).unwrap();
    assert_eq!(res, Uint128::from(5_000u64));

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetReferralReward { addr: Addr::unchecked("USER4") }).unwrap();
    let res: Uint128 = from_binary(&res).unwrap();
    assert_eq!(res, Uint128::from(3_000u64));

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetReferralReward { addr: Addr::unchecked("USER5") }).unwrap();
    let res: Uint128 = from_binary(&res).unwrap();
    assert_eq!(res, Uint128::from(2_000u64));

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetReferralReward { addr: Addr::unchecked("USER6") }).unwrap();
    let res: Uint128 = from_binary(&res).unwrap();
    assert_eq!(res, Uint128::from(1_000u64));



    let res = query(deps.as_ref(), mock_env_height(30_000), QueryMsg::GetReward { addr: Addr::unchecked("USER10") }).unwrap();
    let res: Uint128 = from_binary(&res).unwrap();
    assert_eq!(res, Uint128::from(150000u64));

    let msg = ExecuteMsg::Claim {
        start_round: None,
    };
    let info = mock_info("USER10", &vec![]);
    let res = execute(deps.as_mut(), mock_env_height(30_000), info.clone(), msg.clone()).unwrap();

    let res = query(deps.as_ref(), mock_env_height(30_000), QueryMsg::GetReward { addr: Addr::unchecked("USER10") }).unwrap();
    let res: Uint128 = from_binary(&res).unwrap();
    assert_eq!(res, Uint128::from(0u64));


    let info = mock_info("USER11", &vec![]);
    let res = execute(deps.as_mut(), mock_env_height(30_000), info.clone(), msg.clone()).unwrap();
    let info = mock_info("USER12", &vec![]);
    let res = execute(deps.as_mut(), mock_env_height(30_000), info.clone(), msg.clone()).unwrap();
    // let info = mock_info("USER1", &vec![]);
    // let res = execute(deps.as_mut(), mock_env_height(30_000), info.clone(), msg.clone()).unwrap();
    let info = mock_info("USER2", &vec![]);
    let res = execute(deps.as_mut(), mock_env_height(30_000), info.clone(), msg.clone()).unwrap();
    
    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetFollowers { reward_addr: Addr::unchecked("USER4"), target_addr: Addr::unchecked("USER4") }).unwrap();
    let res: Vec<UserAmountInfo> = from_binary(&res).unwrap();
    assert_eq!(res, vec![UserAmountInfo { address: Addr::unchecked("USER3"), amount: Uint128::zero() }]);

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetFollowers { reward_addr: Addr::unchecked("USER4"), target_addr: Addr::unchecked("USER3") }).unwrap();
    let res: Vec<UserAmountInfo> = from_binary(&res).unwrap();
    assert_eq!(res, vec![UserAmountInfo { address: Addr::unchecked("USER2"), amount: Uint128::from(5000u64) }]);


    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetFollowers { reward_addr: Addr::unchecked("USER4"), target_addr: Addr::unchecked("USER2") }).unwrap();
    let res: Vec<UserAmountInfo> = from_binary(&res).unwrap();
    assert_eq!(res, vec![UserAmountInfo { address: Addr::unchecked("USER1"), amount: Uint128::from(3000u64) }]);


    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetFollowers { reward_addr: Addr::unchecked("USER4"), target_addr: Addr::unchecked("USER1") }).unwrap();
    let res: Vec<UserAmountInfo> = from_binary(&res).unwrap();
    assert_eq!(res, vec![UserAmountInfo { address: Addr::unchecked("USER10"), amount: Uint128::from(4000u64) }, UserAmountInfo { address: Addr::unchecked("USER11"), amount: Uint128::from(2000u64) }, UserAmountInfo { address: Addr::unchecked("USER12"), amount: Uint128::from(2000u64) }]);


    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetReferralReward { addr: Addr::unchecked("USER1") }).unwrap();
    let res: Uint128 = from_binary(&res).unwrap();
    assert_eq!(res, Uint128::from(37000u64));


    let info = mock_info("USER1", &vec![]);
    let msg = ExecuteMsg::ClaimReferral { };
    let res = execute(deps.as_mut(), mock_env_height(30_000), info.clone(), msg).unwrap();


    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetReferralReward { addr: Addr::unchecked("USER1") }).unwrap();
    let res: Uint128 = from_binary(&res).unwrap();
    assert_eq!(res, Uint128::from(0u64));
}
// #[test]
// fn claim_referral() {

//     let mut deps = mock_dependencies();
//     mock_instantiate(deps.as_mut());

//     mock_register_referral(deps.as_mut(), "USER1");
//     mock_register_referral(deps.as_mut(), "USER2");
//     mock_following(deps.as_mut(), "USER2", "USER1");
//     mock_following(deps.as_mut(), "USER3", "USER1");
//     mock_following(deps.as_mut(), "USER4", "USER1");
//     mock_following(deps.as_mut(), "USER5", "USER4");

//     mock_add_share(deps.as_mut(), 20_000, "USER3", 200_000);
//     mock_add_share(deps.as_mut(), 20_000, "USER4", 100_000);
//     mock_add_share(deps.as_mut(), 20_000, "USER5", 100_000);

//     mock_add_round(deps.as_mut(), 19, 1_000_000);


//     let info = mock_info("USER2", &vec![]);
//     let res = execute(deps.as_mut(), mock_env_height(30_000), info.clone(), msg.clone()).unwrap();

// }
// #[test]
// fn increment() {
//     let mut deps = mock_dependencies(&coins(2, "token"));

//     let msg = InstantiateMsg { count: 17 };
//     let info = mock_info("creator", &coins(2, "token"));
//     let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

//     // beneficiary can release it
//     let info = mock_info("anyone", &coins(2, "token"));
//     let msg = ExecuteMsg::Increment {};
//     let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//     // should increase counter by 1
//     let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
//     let value: CountResponse = from_binary(&res).unwrap();
//     assert_eq!(18, value.count);
// }

// #[test]
// fn reset() {
//     let mut deps = mock_dependencies(&coins(2, "token"));

//     let msg = InstantiateMsg { count: 17 };
//     let info = mock_info("creator", &coins(2, "token"));
//     let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

//     // beneficiary can release it
//     let unauth_info = mock_info("anyone", &coins(2, "token"));
//     let msg = ExecuteMsg::Reset { count: 5 };
//     let res = execute(deps.as_mut(), mock_env(), unauth_info, msg);
//     match res {
//         Err(ContractError::Unauthorized {}) => {}
//         _ => panic!("Must return unauthorized error"),
//     }

//     // only the original creator can reset the counter
//     let auth_info = mock_info("creator", &coins(2, "token"));
//     let msg = ExecuteMsg::Reset { count: 5 };
//     let _res = execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();

//     // should now be 5
//     let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
//     let value: CountResponse = from_binary(&res).unwrap();
//     assert_eq!(5, value.count);
// }
    