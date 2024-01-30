use cosmwasm_std::{coin, Decimal, Uint128};
use osmosis_std::types::osmosis::poolmanager::v1beta1::{
    MsgSwapExactAmountInResponse, MsgSwapExactAmountOutResponse,
};
use osmosis_test_tube::{Account, RunnerError};
use white_whale_std::fee::Fee;
use white_whale_std::pool_network::asset::{Asset, AssetInfo};
use white_whale_std::pool_network::pair::PoolFee;

use osmosis_cw_pool::msg::{CalcInAmtGivenOutResponse, CalcOutAmtGivenInResponse, QueryMsg};

use crate::suite::TestingSuite;

mod osmosis_cosmwasm_pool;
mod suite;

#[test]
fn swap_tokens_in() {
    let mut suite = TestingSuite::default_with_balances(&[
        coin(1_000_000_000_000_000, "uosmo"),
        coin(1_000_000_000_000_000, "uwhale"),
    ]);

    suite.create_ww_pool(
        [
            AssetInfo::NativeToken {
                denom: "uosmo".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
        ],
        [6, 6],
        PoolFee {
            protocol_fee: Fee {
                share: Decimal::permille(1),
            },
            swap_fee: Fee {
                share: Decimal::permille(1),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            osmosis_fee: Fee {
                share: Decimal::permille(1),
            },
        },
    );

    suite
        .provide_liquidity([
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uosmo".to_string(),
                },
                amount: Uint128::new(10_000_000),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(10_000_000),
            },
        ])
        .query_ww_pool(
            white_whale_std::pool_network::pair::QueryMsg::Pool {},
            |res: Result<white_whale_std::pool_network::pair::PoolResponse, RunnerError>| {
                let pool_response = res.unwrap();
                assert_eq!(pool_response.total_share, Uint128::new(10_000_000));
            },
        );

    suite.create_cosmwasm_pool();

    let new_account = suite
        .app
        .init_account(&[coin(10_000_000_000, "uosmo"), coin(10_000_000_000, "usdc")])
        .unwrap();

    suite
        .swap_token_in(
            &new_account,
            coin(0, "usdc"),
            "uwhale".to_string(),
            Uint128::new(9_950),
            |result| {
                let err = result.unwrap_err();
                assert_eq!(
                    err,
                    RunnerError::ExecuteError {
                        msg: "0usdc: invalid coins".to_string()
                    }
                );
            },
        )
        .swap_token_in(
            &new_account,
            coin(10_000, "usdc"),
            "uwhale".to_string(),
            Uint128::new(9_950),
            |result| {
                let err = result.unwrap_err();
                assert_eq!(
                    err,
                    RunnerError::ExecuteError {
                        msg: "failed to execute message; message index: 0: Generic error: Asset \
                    usdc not found in the pool: execute wasm contract failed".to_string()
                    }
                );
            },
        )
        .swap_token_in(
            &new_account,
            coin(10_000, "usdc"),
            "uosmo".to_string(),
            Uint128::new(9_950),
            |result| {
                let err = result.unwrap_err();
                assert_eq!(
                    err,
                    RunnerError::ExecuteError {
                        msg: "failed to execute message; message index: 0: Generic error: Asset \
                    usdc not found in the pool: execute wasm contract failed".to_string()
                    }
                );
            },
        )
        .swap_token_in(
            &new_account,
            coin(10_000, "uosmo"),
            "usdc".to_string(),
            Uint128::new(9_950),
            |result| {
                let err = result.unwrap_err();
                assert_eq!(
                    err,
                    RunnerError::ExecuteError {
                        msg: "failed to execute message; message index: 0: Impossible to match \
                    the paired tokens with the token provided.: execute wasm contract failed".to_string()
                    }
                );
            },
        )
        .swap_token_in(
            &new_account,
            coin(10_000, "uosmo"),
            "uwhale".to_string(),
            Uint128::new(10_000),
            |result| {
                let err = result.unwrap_err();
                assert_eq!(
                    err,
                    RunnerError::ExecuteError {
                        msg: "failed to execute message; message index: 0: dispatch: submessages: reply: \
                        SwapExactAmountIn returned less than expected. Minimum receive amount: 10000, received amount \
                        after swap: 9963: execute wasm contract failed".to_string()
                    }
                );
            },
        )
        .check_address_balance(new_account.address().clone(), "uwhale".into(), |amount| {
            assert_eq!(amount, Uint128::zero());
        })
        .query_osmosis_pool_interface(QueryMsg::CalcOutAmtGivenIn {
            token_in: coin(10_000, "uosmo"),
            token_out_denom: "uwhale".to_string(),
            swap_fee: Default::default(),
        }, |result: Result<CalcOutAmtGivenInResponse, RunnerError>| {
            let response = result.unwrap();
            assert_eq!(
                response,
                CalcOutAmtGivenInResponse {
                    token_out: coin(9_963, "uwhale")
                }
            );
        })
        .swap_token_in(
            &new_account,
            coin(10_000, "uosmo"),
            "uwhale".to_string(),
            Uint128::new(9_950),
            |result| {
                println!("result: {:?}", result);
                let response = result.unwrap();
                assert_eq!(
                    response.data,
                    MsgSwapExactAmountInResponse {
                        token_out_amount: "9963".to_string()
                    }
                );
            },
        )
        .check_address_balance(new_account.address().clone(), "uwhale".into(), |amount| {
            assert_eq!(amount, Uint128::new(9_963));
        })
        .swap_token_in(
            &new_account,
            coin(10_000, "uwhale"),
            "uosmo".to_string(),
            Uint128::new(9_950),
            |result| {
                let err = result.unwrap_err();
                assert_eq!(
                    err,
                    RunnerError::ExecuteError {
                        msg: "failed to execute message; message index: 0: spendable balance 9963uwhale is smaller than \
                        10000uwhale: insufficient funds".to_string()
                    }
                );
            },
        );
}

#[test]
fn swap_tokens_out() {
    let mut suite = TestingSuite::default_with_balances(&[
        coin(1_000_000_000_000_000, "uosmo"),
        coin(1_000_000_000_000_000, "uwhale"),
    ]);

    suite.create_ww_pool(
        [
            AssetInfo::NativeToken {
                denom: "uosmo".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
        ],
        [6, 6],
        PoolFee {
            protocol_fee: Fee {
                share: Decimal::permille(1),
            },
            swap_fee: Fee {
                share: Decimal::permille(1),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
            osmosis_fee: Fee {
                share: Decimal::permille(1),
            },
        },
    );

    suite
        .provide_liquidity([
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uosmo".to_string(),
                },
                amount: Uint128::new(10_000_000),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(10_000_000),
            },
        ])
        .query_ww_pool(
            white_whale_std::pool_network::pair::QueryMsg::Pool {},
            |res: Result<white_whale_std::pool_network::pair::PoolResponse, RunnerError>| {
                let pool_response = res.unwrap();
                assert_eq!(pool_response.total_share, Uint128::new(10_000_000));
            },
        );

    suite.create_cosmwasm_pool();

    let new_account = suite
        .app
        .init_account(&[coin(10_000_000_000, "uosmo"), coin(10_000_000_000, "usdc")])
        .unwrap();
    //2_967_382_500uosmo
    println!(
        "cw_osmosis_pool_interface: {:?}",
        suite.cw_osmosis_pool_interface
    );
    println!("ww_pool_addr: {:?}", suite.ww_pool_addr);
    println!("new_account: {:?}", new_account.address());

    suite
        .swap_token_out(
            &new_account,
            coin(0, "usdc"),
            "uwhale".to_string(),
            Uint128::new(10_000),
            |result| {
                let err = result.unwrap_err();
                assert_eq!(
                    err,
                    RunnerError::ExecuteError {
                        msg: "0usdc: invalid coins".to_string()
                    }
                );
            },
        )
        .swap_token_out(
            &new_account,
            coin(10_000, "usdc"),
            "uwhale".to_string(),
            Uint128::new(10_000),
            |result| {
                let err = result.unwrap_err();
                assert_eq!(
                    err,
                    RunnerError::ExecuteError {
                        msg: "failed to execute message; message index: 0: Generic error: Asset usdc not found in the \
                        pool: query wasm contract failed".to_string()
                    }
                );
            },
        )
        .swap_token_out(
            &new_account,
            coin(10_000, "usdc"),
            "uosmo".to_string(),
            Uint128::new(10_000),
            |result| {
                let err = result.unwrap_err();
                assert_eq!(
                    err,
                    RunnerError::ExecuteError {
                        msg: "failed to execute message; message index: 0: Generic error: Asset usdc not found in the \
                        pool: query wasm contract failed".to_string()
                    }
                );
            },
        )
        .swap_token_out(
            &new_account,
            coin(10_000, "uosmo"),
            "usdc".to_string(),
            Uint128::new(10_000),
            |result| {
                let err = result.unwrap_err();
                assert_eq!(
                    err,
                    RunnerError::ExecuteError {
                        msg: "failed to execute message; message index: 0: Generic error: Asset usdc not found in the \
                        pool: query wasm contract failed".to_string()
                    }
                );
            },
        )
        .query_osmosis_pool_interface(QueryMsg::CalcInAmtGivenOut {
            token_out: coin(10_000, "uwhale"),
            token_in_denom: "uosmo".to_string(),
            swap_fee: Default::default(),
        }, |result: Result<CalcInAmtGivenOutResponse, RunnerError>| {
            let response = result.unwrap();
            assert_eq!(
                response,
                CalcInAmtGivenOutResponse {
                    token_in: coin(10_040, "uosmo")
                }
            );
        })
        .swap_token_out(
            &new_account,
            coin(10_000, "uwhale"),
            "uosmo".to_string(),
            Uint128::new(10_000),
            |result| {
                let err = result.unwrap_err();
                assert_eq!(
                    err,
                    RunnerError::ExecuteError {
                        msg: "failed to execute message; message index: 0: SwapExactAmountOut used more tokens than \
                        allowed. Maximum token in amount: 10000, token in used: 10040: execute wasm contract failed".to_string()
                    }
                );
            },
        )
        .check_address_balance(new_account.address().clone(), "uosmo".into(), |amount| {
            assert_eq!(amount, Uint128::new(10_000_000_000));
        })
        .swap_token_out(
            &new_account,
            coin(10_000, "uwhale"),
            "uosmo".to_string(),
            Uint128::new(10_100),
            |result| {
                println!("result: {:?}", result);
                let response = result.unwrap();
                assert_eq!(
                    response.data,
                    MsgSwapExactAmountOutResponse {
                        token_in_amount: "10040".to_string()
                    }
                );
            },
        )
        .check_address_balance(new_account.address().clone(), "uwhale".into(), |amount| {
            assert_eq!(amount, Uint128::new(9_999));
        })
    ;
}
