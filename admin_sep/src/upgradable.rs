use contract_trait_macro::contracttrait;

use crate::administratable::Administratable;

#[contracttrait]
pub trait Upgradable: Administratable {
    fn upgrade(env: &soroban_sdk::Env, wasm_hash: soroban_sdk::BytesN<32>) {
        Self::admin(env).require_auth();
        env.deployer().update_current_contract_wasm(wasm_hash);
    }
}
