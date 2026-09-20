//! First model-first, need-driven use-case crate for Dwata's restart.
//!
//! See `epics/002-transaction-type-family-first-use-case.md` for the epic
//! this crate implements, and `log/` for the dated working log required by
//! that epic's working method. `README.md` in this crate records the
//! schema-integration decision and the (still open) CLI-testing task.

pub mod account;
pub mod extractors;
pub mod money;
pub mod party;
pub mod party_registry;
pub mod transaction;

pub use account::{Account, AccountKind, AccountStatus};
pub use money::{CurrencyCode, Money};
pub use party::{OrganisationRole, Party, PersonRelationship};
pub use transaction::{
    BusinessContext, Transaction, TransactionDirection, TransactionKind, TransactionSource,
};
