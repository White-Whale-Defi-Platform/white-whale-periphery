use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;
use white_whale::migrate_guards::check_contract_name;

use crate::error::ContractError;
use crate::msg::{Config, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg};
use crate::state::CONFIG;
use crate::ContractError::MigrateInvalidVersion;
use crate::{commands, queries};

const CONTRACT_NAME: &str = "crates.io:white_whale-osmosis_cw_pool";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONFIG.save(
        deps.storage,
        &Config {
            white_whale_pool: deps.api.addr_validate(&msg.white_whale_pool)?,
        },
    )?;

    let response = Response::default().add_attributes(vec![("action", "instantiate".to_string())]);

    if let Some(after_pool_created) = msg.after_pool_created {
        Ok(response.set_data(to_json_binary(&after_pool_created)?))
    } else {
        Ok(response)
    }
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // only the contract itself can execute messages
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }

    match msg {
        ExecuteMsg::AssertMinimumReceive {
            asset_info,
            prev_balance,
            minimum_receive,
            receiver,
        } => {
            // let receiver_balance = asset_info.query_balance(
            let receiver_balance = asset_info.query_pool(
                &deps.querier,
                deps.api,
                deps.api.addr_validate(receiver.as_str())?,
            )?;
            let swap_amount = receiver_balance.checked_sub(prev_balance)?;

            if swap_amount < minimum_receive {
                return Err(ContractError::MinimumReceiveAssertion {
                    minimum_receive,
                    swap_amount,
                });
            }

            Ok(Response::default().add_attribute("action", "assert_minimum_receive"))
        }
        ExecuteMsg::AssertMaximumReceive {
            asset_info,
            prev_balance,
            maximum_receive,
            receiver,
        } => {
            // let receiver_balance = asset_info.query_balance(
            let receiver_balance = asset_info.query_pool(
                &deps.querier,
                deps.api,
                deps.api.addr_validate(receiver.as_str())?,
            )?;
            let swap_amount = receiver_balance.checked_sub(prev_balance)?;

            if swap_amount > maximum_receive {
                return Err(ContractError::MaximumReceiveAssertion {
                    maximum_receive,
                    swap_amount,
                });
            }

            Ok(Response::default().add_attribute("action", "assert_maximum_receive"))
        }
    }
}

#[entry_point]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::SetActive { is_active } => commands::set_active(deps, is_active),
        SudoMsg::SwapExactAmountIn {
            sender,
            token_in,
            token_out_min_amount,
            ..
        } => commands::swap_exact_amount_in(deps, env, sender, token_in, token_out_min_amount),
        SudoMsg::SwapExactAmountOut {
            sender,
            token_out,
            token_in_max_amount,
            ..
        } => commands::swap_exact_amount_out(deps, env, sender, token_out, token_in_max_amount),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetSwapFee {} => Ok(to_json_binary(&queries::get_swap_fee(deps)?)?),
        QueryMsg::IsActive {} => unimplemented!(
            "This query is not implemented. Query the Config on the White Whale pool instead."
        ),
        QueryMsg::GetTotalPoolLiquidity {} => {
            Ok(to_json_binary(&queries::get_total_pool_liquidity(deps)?)?)
        }
        QueryMsg::SpotPrice {
            quote_asset_denom,
            base_asset_denom,
        } => Ok(to_json_binary(&queries::spot_price(
            deps,
            quote_asset_denom,
            base_asset_denom,
        )?)?),
        QueryMsg::CalcOutAmtGivenIn {
            token_in,
            token_out_denom,
            ..
        } => Ok(to_json_binary(&queries::calc_out_amt_given_in(
            deps,
            token_in,
            token_out_denom,
        )?)?),
        QueryMsg::CalcInAmtGivenOut {
            token_out,
            token_in_denom,
            ..
        } => Ok(to_json_binary(&queries::calc_in_amt_given_out(
            deps,
            token_out,
            token_in_denom,
        )?)?),
        QueryMsg::GetConfig {} => Ok(to_json_binary(&queries::get_config(deps)?)?),
    }
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    check_contract_name(deps.storage, CONTRACT_NAME.to_string())?;

    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version >= version {
        return Err(MigrateInvalidVersion {
            current_version: storage_version,
            new_version: version,
        });
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
