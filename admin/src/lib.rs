#![no_std]

use soroban_sdk::{Address, Env, contract, contracttype};

use admin_sep::{
    Admin, Administratable, AdministratableExt, Constructable, HasAdmin, Upgradable,
    derive_contract,
};

#[contract]
#[derive_contract(Administratable, Upgradable(ext = AdministratableExt))]
pub struct Contract;

Constructable!(Contract, Contract, CustomArgs);

#[contracttype]
pub struct CustomArgs(pub Address, pub u32);

impl HasAdmin for CustomArgs {
    fn admin(&self) -> &soroban_sdk::Address {
        &self.0
    }
}

const COUNT: soroban_sdk::Symbol = soroban_sdk::symbol_short!("COUNT");

impl Constructable<CustomArgs> for Contract {
    fn construct(env: &Env, args: CustomArgs) {
        env.storage().persistent().set(&COUNT, &args.1);
    }
}

#[soroban_sdk::contractimpl]
impl Contract {
    pub fn increment(env: Env) -> u32 {
        let mut count: u32 = env.storage().persistent().get(&COUNT).unwrap_or(0);
        count += 1;
        env.storage().persistent().set(&COUNT, &count);
        count
    }
}

mod test;
