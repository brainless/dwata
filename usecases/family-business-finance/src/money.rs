//! Decimal-safe money handling for this use case.
//!
//! `f64` is explicitly disallowed for money in this crate (see
//! `epics/002-transaction-type-family-first-use-case.md`). Amounts are
//! stored as integer minor units (e.g. paise, cents) alongside an ISO 4217
//! currency code, so arithmetic never loses precision to floating point.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A validated ISO 4217 currency code (e.g. `INR`, `USD`).
///
/// Kept as a validated newtype rather than an exhaustive enum: the set of
/// currency codes is a standard list, not a domain-specific variation this
/// use case needs to branch on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CurrencyCode([u8; 3]);

impl CurrencyCode {
    pub const INR: CurrencyCode = CurrencyCode([b'I', b'N', b'R']);

    pub fn new(code: &str) -> Option<Self> {
        let bytes = code.as_bytes();
        if bytes.len() == 3 && bytes.iter().all(|b| b.is_ascii_uppercase()) {
            Some(CurrencyCode([bytes[0], bytes[1], bytes[2]]))
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).expect("CurrencyCode is always valid ASCII")
    }
}

impl Default for CurrencyCode {
    fn default() -> Self {
        Self::INR
    }
}

impl fmt::Display for CurrencyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An amount of money in a specific currency, stored as integer minor
/// units (e.g. paise for INR, cents for USD) to avoid floating-point
/// rounding error in totals and balances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    minor_units: i64,
    currency: CurrencyCode,
}

impl Money {
    pub fn from_minor_units(minor_units: i64, currency: CurrencyCode) -> Self {
        Self {
            minor_units,
            currency,
        }
    }

    pub fn minor_units(&self) -> i64 {
        self.minor_units
    }

    pub fn currency(&self) -> CurrencyCode {
        self.currency
    }

    /// Add two amounts, returning `None` on currency mismatch or overflow
    /// rather than silently producing a wrong total.
    pub fn checked_add(&self, other: &Money) -> Option<Money> {
        if self.currency != other.currency {
            return None;
        }
        self.minor_units
            .checked_add(other.minor_units)
            .map(|minor_units| Money::from_minor_units(minor_units, self.currency))
    }
}
