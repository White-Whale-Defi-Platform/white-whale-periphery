use cosmwasm_std::{Coin, StdResult};
use osmosis_test_tube::{Account, Module, OsmosisTestApp, RunnerResult, SigningAccount, Wasm};
use white_whale::pool_network::asset::{Asset, AssetInfo, PairType, ToCoins};
use white_whale::pool_network::pair::PoolFee;

use osmosis_cw_pool::msg::{InstantiateMsg, QueryMsg};

pub struct TestingSuite {
    app: OsmosisTestApp,
    pub accounts: Vec<SigningAccount>,
    pub ww_pool_addr: String,
    pub cw_osmosis_pool_interface: String,
}

impl TestingSuite {
    #[track_caller]
    pub fn default_with_balances(initial_balance: &[Coin]) -> Self {
        let app = OsmosisTestApp::new();
        let accounts = app.init_accounts(initial_balance, 3).unwrap();

        Self {
            app,
            accounts,
            ww_pool_addr: "".to_string(),
            cw_osmosis_pool_interface: "".to_string(),
        }
    }

    #[track_caller]
    pub fn create_ww_pool(
        &mut self,
        asset_infos: [AssetInfo; 2],
        asset_decimals: [u8; 2],
        pool_fees: PoolFee,
    ) -> &mut Self {
        let wasm = Wasm::new(&self.app);
        let admin = &self.accounts[0];
        let code_id = store_contract(&wasm, "tests/test_artifacts/ww_pool.wasm", admin);

        let contract_addr = wasm
            .instantiate(
                code_id,
                &white_whale::pool_network::pair::InstantiateMsg {
                    asset_infos,
                    token_code_id: 123, // doesn't matter, we are using the token factory to mint LP tokens
                    asset_decimals,
                    pool_fees,
                    fee_collector_addr: admin.address(),
                    pair_type: PairType::ConstantProduct,
                    token_factory_lp: true,
                },
                None,
                Some("ww_pool"),
                &[],
                admin,
            )
            .unwrap()
            .data
            .address;

        self.ww_pool_addr = contract_addr.clone();

        self
    }

    #[track_caller]
    pub fn create_osmosis_pool_interface(&mut self, white_whale_pool: String) -> &mut Self {
        let wasm = Wasm::new(&self.app);
        let admin = &self.accounts[0];
        let code_id = store_contract(&wasm, "tests/test_artifacts/osmosis_cw_pool.wasm", admin);

        let contract_addr = wasm
            .instantiate(
                code_id,
                &InstantiateMsg {
                    white_whale_pool,
                    after_pool_created: None,
                },
                None,
                Some("osmosis_cw_pool_interface"),
                &[],
                admin,
            )
            .unwrap()
            .data
            .address;

        self.ww_pool_addr = contract_addr.clone();

        self
    }
}

/// pool related actions
impl TestingSuite {

    #[track_caller]
    pub fn provide_liquidity(&mut self, assets: [Asset; 2]) -> &mut Self {
        let wasm = Wasm::new(&self.app);
        let contract_addr = self.ww_pool_addr.clone();

        wasm.execute::<white_whale::pool_network::pair::ExecuteMsg>(
            &contract_addr,
            &white_whale::pool_network::pair::ExecuteMsg::ProvideLiquidity {
                assets: assets.clone(),
                slippage_tolerance: None,
                receiver: None,
            },
            &assets.to_vec().to_coins().unwrap(),
            &self.accounts[0],
        )
        .unwrap();

        self
    }

    #[track_caller]
    pub fn query_ww_pool<Q, R>(
        &mut self,
        query_msg: Q,
        result: impl Fn(RunnerResult<R>),
    ) -> &mut Self
        where
            Q: Into<white_whale::pool_network::pair::QueryMsg>,
            R: serde::de::DeserializeOwned,
    {
        let wasm = Wasm::new(&self.app);
        let contract_addr = self.ww_pool_addr.clone();

        let response = wasm.query::<white_whale::pool_network::pair::QueryMsg, R>(
            &contract_addr,
            &query_msg.into(),
        );

        result(response);

        self
    }
}

/// osmosis pool interface related actions
impl TestingSuite {
    #[track_caller]
    pub fn swap_token_in(&mut self) -> &mut Self {
        self
    }

    #[track_caller]
    pub fn query_osmosis_pool_interface<Q, R>(
        &mut self,
        query_msg: Q,
        result: impl Fn(RunnerResult<R>),
    ) -> &mut Self
        where
            Q: Into<QueryMsg>,
            R: serde::de::DeserializeOwned,
    {
        let wasm = Wasm::new(&self.app);
        let contract_addr = self.cw_osmosis_pool_interface.clone();

        let response = wasm.query::<QueryMsg, R>(
            &contract_addr,
            &query_msg.into(),
        );

        result(response);

        self
    }
}

/// Stores a contract given its path and returns the code id
fn store_contract(wasm: &Wasm<OsmosisTestApp>, contract_path: &str, admin: &SigningAccount) -> u64 {
    // Load compiled wasm bytecode
    let wasm_byte_code = std::fs::read(contract_path).unwrap();
    let code_id = wasm
        .store_code(&wasm_byte_code, None, admin)
        .unwrap()
        .data
        .code_id;

    code_id
}
