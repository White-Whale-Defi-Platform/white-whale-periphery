use std::collections::HashMap;

use cosmwasm_std::{to_json_binary, Coin, Uint128};
use osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceRequest;
use osmosis_std::types::osmosis::cosmwasmpool::v1beta1::{
    ContractInfoByPoolIdRequest, ContractInfoByPoolIdResponse, MsgCreateCosmWasmPool,
    UploadCosmWasmPoolCodeAndWhiteListProposal,
};
use osmosis_std::types::osmosis::poolmanager::v1beta1::{
    MsgSwapExactAmountIn, MsgSwapExactAmountInResponse, MsgSwapExactAmountOut,
    MsgSwapExactAmountOutResponse, SwapAmountInRoute, SwapAmountOutRoute,
};
use osmosis_test_tube::{
    Account, Bank, GovWithAppAccess, Module, OsmosisTestApp, RunnerError, RunnerExecuteResult,
    RunnerResult, SigningAccount, Wasm,
};
use white_whale_std::pool_network::asset::{Asset, AssetInfo, PairType, ToCoins};
use white_whale_std::pool_network::pair::PoolFee;

use osmosis_cw_pool::msg::{InstantiateMsg, QueryMsg, SudoMsg};

use crate::osmosis_cosmwasm_pool::CosmwasmPool;
pub struct TestingSuite {
    pub app: OsmosisTestApp,
    pub accounts: HashMap<usize, SigningAccount>,
    pub ww_pool_addr: String,
    pub cw_osmosis_pool_interface: String,
    pub osmosis_pool_id: u64,
}

impl TestingSuite {
    #[track_caller]
    pub fn default_with_balances(initial_balance: &[Coin]) -> Self {
        let app = OsmosisTestApp::new();
        let accounts = app
            .init_accounts(initial_balance, 2)
            .unwrap()
            .into_iter()
            .enumerate()
            .map(|(index, acc)| (index, acc))
            .collect::<HashMap<_, _>>();

        Self {
            app,
            accounts,
            ww_pool_addr: "".to_string(),
            cw_osmosis_pool_interface: "".to_string(),
            osmosis_pool_id: 0,
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
        let admin = &self.accounts[&0];
        let code_id = store_contract(&wasm, "tests/test_artifacts/ww_pool.wasm", admin);

        let contract_addr = wasm
            .instantiate(
                code_id,
                &white_whale_std::pool_network::pair::InstantiateMsg {
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
    pub fn check_address_balance(
        &mut self,
        address: String,
        denom: String,
        result: impl Fn(Uint128),
    ) -> &mut Self {
        let coin: osmosis_std::types::cosmos::base::v1beta1::Coin = Bank::new(&self.app)
            .query_balance(&QueryBalanceRequest { address, denom })
            .unwrap()
            .balance
            .unwrap();

        result(Uint128::new(coin.amount.parse::<u128>().unwrap()));

        self
    }
}

/// pool related actions
impl TestingSuite {
    #[track_caller]
    pub fn provide_liquidity(&mut self, assets: [Asset; 2]) -> &mut Self {
        let wasm = Wasm::new(&self.app);
        let contract_addr = self.ww_pool_addr.clone();

        wasm.execute::<white_whale_std::pool_network::pair::ExecuteMsg>(
            &contract_addr,
            &white_whale_std::pool_network::pair::ExecuteMsg::ProvideLiquidity {
                assets: assets.clone(),
                slippage_tolerance: None,
                receiver: None,
            },
            &assets.to_vec().to_coins().unwrap(),
            &self.accounts[&0],
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
        Q: Into<white_whale_std::pool_network::pair::QueryMsg>,
        R: serde::de::DeserializeOwned,
    {
        let wasm = Wasm::new(&self.app);
        let contract_addr = self.ww_pool_addr.clone();

        let response = wasm.query::<white_whale_std::pool_network::pair::QueryMsg, R>(
            &contract_addr,
            &query_msg.into(),
        );

        result(response);

        self
    }
}

/// osmosis_cosmwasm_pool pool interface related actions
impl TestingSuite {
    #[track_caller]
    pub fn swap_token_in(
        &mut self,
        sender: &SigningAccount,
        token_in: Coin,
        token_out_denom: String,
        token_out_min_amount: Uint128,
        result: impl Fn(RunnerExecuteResult<MsgSwapExactAmountInResponse>),
    ) -> &mut Self {
        let cp = CosmwasmPool::new(&self.app);

        let routes = vec![SwapAmountInRoute {
            pool_id: self.osmosis_pool_id,
            token_out_denom,
        }];

        result(cp.swap_exact_amount_in(
            MsgSwapExactAmountIn {
                sender: sender.address(),
                token_in: Some(token_in.into()),
                routes,
                token_out_min_amount: token_out_min_amount.into(),
            },
            sender,
        ));

        self
    }
    #[track_caller]
    pub fn swap_token_out(
        &mut self,
        sender: &SigningAccount,
        token_out: Coin,
        token_in_denom: String,
        token_in_max_amount: Uint128,
        result: impl Fn(RunnerExecuteResult<MsgSwapExactAmountOutResponse>),
    ) -> &mut Self {
        let cp = CosmwasmPool::new(&self.app);

        let routes = vec![SwapAmountOutRoute {
            pool_id: self.osmosis_pool_id,
            token_in_denom,
        }];

        result(cp.swap_exact_amount_out(
            MsgSwapExactAmountOut {
                sender: sender.address(),
                routes,
                token_in_max_amount: token_in_max_amount.into(),
                token_out: Some(token_out.into()),
            },
            sender,
        ));

        self
    }

    #[track_caller]
    pub fn set_active(
        &mut self,
        active: bool,
        result: impl Fn(Result<Vec<u8>, RunnerError>),
    ) -> &mut Self {
        result(execute_sudo(
            &self.app,
            &self.cw_osmosis_pool_interface,
            SudoMsg::SetActive { is_active: active },
        ));
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

        let response = wasm.query::<QueryMsg, R>(&contract_addr, &query_msg.into());

        result(response);

        self
    }
}

/// pool manager stuff
impl TestingSuite {
    #[track_caller]
    pub fn create_cosmwasm_pool(&mut self) -> &mut Self {
        let cp = CosmwasmPool::new(&self.app);
        let gov = GovWithAppAccess::new(&self.app);

        let signer = &self.accounts[&0];

        let code_id = 2; // ww_pool is code_id 1 at this point
        gov.propose_and_execute(
            UploadCosmWasmPoolCodeAndWhiteListProposal::TYPE_URL.to_string(),
            UploadCosmWasmPoolCodeAndWhiteListProposal {
                title: String::from("store test cosmwasm pool code"),
                description: String::from("test"),
                wasm_byte_code: get_wasm_byte_code("tests/test_artifacts/osmosis_cw_pool.wasm"),
            },
            signer.address(),
            signer,
        )
        .unwrap();

        let instantiate_msg = &InstantiateMsg {
            white_whale_pool: self.ww_pool_addr.clone(),
            after_pool_created: None,
        };

        let res = cp
            .create_cosmwasm_pool(
                MsgCreateCosmWasmPool {
                    code_id,
                    instantiate_msg: to_json_binary(instantiate_msg).unwrap().to_vec(),
                    sender: signer.address(),
                },
                signer,
            )
            .unwrap();

        let pool_id = res.data.pool_id;

        let ContractInfoByPoolIdResponse {
            contract_address,
            code_id: _,
        } = cp
            .contract_info_by_pool_id(&ContractInfoByPoolIdRequest { pool_id })
            .unwrap();

        self.cw_osmosis_pool_interface = contract_address;
        self.osmosis_pool_id = pool_id;

        self
    }
}

/// Gets wasm byte code from a contract
fn get_wasm_byte_code(contract_path: &str) -> Vec<u8> {
    std::fs::read(contract_path).unwrap()
}

/// Stores a contract given its path and returns the code id
fn store_contract(wasm: &Wasm<OsmosisTestApp>, contract_path: &str, admin: &SigningAccount) -> u64 {
    // Load compiled wasm bytecode
    let wasm_byte_code = get_wasm_byte_code(contract_path);
    let code_id = wasm
        .store_code(&wasm_byte_code, None, admin)
        .unwrap()
        .data
        .code_id;

    code_id
}

#[track_caller]
/// Executes sudo messages
fn execute_sudo<M: serde::Serialize>(
    app: &OsmosisTestApp,
    contract_address: &str,
    sudo_msg: M,
) -> Result<Vec<u8>, RunnerError> {
    app.wasm_sudo(&contract_address, sudo_msg)
}
