use cosmwasm_std::{OverflowError, StdError, Uint128};
use cw_utils::ParseReplyError;
use semver::Version;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error(
        "Assertion failed; minimum receive amount: {minimum_receive}, swap amount: {swap_amount}"
    )]
    MinimumReceiveAssertion {
        minimum_receive: Uint128,
        swap_amount: Uint128,
    },

    #[error(
        "Assertion failed; maximum receive amount: {maximum_receive}, swap amount: {swap_amount}"
    )]
    MaximumReceiveAssertion {
        maximum_receive: Uint128,
        swap_amount: Uint128,
    },

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("Cannot read assertion data")]
    CannotReadAssertionData,

    #[error(
        "The token denom {token_denom} does not match the paired asset denom {paired_asset_denom}"
    )]
    PairedAssetMissmatch {
        token_denom: String,
        paired_asset_denom: String,
    },

    #[error("Attempt to migrate to version {new_version}, but contract is on a higher version {current_version}")]
    MigrateInvalidVersion {
        new_version: Version,
        current_version: Version,
    },

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
