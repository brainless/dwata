//! Extractor for HDFC bank statement CSV exports.
//!
//! Built against a real (gitignored, never-committed) sample export whose
//! columns are: `Date, Narration, Chq./Ref.No., Value Dt, Withdrawal Amt.,
//! Deposit Amt., Closing Balance`, with a blank leading line before the
//! header. See `log/003-hdfc-statement-first-extractor.md`.
//!
//! Only narration patterns actually seen in that sample are classified;
//! everything else — including patterns not yet seen — becomes
//! `TransactionKind::Unresolved` rather than being dropped or panicking,
//! per this crate's totality requirement.

use crate::money::{CurrencyCode, Money};
use crate::party::Party;
use crate::party_registry::PartyRegistry;
use crate::transaction::{
    BusinessContext, Transaction, TransactionDirection, TransactionKind, TransactionSource,
};
use chrono::NaiveDate;

/// Parse an HDFC statement CSV export into `Transaction`s.
///
/// `account_slug` identifies which of this use case's accounts the
/// statement belongs to — the file itself carries no account identity.
/// `registry` resolves names seen in narrations (e.g. a salary recipient)
/// to a known `Party`; an unrecognized name becomes `Party::Unknown`
/// rather than failing extraction.
pub fn parse_hdfc_statement_csv(
    csv_content: &str,
    account_slug: &str,
    registry: &PartyRegistry,
) -> Vec<Transaction> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(csv_content.as_bytes());

    let mut header_seen = false;
    let mut transactions = Vec::new();

    for result in reader.records() {
        let Ok(record) = result else { continue };

        if !header_seen {
            if record.get(0) == Some("Date") {
                header_seen = true;
            }
            continue;
        }

        let Some(date_field) = record.get(0) else {
            continue;
        };
        let Ok(occurred_on) = NaiveDate::parse_from_str(date_field.trim(), "%d/%m/%y") else {
            // Not a data row (e.g. a trailing footer/summary line).
            continue;
        };

        let narration = record.get(1).unwrap_or("").trim().to_string();
        let withdrawal = record.get(4).unwrap_or("").trim();
        let deposit = record.get(5).unwrap_or("").trim();

        let (direction, amount_minor_units) = match (
            parse_amount_to_minor_units(withdrawal),
            parse_amount_to_minor_units(deposit),
        ) {
            (Some(v), _) => (TransactionDirection::Debit, v),
            (None, Some(v)) => (TransactionDirection::Credit, v),
            (None, None) => continue, // no amount at all: not a real transaction row
        };

        let (kind, business_context) = classify_narration(&narration, registry);

        transactions.push(Transaction {
            account_slug: account_slug.to_string(),
            occurred_on,
            direction,
            amount: Money::from_minor_units(amount_minor_units, CurrencyCode::INR),
            business_context,
            kind,
            raw_description: narration,
            source: TransactionSource::BankCsv,
        });
    }

    transactions
}

/// Parse a plain decimal amount string (e.g. `"50000"`, `"1366157.9"`)
/// into integer paise, without going through floating point.
fn parse_amount_to_minor_units(field: &str) -> Option<i64> {
    if field.is_empty() {
        return None;
    }
    let mut parts = field.splitn(2, '.');
    let whole: i64 = parts.next()?.parse().ok()?;
    let frac_str = parts.next().unwrap_or("");
    if frac_str.len() > 2 {
        return None; // finer than paise: not a value this extractor expects
    }
    let frac: i64 = format!("{frac_str:0<2}").parse().ok()?;
    Some(whole * 100 + frac)
}

fn classify_narration(
    narration: &str,
    registry: &PartyRegistry,
) -> (TransactionKind, BusinessContext) {
    let upper = narration.to_uppercase();

    if upper.contains("SALARY") {
        let recipient = registry.resolve(narration).unwrap_or(Party::Unknown {
            raw_label: Some(narration.to_string()),
        });
        // Which business this salary draws against isn't determinable
        // from the row alone (open question, see log 003) — left
        // Unresolved rather than guessed.
        return (
            TransactionKind::Salary { recipient },
            BusinessContext::Unresolved,
        );
    }

    if upper.starts_with("INW ") && upper.contains("USD") {
        // Inward foreign remittance from a consulting client. The client
        // isn't named in the narration in this sample, so it's recorded
        // as an unresolved party rather than invented.
        return (
            TransactionKind::ConsultancyIncome {
                client: Party::Unknown {
                    raw_label: Some(narration.to_string()),
                },
            },
            BusinessContext::SoftwareConsultancy,
        );
    }

    // GST on the inward remittance and recurring SMS-alert bank fees:
    // explicitly folded into Unresolved for now (owner decision, log 003)
    // rather than given their own TransactionKind variant.
    (
        TransactionKind::Unresolved {
            raw_note: Some(narration.to_string()),
        },
        BusinessContext::Unresolved,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::party::PersonRelationship;

    fn test_registry() -> PartyRegistry {
        PartyRegistry::from_json_str(
            r#"{"entries": [
                {"name_fragment": "OWNER NAME", "party": {"Person": {"name": "Owner Name", "relationship": "Owner"}}},
                {"name_fragment": "SPOUSE NAME", "party": {"Person": {"name": "Spouse Name", "relationship": "Spouse"}}}
            ]}"#,
        )
        .expect("test registry JSON must be valid")
    }

    const HEADER_AND_ROWS: &str = "\
,,,,,,
Date,Narration,Chq./Ref.No.,Value Dt,Withdrawal Amt.,Deposit Amt.,Closing Balance
02/07/26,01061600000068-TPT-SALARY JUNE-OWNER NAME,0000000612353600,02/07/26,50000,,1174469.73
03/07/26,\"NEFT DR-BDBL0001468-SPOUSE NAME-NETBANK, MUM-HDFCH01098537395-SALARY JUNE\",HDFCH01098537395,03/07/26,50000,,1124469.73
22/07/26,INW 220726I049905985 USD2550.0@94.84,0000220726163333,22/07/26,,241842,1366311.73
22/07/26,220726I049905985 DPO2720352341222 CGST,0000220726163334,22/07/26,153.83,,1366157.9
25/07/26,MAYMAY26 INSTAALERTCHG 4 SMS-EPR2720567449492,EPR2720567449492,25/07/26,0.7,,1366003.37
";

    #[test]
    fn parses_all_data_rows_and_skips_header() {
        let transactions =
            parse_hdfc_statement_csv(HEADER_AND_ROWS, "business-current", &test_registry());
        assert_eq!(transactions.len(), 5);
    }

    #[test]
    fn resolves_salary_recipients_from_registry() {
        let transactions =
            parse_hdfc_statement_csv(HEADER_AND_ROWS, "business-current", &test_registry());

        let TransactionKind::Salary { recipient } = &transactions[0].kind else {
            panic!("expected Salary");
        };
        assert!(matches!(
            recipient,
            Party::Person {
                relationship: PersonRelationship::Owner,
                ..
            }
        ));

        let TransactionKind::Salary { recipient } = &transactions[1].kind else {
            panic!("expected Salary");
        };
        assert!(matches!(
            recipient,
            Party::Person {
                relationship: PersonRelationship::Spouse,
                ..
            }
        ));
    }

    #[test]
    fn classifies_fx_consultancy_income() {
        let transactions =
            parse_hdfc_statement_csv(HEADER_AND_ROWS, "business-current", &test_registry());
        let tx = &transactions[2];
        assert!(matches!(tx.kind, TransactionKind::ConsultancyIncome { .. }));
        assert_eq!(tx.direction, TransactionDirection::Credit);
        assert_eq!(tx.business_context, BusinessContext::SoftwareConsultancy);
    }

    #[test]
    fn folds_gst_and_sms_fees_into_unresolved_without_dropping_them() {
        let transactions =
            parse_hdfc_statement_csv(HEADER_AND_ROWS, "business-current", &test_registry());
        assert!(matches!(
            transactions[3].kind,
            TransactionKind::Unresolved { .. }
        ));
        assert!(matches!(
            transactions[4].kind,
            TransactionKind::Unresolved { .. }
        ));
    }

    #[test]
    fn unknown_salary_recipient_becomes_unknown_party_not_a_failure() {
        let csv = "\
Date,Narration,Chq./Ref.No.,Value Dt,Withdrawal Amt.,Deposit Amt.,Closing Balance
01/01/26,SALARY JULY-SOMEONE NOT IN REGISTRY,REF1,01/01/26,1000,,1000\n";
        let transactions = parse_hdfc_statement_csv(csv, "business-current", &test_registry());
        let TransactionKind::Salary { recipient } = &transactions[0].kind else {
            panic!("expected Salary");
        };
        assert!(matches!(recipient, Party::Unknown { .. }));
    }

    #[test]
    fn parses_paise_precision_without_float_rounding() {
        let transactions =
            parse_hdfc_statement_csv(HEADER_AND_ROWS, "business-current", &test_registry());
        // 153.83 -> 15383 paise, exactly.
        assert_eq!(transactions[3].amount.minor_units(), 15383);
    }
}
