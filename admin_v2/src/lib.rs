#![no_std]

use admin_sep::*;
use soroban_sdk::{Address, Env, contract, contractimpl};

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn __constructor(env: &Env, admin: &Address) {
        Self::set_admin(env, admin);
    }

    pub fn new_method(env: &Env) {
        Self::require_admin(env);
    }
}

#[contractimpl(contracttrait)]
impl Administratable for Contract {}

#[contractimpl(contracttrait)]
impl Upgradable for Contract {}

mod test;
