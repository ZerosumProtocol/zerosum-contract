use crate::asset::{AssetInfo};
use crate::random::{QueryMsg as RandomQueryMsg};
use crate::terraswap::{PoolResponse, QueryMsg as TerraswapQueryMsg};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{
    to_binary, Addr, AllBalanceResponse, BalanceResponse, BankQuery, Coin, QuerierWrapper,
    QueryRequest, StdResult, Uint128, WasmQuery, Decimal, CustomQuery,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, TokenInfoResponse};

const NATIVE_TOKEN_PRECISION: u8 = 6;

/// TerraQueryWrapper is an override of QueryRequest::Custom to access Terra-specific modules
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TendermintQueryWrapper {
    pub route: TendermintRoute,
    pub query_data: TendermintQuery,
}

// implement custom query
impl CustomQuery for TendermintQueryWrapper {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TendermintQuery {
    BlockInfo {
        height: u64,
    },
    TaxRate {},
}

/// TerraRoute is enum type to represent terra query route path
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TendermintRoute {
    Tendermint,
    Oracle,
}

pub fn query_balance(
    querier: &QuerierWrapper,
    account_addr: Addr,
    denom: String,
) -> StdResult<Uint128> {
    let balance: BalanceResponse = querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: String::from(account_addr),
        denom,
    }))?;
    Ok(balance.amount.amount)
}

/// ## Description
/// Returns the total balance for all coins at the specified account address.
/// ## Params
/// * **querier** is the object of type [`QuerierWrapper`].
///
/// * **account_addr** is the object of type [`Addr`].
pub fn query_all_balances(querier: &QuerierWrapper, account_addr: Addr) -> StdResult<Vec<Coin>> {
    let all_balances: AllBalanceResponse =
        querier.query(&QueryRequest::Bank(BankQuery::AllBalances {
            address: String::from(account_addr),
        }))?;
    Ok(all_balances.amount)
}

/// ## Description
/// Returns the token balance at the specified contract address.
/// ## Params
/// * **querier** is the object of type [`QuerierWrapper`].
///
/// * **contract_addr** is the object of type [`Addr`]. Sets the address of the contract for which
/// the balance will be requested
///
/// * **account_addr** is the object of type [`Addr`].
pub fn query_token_balance(
    querier: &QuerierWrapper,
    contract_addr: Addr,
    account_addr: Addr,
) -> StdResult<Uint128> {
    // load balance from the token contract
    let res: Cw20BalanceResponse = querier
        .query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: String::from(contract_addr),
            msg: to_binary(&Cw20QueryMsg::Balance {
                address: String::from(account_addr),
            })?,
        }))
        .unwrap_or_else(|_| Cw20BalanceResponse {
            balance: Uint128::zero(),
        });

    Ok(res.balance)
}

/// ## Description
/// Returns the token symbol at the specified contract address.
/// ## Params
/// * **querier** is the object of type [`QuerierWrapper`].
///
/// * **contract_addr** is the object of type [`Addr`].
pub fn query_token_symbol(querier: &QuerierWrapper, contract_addr: Addr) -> StdResult<String> {
    let res: TokenInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: String::from(contract_addr),
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    }))?;

    Ok(res.symbol)
}

/// ## Description
/// Returns the total supply at the specified contract address.
/// ## Params
/// * **querier** is the object of type [`QuerierWrapper`].
///
/// * **contract_addr** is the object of type [`Addr`].
pub fn query_supply(querier: &QuerierWrapper, contract_addr: Addr) -> StdResult<Uint128> {
    let res: TokenInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: String::from(contract_addr),
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    }))?;

    Ok(res.total_supply)
}

/// ## Description
/// Returns the token precision at the specified asset of type [`AssetInfo`].
/// ## Params
/// * **querier** is the object of type [`QuerierWrapper`].
///
/// * **asset_info** is the object of type [`AssetInfo`].
pub fn query_token_precision(querier: &QuerierWrapper, asset_info: AssetInfo) -> StdResult<u8> {
    Ok(match asset_info {
        AssetInfo::NativeToken { denom: _ } => NATIVE_TOKEN_PRECISION,
        AssetInfo::Token { contract_addr } => {
            let res: TokenInfoResponse =
                querier.query_wasm_smart(contract_addr, &Cw20QueryMsg::TokenInfo {})?;

            res.decimals
        }
    })
}

pub fn query_random(
    querier: &QuerierWrapper, 
    random_contract: Addr, 
    height: u64, 
    entropy: Option<Vec<u8>>, 
    max_value: u32
) -> StdResult<Option<u32>> {
    let res: Option<u32> = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: String::from(random_contract),
        msg: to_binary(&RandomQueryMsg::RandomOne {
            height: height,
            entropy: entropy,
            max_value: max_value,
        })?,
    }))?;
    Ok(res)
}

pub fn token_to_ust(
    querier: &QuerierWrapper,
    swap_contract: Addr,
    amount: Uint128,
) -> StdResult<Uint128> {
    let response: PoolResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: swap_contract.to_string(),
        msg: to_binary(&TerraswapQueryMsg::Pool {})?,
    }))?;
    let weight = Decimal::from_ratio(response.assets[1].amount, response.assets[0].amount);
    Ok(amount * weight)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct BlockInfoResposne {
    pub block_id: Block,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Block {
    pub hash: String,
}
