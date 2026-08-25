use soroban_sdk::{Address, Env, Symbol, contracttrait, symbol_short};

/// Trait for using an admin address to control access.
///
/// # Safety invariant
///
/// The default [`Self::admin`] assumes the admin storage entry exists.
/// Consuming contracts uphold this by calling [`Self::set_admin`] from their
/// constructor (its first call skips the auth check for exactly this
/// purpose), so the entry is written before any entry point can read it.
/// A contract that adopts this trait without constructor wiring makes
/// `admin()` undefined behavior.
#[contracttrait]
pub trait Administratable {
    fn admin(env: &Env) -> soroban_sdk::Address {
        // SAFETY: every consuming contract stores the admin in its
        // constructor (see the trait-level safety invariant), so the entry
        // exists before any read.
        unsafe { admin_from_storage(env).unwrap_unchecked() }
    }

    fn set_admin(env: &Env, new_admin: soroban_sdk::Address) {
        if let Some(owner) = admin_from_storage(env) {
            owner.require_auth();
        }
        env.storage().instance().set(STORAGE_KEY, &new_admin);
    }
}

pub trait AdministratableExtension: Administratable {
    fn require_admin(env: &Env);
}

impl<T: Administratable> AdministratableExtension for T {
    fn require_admin(env: &Env) {
        Self::admin(env).require_auth();
    }
}

fn admin_from_storage(env: &Env) -> Option<Address> {
    env.storage().instance().get(STORAGE_KEY)
}

pub const STORAGE_KEY: &Symbol = &symbol_short!("ADMIN");
