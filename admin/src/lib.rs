#![no_std]

use contract_trait_macro::derive_contract;
use soroban_sdk::{Address, Env, contract};

use admin_sep::{Admin, Administratable, AdministratableExt, Upgradable};

// pub mod admin;
#[macro_use]
pub mod constructor;

use crate::constructor::HasAdmin;

#[contract]
#[derive_contract(Administratable, Upgradable(ext = AdministratableExt))]
pub struct Contract;

// Upgradable!(Contract);

constructor_gen!(Contract, Contract, (Address, u32));
// admin_gen!(Contract);
// Admin_gen!(Contract);
// Upgradable_gen!(Contract);

type CustomArgs = (Address, u32);

impl HasAdmin for CustomArgs {
    fn admin(&self) -> soroban_sdk::Address {
        self.0.clone()
    }
}

const COUNT: soroban_sdk::Symbol = soroban_sdk::symbol_short!("COUNT");

impl constructor::Constructable<CustomArgs> for Contract {
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
