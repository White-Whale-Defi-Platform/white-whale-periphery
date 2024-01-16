use cosmwasm_std::{
    to_json_binary, Addr, Coin, CosmosMsg, DepsMut, Env, QueryRequest, Response, StdError, Uint128,
    WasmMsg, WasmQuery,
};
use white_whale::pool_network::asset::{Asset, AssetInfo, PairInfo};

use crate::msg::ExecuteMsg;
use crate::state::CONFIG;
use crate::ContractError;

/// Sets the pool to active or inactive.
pub(crate) fn set_active(_deps: DepsMut, _is_active: bool) -> Result<Response, ContractError> {
    unimplemented!("set_active")
}

/// Swaps an exact amount of tokens in for as many tokens out as possible.
pub(crate) fn swap_exact_amount_in(
    deps: DepsMut,
    env: Env,
    sender: String,
    token_in: Coin,
    minimum_receive: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let sender = deps.api.addr_validate(sender.as_str())?;

    // get the pool info
    let pool = config.white_whale_pool;
    let pair_info = get_pair_info(&deps, &pool)?;
    let ask_asset_info = get_paired_asset_info(&token_in, pair_info)?;

    let mut messages: Vec<CosmosMsg> = vec![];

    // add swap message
    messages.push(create_swap_msg(
        pool.into_string(),
        token_in,
        sender.clone().into_string(),
    )?);

    // Execute minimum amount assertion
    // let receiver_balance = ask_asset_info.query_balance(&deps.querier, deps.api, sender.clone())?;
    let receiver_balance = ask_asset_info.query_pool(&deps.querier, deps.api, sender.clone())?;
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        funds: vec![],
        msg: to_json_binary(&ExecuteMsg::AssertMinimumReceive {
            asset_info: ask_asset_info,
            prev_balance: receiver_balance,
            minimum_receive,
            receiver: sender.into_string(),
        })?,
    }));

    // @Boss:
    // Sorry for lacking documentation, But we need to `set_data` to response with this struct
    // ```
    // #[cw_serde]
    // pub struct SwapExactAmountInResponseData {
    //   pub token_out_amount: Uint128,
    // }
    // ```
    // so that the calculated amount can be used further in swap routing
    Ok(Response::default()
        .add_messages(messages)
        .add_attributes(vec![("action", "swap_exact_amount_in".to_string())]))
}

/// Swaps as many tokens in as possible for an exact amount of tokens out.
pub(crate) fn swap_exact_amount_out(
    deps: DepsMut,
    env: Env,
    sender: String,
    token_out: Coin,
    maximum_receive: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let sender = deps.api.addr_validate(sender.as_str())?;

    // get the pool info
    let pool = config.white_whale_pool;
    let pair_info = get_pair_info(&deps, &pool)?;
    let offer_asset_info = get_paired_asset_info(&token_out, pair_info)?;

    let mut messages: Vec<CosmosMsg> = vec![];

    // add swap message
    messages.push(create_swap_msg(
        pool.into_string(),
        token_out,
        sender.clone().into_string(),
    )?);

    // Execute maximum amount assertion
    let receiver_balance =
        // offer_asset_info.query_balance(&deps.querier, deps.api, sender.clone())?;
        offer_asset_info.query_pool(&deps.querier, deps.api, sender.clone())?;

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        funds: vec![],
        msg: to_json_binary(&ExecuteMsg::AssertMaximumReceive {
            asset_info: offer_asset_info,
            prev_balance: receiver_balance,
            maximum_receive,
            receiver: sender.into_string(),
        })?,
    }));

    // @Boss:
    // Sorry for lacking documentation, But we need to `set_data` to response with this struct
    // ```
    // #[cw_serde]
    // pub struct SwapExactAmountOutResponseData {
    //     pub token_in_amount: Uint128,
    // }
    // ```
    // so that the calculated amount can be used further in swap routing
    Ok(Response::default().add_attributes(vec![("action", "swap_exact_amount_out".to_string())]))
}

/// Creates a swap message for the White Whale pool.
fn create_swap_msg(
    contract_addr: String,
    coin: Coin,
    sender: String,
) -> Result<CosmosMsg, ContractError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr,
        msg: to_json_binary(&white_whale::pool_network::pair::ExecuteMsg::Swap {
            offer_asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: coin.clone().denom,
                },
                amount: coin.clone().amount,
            },
            belief_price: None,
            max_spread: None,
            to: Some(sender),
        })?,
        funds: vec![coin],
    }))
}

/// Gets the pair info from the White Whale pool.
fn get_pair_info(deps: &DepsMut, pool: &Addr) -> Result<PairInfo, ContractError> {
    let pair_info: PairInfo = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pool.to_string(),
        msg: to_json_binary(&white_whale::pool_network::pair::QueryMsg::Pair {})?,
    }))?;
    Ok(pair_info)
}

/// Gets the asset a token is paired with in the White Whale pool.
fn get_paired_asset_info(token_in: &Coin, pair_info: PairInfo) -> Result<AssetInfo, ContractError> {
    let asset_info: AssetInfo = pair_info
        .asset_infos
        .into_iter()
        .find(|asset_info| {
            *asset_info
                != AssetInfo::NativeToken {
                    denom: token_in.clone().denom,
                }
        })
        .ok_or_else(|| {
            StdError::generic_err(format!(
                "The asset paired with {} was not found",
                token_in.denom
            ))
        })?;
    Ok(asset_info)
}
