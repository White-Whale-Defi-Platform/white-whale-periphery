use crate::msg::Config;
use cw_storage_plus::Item;

pub const ACTIVE_STATUS: Item<bool> = Item::new("active_status");

pub const CONFIG: Item<Config> = Item::new("config");
