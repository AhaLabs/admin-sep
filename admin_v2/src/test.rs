#![cfg(test)]
extern crate std;
use crate::{Contract, ContractClient};
use soroban_sdk::{Address, Env, testutils::Address as _};

#[test]
fn test() {
    let env = Env::default();
    let admin = &Address::generate(&env);
    let contract_id = env.register(Contract, (admin,));
    let _client = ContractClient::new(&env, &contract_id);

    // assert_eq!(client.increment(), 2);
    // assert_eq!(client.increment(), 3);
    // assert_eq!(client.increment(), 4);
}
