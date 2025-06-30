#![recursion_limit = "128"]
extern crate proc_macro;

use proc_macro::TokenStream;

mod args;
mod contracttrait;
mod error;
mod util;

/// # Creates a Contract Trait
/// 
/// A contract trait is defines an interface of a contract and declaritive macro with the same name.
/// 
/// When writing a soroban contract, you must expose a methods in an implementation with `#[contractimpl]`.
/// This works for implementations of traits, but the implementation must define all methods of the trait.
/// 
/// For example, consider the following trait and a default implementation:
/// 
/// ```ignore
/// trait Administratable {
///    fn admin(env: &Env) -> Address;
///    fn set_admin(env: &Env, new_admin: &Address);
/// }
/// 
/// struct Admin;
/// impl Administratable for Admin {
///    fn admin(env: &Env) -> Address {
///  //...
/// 
/// }
/// 
/// #[contract]
/// pub struct Contract;
/// #[contractimpl]
/// impl Administratable Contract {
///     fn admin(env: &Env) -> Address {
///        Admin::admin(env)
///     }
///     fn set_admin(env: &Env, new_admin: &Address) {
///       Admin::set_admin(env, new_admin);
///     }
/// } 
/// ```
///
/// Now this works, but it is not very convenient and is very verbose.
/// One way to make this more convenient is to use an associated type in the trait:
/// 
/// ```ignore
/// trait Administratable {
///   type Impl: Administratable;
///   fn admin(env: &Env) -> Address {
///     Self::Impl::admin(env)}
///   }
///   fn set_admin(env: &Env, new_admin: &Address) {
///      Self::Impl::set_admin(env, new_admin);
///   }
/// }
/// ```
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
/// 
/// ```ignore
/// #[contract]
/// #[derive_contract(Administratable, Upgradable(ext = AdministratableExt))]
/// pub struct Contract;
/// ```
#[proc_macro_attribute]
pub fn derive_contract(attr: TokenStream, item: TokenStream) -> TokenStream {
    let (parsed_args, parsed) = match args::parse(attr, item) {
        Ok((args, item)) => (args, item),
        Err(e) => return Into::<proc_macro2::TokenStream>::into(e).into(),
    };
    contracttrait::derive_contract(&parsed_args, &parsed).into()
}
