//! Bank/card accounts this use case's transactions belong to.

use crate::money::CurrencyCode;
use crate::party::PersonRelationship;
use serde::{Deserialize, Serialize};

/// The kind of account, per the 2026-09-20 real-use-case session (log
/// `002`): two personal savings accounts, one business current account,
/// two credit cards. No dedicated family-farm-business account kind yet —
/// that account doesn't exist yet (owner plans to open one later); it's
/// added when it's real, not modeled speculatively.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountKind {
    PersonalSavings,
    BusinessCurrent,
    CreditCard,
}

/// Whether an account is in its originally-intended use, or repurposed.
/// The current business account was opened for a previous, unrelated
/// business and is now reused for the family-run business — modeled
/// `Legacy` so that history is visible rather than implied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountStatus {
    Active,
    Legacy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// Stable, human-assigned identifier (e.g. "owner-savings-1",
    /// "business-current-legacy"), not a database row id.
    pub slug: String,
    pub kind: AccountKind,
    pub status: AccountStatus,
    /// Whose account this is, e.g. `Owner` for the two personal savings
    /// accounts and the credit cards, `Mother` for the mother's own
    /// account.
    pub holder: PersonRelationship,
    pub currency: CurrencyCode,
    /// Free-text only for operator-facing context (e.g. "originally
    /// opened for a previous business, now used for family business
    /// expenses") — never used to carry data that should be a typed
    /// field.
    pub note: Option<String>,
}
