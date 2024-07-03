extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemEnum};

/// Add a migration-related error variant to an enum.
///
/// The `migrate_invalid_version_error` macro appends an error variant to an existing enum
/// that represents an error occurring when attempting to migrate a contract to a version that
/// is lower than the current version.
///
/// For example, applying the `migrate_invalid_version_error` macro to the following enum:
///
/// ```rust
/// use thiserror::Error;
/// use my_macro_crate::migrate_invalid_version_error;
///
/// #[cw_migrate_invalid_version_error]
/// #[derive(Error, Debug)]
/// pub enum ContractError {
///     #[error("{0}")]
///     Std(#[from] StdError),
/// }
/// ```
///
/// Is equivalent to:
///
/// ```rust
/// use thiserror::Error;
///
/// #[derive(Error, Debug)]
/// pub enum ContractError {
///     #[error("{0}")]
///     Std(#[from] StdError),
///
///     #[error("Attempt to migrate to version {new_version}, but contract is on a higher version {current_version}")]
///     MigrateInvalidVersion {
///         new_version: semver::Version,
///         current_version: semver::Version,
///     },
/// }
/// ```
///
/// The `MigrateInvalidVersion` variant has two fields, `new_version` and `current_version`, both of
/// which are of type `Version`. The error message indicates that the migration attempt failed because
/// the contract is already at a higher version.
///
/// Note: `#[cw_migrate_invalid_version_error]` must be applied _before_ `#[derive(Error, Debug)]`.
#[proc_macro_attribute]
pub fn cw_migrate_invalid_version_error(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input error enum
    let input = parse_macro_input!(item as ItemEnum);
    let name = &input.ident;
    let generics = &input.generics;
    let attrs = &input.attrs; // Capture existing attributes

    // Extract the existing error variants
    let mut variants = input.variants.clone();

    // Define the MigrateInvalidVersion error variant
    let new_variant = syn::parse_quote! {
        #[error("Attempt to migrate to version {new_version}, but contract is on a higher version {current_version}")]
        MigrateInvalidVersion {
            new_version: semver::Version,
            current_version: semver::Version,
        }
    };

    // Add the new variant to the list
    variants.push(new_variant);

    // Generate the updated enum definition
    let expanded = quote! {
        #(#attrs)*
        pub enum #name #generics {
            #variants
        }
    };

    TokenStream::from(expanded)
}
