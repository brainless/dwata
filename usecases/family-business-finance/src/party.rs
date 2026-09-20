//! People and organisations tagged on transactions.
//!
//! `Party::Unknown` is the totality escape hatch: an extractor that sees a
//! counterparty it cannot resolve records it here (keeping the raw label
//! seen in the source) rather than dropping the transaction or failing.

use serde::{Deserialize, Serialize};

/// How a person relates to the owner's household/business, as described
/// in the 2026-09-20 real-use-case session (log `002`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PersonRelationship {
    /// The epic owner themself.
    Owner,
    Spouse,
    Mother,
    BusinessPartner,
    OtherFamily,
}

/// How an organisation relates to the owner's business activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrganisationRole {
    /// A client of the software consultancy business.
    ConsultancyClient,
    /// A vendor/supplier to the family-run farm business.
    FarmVendor,
    /// A general merchant (retail, utility, etc.) not otherwise classified.
    Merchant,
    Other,
}

/// A person or organisation involved in a transaction, or an unresolved
/// counterparty when the extractor cannot classify one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Party {
    Person {
        name: String,
        relationship: PersonRelationship,
    },
    Organisation {
        name: String,
        role: OrganisationRole,
    },
    /// Extraction could not resolve who/what this counterparty is. Keeps
    /// the raw label seen in the source (e.g. a bank statement narration
    /// fragment) so it can be reviewed and reclassified later.
    Unknown { raw_label: Option<String> },
}
