use cosmwasm_std::{OverflowError, StdError, Uint128};
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

    #[error("Attempt to migrate to version {new_version}, but contract is on a higher version {current_version}")]
    MigrateInvalidVersion {
        new_version: Version,
        current_version: Version,
    },
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
