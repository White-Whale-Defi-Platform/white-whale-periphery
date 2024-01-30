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
        "SwapExactAmountIn returned less than expected. Minimum receive amount: {minimum_receive}, received amount after swap: {swap_amount}"
    )]
    MinimumReceiveAssertion {
        minimum_receive: Uint128,
        swap_amount: Uint128,
    },

    #[error(
        "SwapExactAmountOut used more tokens than allowed. Maximum token in amount: {token_in_max_amount}, token in used: {token_in_used}"
    )]
    MaximumTokenInAssertion {
        token_in_max_amount: Uint128,
        token_in_used: Uint128,
    },

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("Cannot read assertion data")]
    CannotReadAssertionData,

    #[error("Impossible to match the paired tokens with the token provided.")]
    PairedAssetMissmatch,

    #[error("Attempt to migrate to version {new_version}, but contract is on a higher version {current_version}")]
    MigrateInvalidVersion {
        new_version: Version,
        current_version: Version,
    },

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),

    #[error("Can't swap zero amount")]
    ZeroAmount,
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
