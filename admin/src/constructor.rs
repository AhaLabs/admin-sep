use soroban_sdk::{Address, Env};


pub trait HasAdmin {
    fn admin(&self) -> Address;
}

// #[contracttrait(DefaultAdmin)]
pub trait Constructable<T = Address>: admin_sep::Administratable
where
    T: HasAdmin,
{
    #[allow(unused_variables)]
    fn construct(env: &Env, args: T) {}
    fn __constructor(env: &Env, args: T) {
        Self::set_admin(env, args.admin());
        Self::construct(env, args);
    }
}

// // Generates
macro_rules! constructor_gen {
    ($contract_name:ident) => {
        impl constructor::Constructable<soroban_sdk::Address> for $contract_name {}
        constructor_gen!($contract_name, $contract_name, soroban_sdk::Address);
    };
    ($contract_name:ident, $impl_name:path, $type_name:ty) => {
        mod c {
            use super::*;
            pub trait ConstructableExt {
                type Args: HasAdmin;
                fn __constructor(env: Env, args: Self::Args);
            }
            #[soroban_sdk::contractimpl]
            impl ConstructableExt for $contract_name {
                type Args = $type_name;
                fn __constructor(env: Env, args: Self::Args) {
                    <$impl_name as constructor::Constructable<$type_name>>::__constructor(
                        &env, args,
                    );
                }
            }
        }
    };
    () => {};
}

// impl Constructable<Address> for DefaultAdmin {}

impl HasAdmin for Address {
    fn admin(&self) -> Address {
        self.clone()
    }
}
