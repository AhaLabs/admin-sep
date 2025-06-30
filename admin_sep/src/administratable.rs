use contract_trait_macro::contracttrait;
use soroban_sdk::{Address, Env, Symbol, symbol_short};

/// Trait for using an admin address to control access.
#[contracttrait(default = Admin, is_extension = true)]
pub trait Administratable {
    fn admin(env: &Env) -> soroban_sdk::Address;
    fn set_admin(env: &Env, new_admin: soroban_sdk::Address);
}

pub const STORAGE_KEY: Symbol = symbol_short!("A");

fn get(env: &Env) -> Option<Address> {
    env.storage().instance().get(&STORAGE_KEY)
}

pub struct Admin;

impl Administratable for Admin {
    type Impl = Admin;
    fn admin(env: &Env) -> soroban_sdk::Address {
        unsafe { get(env).unwrap_unchecked() }
    }
    fn set_admin(env: &Env, new_admin: soroban_sdk::Address) {
        if let Some(address) = get(env) {
            address.require_auth();
        }
        env.storage().instance().set(&STORAGE_KEY, &new_admin);
    }
}

pub trait AdminExt {
    fn require_admin(e: &Env);
}

impl<T> AdminExt for T
where
    T: Administratable,
{
    fn require_admin(e: &Env) {
        T::admin(e).require_auth();
    }
}
