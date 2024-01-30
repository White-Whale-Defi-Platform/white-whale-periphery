use cosmwasm_std::{
    coin, to_json_binary, Addr, Coin, CosmosMsg, Decimal, DepsMut, QueryRequest, Response,
    StdError, SubMsg, Uint128, WasmMsg, WasmQuery,
};
use white_whale_std::pool_network::asset::{Asset, AssetInfo, PairInfo};

use crate::contract::ASSERT_MINIMUM_RECEIVE_REPLY_ID;
use crate::msg::{
    MinimumReceiveAssertion, SwapExactAmountInResponseData, SwapExactAmountOutResponseData,
};
use crate::queries::{calc_in_amt_given_out, calc_out_amt_given_in};
use crate::state::{CONFIG, TEMP_MIN_ASSERTION_DATA};
use crate::ContractError;

/// Sets the pool to active or inactive.
pub(crate) fn set_active(_deps: DepsMut, _is_active: bool) -> Result<Response, ContractError> {
    unimplemented!("set_active")
}

/// Swaps an exact amount of tokens in for as many tokens out as possible.
pub(crate) fn swap_exact_amount_in(
    deps: DepsMut,
    sender: String,
    token_in: Coin,
    token_out_denom: String,
    minimum_receive: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let sender = deps.api.addr_validate(sender.as_str())?;

    // get the pool info
    let pool = config.white_whale_pool;
    let pair_info = get_pair_info(&deps, &pool)?;
    let ask_asset_info = get_paired_asset_info(&token_in, pair_info, &token_out_denom)?;

    let expected_token_out =
        calc_out_amt_given_in(deps.as_ref(), token_in.clone(), token_out_denom)?.token_out;

    // let receiver_balance = ask_asset_info.query_balance(&deps.querier, deps.api, sender.clone())?;
    let receiver_balance = ask_asset_info.query_pool(&deps.querier, deps.api, sender.clone())?;

    let assertion_data = MinimumReceiveAssertion {
        asset_info: ask_asset_info,
        prev_balance: receiver_balance,
        minimum_receive,
        receiver: sender.clone().into_string(),
    };

    TEMP_MIN_ASSERTION_DATA.save(deps.storage, &assertion_data)?;

    Ok(Response::default()
        .set_data(to_json_binary(&SwapExactAmountInResponseData {
            token_out_amount: expected_token_out.amount,
        })?)
        .add_submessage(SubMsg::reply_on_success(
            create_swap_msg(pool.into_string(), token_in, sender.into_string())?,
            ASSERT_MINIMUM_RECEIVE_REPLY_ID,
        ))
        .add_attributes(vec![("action", "swap_exact_amount_in".to_string())]))
}

/// Swaps as many tokens in as possible for an exact amount of tokens out.
pub(crate) fn swap_exact_amount_out(
    deps: DepsMut,
    sender: String,
    token_out: Coin,
    token_in_max_amount: Uint128,
    token_in_denom: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let sender = deps.api.addr_validate(sender.as_str())?;

    // get the pool info
    let pool = config.white_whale_pool;
    let pair_info = get_pair_info(&deps, &pool)?;
    get_paired_asset_info(&token_out, pair_info, &token_in_denom)?;

    let expected_token_in =
        calc_in_amt_given_out(deps.as_ref(), token_out.clone(), token_in_denom.clone())?.token_in;

    // assert the expected token in amount to get the desired token out amount is less than the
    // maximum token in allowed
    if expected_token_in.amount > token_in_max_amount {
        return Err(ContractError::MaximumTokenInAssertion {
            token_in_max_amount,
            token_in_used: expected_token_in.amount,
        });
    }

    Ok(Response::default()
        .set_data(to_json_binary(&SwapExactAmountOutResponseData {
            token_in_amount: expected_token_in.amount,
        })?)
        .add_message(create_swap_msg(
            pool.into_string(),
            coin(expected_token_in.amount.u128(), token_in_denom.clone()),
            sender.into_string(),
        )?)
        .add_attributes(vec![("action", "swap_exact_amount_out".to_string())]))
}

/// Creates a swap message for the White Whale pool.
fn create_swap_msg(
    contract_addr: String,
    coin: Coin,
    sender: String,
) -> Result<CosmosMsg, ContractError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr,
        msg: to_json_binary(&white_whale_std::pool_network::pair::ExecuteMsg::Swap {
            offer_asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: coin.clone().denom,
                },
                amount: coin.clone().amount,
            },
            belief_price: None,
            max_spread: Some(Decimal::percent(30)),
            to: Some(sender),
        })?,
        funds: vec![coin],
    }))
}

/// Gets the pair info from the White Whale pool.
fn get_pair_info(deps: &DepsMut, pool: &Addr) -> Result<PairInfo, ContractError> {
    let pair_info: PairInfo = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pool.to_string(),
        msg: to_json_binary(&white_whale_std::pool_network::pair::QueryMsg::Pair {})?,
    }))?;
    Ok(pair_info)
}

/// Gets the asset a token is paired with in the White Whale pool.
fn get_paired_asset_info(
    token_a: &Coin,
    pair_info: PairInfo,
    token_b_denom: &String,
) -> Result<AssetInfo, ContractError> {
    // sanity check to make sure the input token is in the pool
    if !pair_info.asset_infos.clone().into_iter().any(|asset_info| {
        asset_info
            == AssetInfo::NativeToken {
                denom: token_a.clone().denom,
            }
    }) {
        return Err(StdError::generic_err(format!(
            "Asset {} not found in the pool",
            token_a.denom
        ))
        .into());
    }

    let asset_info: AssetInfo = pair_info
        .asset_infos
        .into_iter()
        .find(|asset_info| {
            *asset_info
                != AssetInfo::NativeToken {
                    denom: token_a.clone().denom,
                }
        })
        .ok_or_else(|| {
            StdError::generic_err(format!(
                "The asset paired with {} was not found",
                token_a.denom
            ))
        })?;

    // verify the token found matches the expected one
    match asset_info.clone() {
        AssetInfo::Token { .. } => return Err(StdError::generic_err("Token not supported").into()),
        AssetInfo::NativeToken { denom } => {
            if denom != *token_b_denom {
                return Err(ContractError::PairedAssetMissmatch);
            }
        }
    }

    Ok(asset_info)
}
