#![no_std]
mod administratable;
pub use administratable::{Admin, AdminExt, Administratable, AdministratableExt};

mod upgradable;
pub use upgradable::*;