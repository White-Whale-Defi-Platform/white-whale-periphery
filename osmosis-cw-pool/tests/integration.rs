use cosmwasm_std::{coin, Decimal, Uint128};
use osmosis_test_tube::RunnerError;
use white_whale::fee::Fee;
use white_whale::pool_network::asset::{Asset, AssetInfo};
use white_whale::pool_network::pair::PoolFee;

use crate::suite::TestingSuite;

mod suite;

#[test]
fn swap_tokens_in() {
    let mut suite = TestingSuite::default_with_balances(&[
        coin(1_000_000_000_000, "uosmo"),
        coin(1_000_000_000_000, "uwhale"),
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

    let ww_pool_addr = suite.ww_pool_addr.clone();

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
            white_whale::pool_network::pair::QueryMsg::Pool {},
            |res: Result<white_whale::pool_network::pair::PoolResponse, RunnerError>| {
                let pool_response = res.unwrap();
                assert_eq!(pool_response.total_share, Uint128::new(10_000_000));
            },
        );

    suite.create_osmosis_pool_interface(ww_pool_addr);
}
