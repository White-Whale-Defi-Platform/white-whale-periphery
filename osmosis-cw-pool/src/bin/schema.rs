use cosmwasm_schema::write_api;

use osmosis_cw_pool::msg::{InstantiateMsg, QueryMsg, SudoMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        sudo: SudoMsg,
    }
}
