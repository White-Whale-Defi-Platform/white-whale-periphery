use cw_storage_plus::Item;

use crate::msg::{Config, MinimumReceiveAssertion};

pub const CONFIG: Item<Config> = Item::new("config");

/// temp variables for storing assertion data when doing swaps
pub const TEMP_MIN_ASSERTION_DATA: Item<MinimumReceiveAssertion> =
    Item::new("temp_min_assertion_data");
