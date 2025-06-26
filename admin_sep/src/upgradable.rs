use contract_trait_macro::contracttrait;

use crate::administratable::{Administratable, AdministratableExt};

#[contracttrait(extension_required = true)]
pub trait Upgradable {
    fn upgrade(env: &soroban_sdk::Env, wasm_hash: soroban_sdk::BytesN<32>) {
        env.deployer().update_current_contract_wasm(wasm_hash);
    }
}

impl<T: Administratable, N: Upgradable> Upgradable for AdministratableExt<T, N> {
    fn upgrade(env: &soroban_sdk::Env, wasm_hash: soroban_sdk::BytesN<32>) {
        T::admin(env).require_auth();
        N::upgrade(env, wasm_hash);
    }
}
