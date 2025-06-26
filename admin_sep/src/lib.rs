#![no_std]
pub use contract_trait_macro::*;

mod administratable;
mod constructor;
mod upgradable;

pub use administratable::*;
pub use constructor::*;
pub use upgradable::*;
