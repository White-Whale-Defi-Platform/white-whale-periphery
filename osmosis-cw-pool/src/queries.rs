use cosmwasm_std::{
    to_json_binary, Coin, Decimal, Deps, QueryRequest, StdError, StdResult, Uint128, WasmQuery,
};
use white_whale::pool_network::asset::{Asset, AssetInfo, ToCoins};
use white_whale::pool_network::pair::{
    ConfigResponse, PoolResponse, ReverseSimulationResponse, SimulationResponse,
};

use crate::msg::{
    CalcInAmtGivenOutResponse, CalcOutAmtGivenInResponse, Config, GetSwapFeeResponse,
    SpotPriceResponse, TotalPoolLiquidityResponse,
};
use crate::state::CONFIG;

/// Queries the pool config
fn get_pool_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;

    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: config.white_whale_pool.to_string(),
        msg: to_json_binary(&white_whale::pool_network::pair::QueryMsg::Config {})?,
    }))
}

/// Queries the pool data
fn get_pool(deps: Deps) -> StdResult<PoolResponse> {
    let config = CONFIG.load(deps.storage)?;

    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: config.white_whale_pool.to_string(),
        msg: to_json_binary(&white_whale::pool_network::pair::QueryMsg::Pool {})?,
    }))
}

/// Finds the amount of tokens in a vector of Assets by denom
fn find_asset_amount_by_denom(assets: &[Asset], denom: &str) -> Option<Uint128> {
    assets
        .iter()
        .find(|&asset| match asset.clone().info {
            AssetInfo::Token { .. } => false,
            AssetInfo::NativeToken { denom: asset_denom } => asset_denom == denom,
        })
        .map(|asset| asset.amount)
}

/// Queries the swap fee
pub(crate) fn get_swap_fee(deps: Deps) -> StdResult<GetSwapFeeResponse> {
    let fees = get_pool_config(deps)?.pool_fees;

    Ok(GetSwapFeeResponse {
        swap_fee: fees.aggregate()?,
    })
}

/// Queries the total pool liquidity
pub(crate) fn get_total_pool_liquidity(deps: Deps) -> StdResult<TotalPoolLiquidityResponse> {
    let pool = get_pool(deps)?;

    Ok(TotalPoolLiquidityResponse {
        total_pool_liquidity: pool.assets.to_coins()?,
    })
}

/// Queries the spot price
pub(crate) fn spot_price(
    deps: Deps,
    quote_asset_denom: String,
    base_asset_denom: String,
) -> StdResult<SpotPriceResponse> {
    let pool = get_pool(deps)?;

    let quote_asset_amount = find_asset_amount_by_denom(&pool.assets, &quote_asset_denom)
        .ok_or_else(|| StdError::generic_err("Quote asset not found"))?;

    let base_asset_amount = find_asset_amount_by_denom(&pool.assets, &base_asset_denom)
        .ok_or_else(|| StdError::generic_err("Base asset not found"))?;

    Ok(SpotPriceResponse {
        spot_price: Decimal::from_ratio(base_asset_amount, quote_asset_amount),
    })
}

/// CalcOutAmtGivenIn calculates the amount of tokenOut given tokenIn and the pool's current state.
pub(crate) fn calc_out_amt_given_in(
    deps: Deps,
    token_in: Coin,
    token_out_denom: String,
) -> StdResult<CalcOutAmtGivenInResponse> {
    let config = CONFIG.load(deps.storage)?;

    assert_denoms(deps, token_in.clone().denom, token_out_denom.clone())?;

    let swap_simulation: SimulationResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: config.white_whale_pool.to_string(),
            msg: to_json_binary(&white_whale::pool_network::pair::QueryMsg::Simulation {
                offer_asset: Asset {
                    info: AssetInfo::NativeToken {
                        denom: token_in.denom,
                    },
                    amount: token_in.amount,
                },
            })?,
        }))?;

    Ok(CalcOutAmtGivenInResponse {
        token_out: Coin {
            denom: token_out_denom,
            amount: swap_simulation.return_amount,
        },
    })
}

/// CalcInAmtGivenOut calculates the amount of tokenIn given tokenOut and the pool's current state.
pub(crate) fn calc_in_amt_given_out(
    deps: Deps,
    token_out: Coin,
    token_in_denom: String,
) -> StdResult<CalcInAmtGivenOutResponse> {
    let config = CONFIG.load(deps.storage)?;

    assert_denoms(deps, token_out.clone().denom, token_in_denom.clone())?;

    let reverse_swap_simulation: ReverseSimulationResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: config.white_whale_pool.to_string(),
            msg: to_json_binary(
                &white_whale::pool_network::pair::QueryMsg::ReverseSimulation {
                    ask_asset: Asset {
                        info: AssetInfo::NativeToken {
                            denom: token_out.denom,
                        },
                        amount: token_out.amount,
                    },
                },
            )?,
        }))?;

    Ok(CalcInAmtGivenOutResponse {
        token_in: Coin {
            denom: token_in_denom,
            amount: reverse_swap_simulation.offer_amount,
        },
    })
}

fn assert_denoms(deps: Deps, token_0: String, token_1: String) -> StdResult<()> {
    let pool = get_pool(deps)?;

    let asset_0 = pool.assets.iter().any(|asset| match asset.clone().info {
        AssetInfo::Token { .. } => false,
        AssetInfo::NativeToken { denom } => denom == token_0,
    });
    let asset_1 = pool.assets.iter().any(|asset| match asset.clone().info {
        AssetInfo::Token { .. } => false,
        AssetInfo::NativeToken { denom } => denom == token_1,
    });

    if asset_0 && asset_1 {
        Ok(())
    } else {
        Err(StdError::generic_err("Asset not found"))
    }
}

/// Queries the config of the contract
pub(crate) fn get_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}
