#![recursion_limit = "128"]
extern crate proc_macro;

use proc_macro::TokenStream;

mod args;
mod contracttrait;
mod error;
mod util;

/// Generates a macro_rules macro that generates a external implementation of the trait for the contract
///
///
/// # Panics
///
/// This macro will panic if:
/// - The input `TokenStream` cannot be parsed into a valid Rust item.
/// - The `contracttrait::generate` function fails to generate the companion trait.
///
#[proc_macro_attribute]
pub fn contracttrait(attr: TokenStream, item: TokenStream) -> TokenStream {
    let (parsed_args, parsed) = match args::parse(attr, item) {
        Ok((args, item)) => (args, item),
        Err(e) => return Into::<proc_macro2::TokenStream>::into(e).into(),
    };
    contracttrait::generate(&parsed_args, &parsed).into()
}

/// Derives a contract trait for the given Contract struct.
#[proc_macro_attribute]
pub fn derive_contract(attr: TokenStream, item: TokenStream) -> TokenStream {
    let (parsed_args, parsed) = match args::parse(attr, item) {
        Ok((args, item)) => (args, item),
        Err(e) => return Into::<proc_macro2::TokenStream>::into(e).into(),
    };
    contracttrait::derive_contract(&parsed_args, &parsed).into()
}
