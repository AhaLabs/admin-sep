#![no_std]
mod administratable;
pub use administratable::{Admin, AdminExt, Administratable, AdministratableImpl};

mod upgradable;
pub use upgradable::*;