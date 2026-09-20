//! The `Transaction` type family for this use case.
//!
//! Extraction into `Transaction` must be total: every source record
//! produces a `Transaction`, using `TransactionKind::Unresolved` and
//! `Party::Unknown` for ambiguity instead of failing to parse. See
//! `docs/03-type-driven-financial-extraction.md` for the pattern this
//! generalizes, and `epics/002-transaction-type-family-first-use-case.md`
//! for why extraction must never hard-fail here.

use crate::money::Money;
use crate::party::Party;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionDirection {
    Credit,
    Debit,
}

/// Which of the owner's businesses (or neither) a transaction belongs to.
/// Distinct from `TransactionKind` because the same kind of transaction
/// (e.g. an expense) can occur in more than one context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BusinessContext {
    /// The owner's software consultancy — majority income source.
    SoftwareConsultancy,
    /// The family-run pig/chicken farming business.
    FamilyFarm,
    /// Not business activity — ordinary family/personal spending.
    Personal,
    /// Extraction could not determine which context this belongs to.
    Unresolved,
}

/// The closed(-ish) set of transaction shapes seen in the owner's real
/// data as of the 2026-09-20 session (log `002`). Expected to grow as more
/// real samples are seen — prefer adding a variant here over adding a
/// free-text field to carry unmodeled meaning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionKind {
    /// Income from a consultancy client into the business account.
    ConsultancyIncome { client: Party },
    /// A salary payment out of the business account to the owner, a
    /// family member, or the business partner.
    Salary { recipient: Party },
    /// An expense paid toward the family-run farm business, by whoever
    /// drew the funds (owner, salaried family member, or partner).
    FamilyBusinessExpense { paid_by: Party, vendor: Party },
    /// Ordinary family/personal spending, not tied to either business.
    PersonalOrFamilyExpense { spender: Party, merchant: Party },
    /// A movement of funds between two of the owner's own accounts.
    AccountTransfer {
        counterparty_account_slug: Option<String>,
    },
    /// A charge on one of the two credit cards.
    CreditCardCharge { merchant: Party },
    /// Extraction could not classify this transaction. Carries the raw
    /// source description so it can be reviewed and reclassified rather
    /// than being dropped or failing extraction.
    Unresolved { raw_note: Option<String> },
}

/// Where a transaction record came from, per the source formats described
/// in the 2026-09-20 session: bank CSV exports, credit-card PDF bills, and
/// digital payment app CSV/Excel exports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionSource {
    BankCsv,
    CreditCardPdf,
    PaymentAppExport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Which account (by slug) this transaction happened on.
    pub account_slug: String,
    pub occurred_on: NaiveDate,
    pub direction: TransactionDirection,
    pub amount: Money,
    pub business_context: BusinessContext,
    pub kind: TransactionKind,
    /// The original, unmodified description/narration text from the
    /// source record. Always kept, regardless of how well `kind` was
    /// resolved, so nothing extraction did is unrecoverable.
    pub raw_description: String,
    pub source: TransactionSource,
}
