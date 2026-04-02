/// Classifies an email sender as a human Person or an Organisation.
///
/// Rules are applied in order; the first match wins.
/// If no rule fires the sender is assumed to be a Person.
#[derive(Debug, PartialEq, Eq)]
pub enum SenderKind {
    Person,
    Organisation,
}

/// Local-part prefixes that are almost never used by real humans.
const ORG_LOCAL_PARTS: &[&str] = &[
    "noreply",
    "no-reply",
    "no_reply",
    "donotreply",
    "do-not-reply",
    "do_not_reply",
    "notifications",
    "notification",
    "alerts",
    "alert",
    "newsletter",
    "newsletters",
    "mailer",
    "mailer-daemon",
    "postmaster",
    "daemon",
    "bounce",
    "bounces",
    "support",
    "helpdesk",
    "help",
    "info",
    "billing",
    "payments",
    "invoices",
    "orders",
    "updates",
    "update",
    "news",
    "team",
    "hello",
    "contact",
    "sales",
    "marketing",
    "jobs",
    "careers",
    "hr",
    "admin",
    "system",
    "automated",
    "robot",
    "bot",
    "service",
    "services",
    "customercare",
    "customer-care",
    "customer_care",
    "feedback",
    "care",
    "reply",
    "no.reply",
    "do.not.reply",
    "security",
    "privacy",
    "legal",
    "compliance",
    "abuse",
    "spam",
    "unsubscribe",
    "automailer",
    "auto",
    "accounts",
    "account",
    "verify",
    "verification",
    "confirm",
    "confirmation",
    "onboarding",
    "welcome",
    "hello",
    "digest",
    "report",
    "reports",
    "statements",
    "statement",
    "receipts",
    "receipt",
    "invoice",
    "transaction",
    "transactions",
];

/// Words that, when found as a standalone token in the display name,
/// indicate the sender is an organisation rather than a person.
const ORG_NAME_WORDS: &[&str] = &[
    // Legal suffixes
    "inc",
    "incorporated",
    "ltd",
    "limited",
    "llc",
    "llp",
    "corp",
    "corporation",
    "pvt",
    "plc",
    "gmbh",
    "ag",
    "sa",
    "srl",
    // Industry keywords
    "bank",
    "banking",
    "insurance",
    "insurer",
    "technologies",
    "technology",
    "solutions",
    "services",
    "group",
    "authority",
    "ministry",
    "department",
    "association",
    "foundation",
    "institute",
    "university",
    "college",
    "school",
    "hospital",
    "clinic",
    // Communication-intent words
    "team",
    "support",
    "helpdesk",
    "help",
    "care",
    "official",
    "alert",
    "alerts",
    "notification",
    "notifications",
    "newsletter",
    "update",
    "updates",
    "info",
    "news",
    "digest",
    "mailer",
    "noreply",
    "no-reply",
    // Common brand suffixes / patterns
    "instaalerts",
    "instabanker",
    "pay",
    // Government / institutional
    "government",
    "govt",
    "municipal",
    "revenue",
    "tax",
    "customs",
    "portal",
];

/// Returns `true` when `token` (already lowercased, punctuation stripped)
/// matches any entry in `ORG_NAME_WORDS`.
fn is_org_name_word(token: &str) -> bool {
    ORG_NAME_WORDS.contains(&token)
}

/// Classify a sender.
///
/// `name` is the RFC 2822 display name (e.g. `"HDFC Bank InstaAlerts"`).
/// `email_addr` is the bare email address (e.g. `"alerts@hdfcbank.bank.in"`).
pub fn classify_sender(name: &str, email_addr: &str) -> SenderKind {
    // ── Rule 1: org local-part ────────────────────────────────────────────────
    let local = email_addr.split('@').next().unwrap_or("").to_lowercase();

    if ORG_LOCAL_PARTS.contains(&local.as_str()) {
        return SenderKind::Organisation;
    }

    // ── Rule 2: org keyword in display name (word-level, case-insensitive) ───
    let name_lower = name.to_lowercase();
    for token in name_lower.split_whitespace() {
        // strip leading/trailing punctuation so "Inc." → "inc", "(Ltd)" → "ltd"
        let token = token.trim_matches(|c: char| !c.is_alphanumeric());
        if is_org_name_word(token) {
            return SenderKind::Organisation;
        }
    }

    // ── Rule 3: too many words ────────────────────────────────────────────────
    // Human names almost never exceed 5 words; long multi-word names are
    // almost always organisation/brand names.
    let word_count = name.split_whitespace().count();
    if word_count > 5 {
        return SenderKind::Organisation;
    }

    // ── Default ───────────────────────────────────────────────────────────────
    SenderKind::Person
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_org_by_local_part() {
        assert_eq!(
            classify_sender("HDFC Bank InstaAlerts", "alerts@hdfcbank.bank.in"),
            SenderKind::Organisation
        );
        assert_eq!(
            classify_sender("Netflix", "noreply@netflix.com"),
            SenderKind::Organisation
        );
        assert_eq!(
            classify_sender("GitHub", "support@github.com"),
            SenderKind::Organisation
        );
    }

    #[test]
    fn test_org_by_name_keyword() {
        assert_eq!(
            classify_sender("HDFC Bank", "hdfc@hdfcbank.com"),
            SenderKind::Organisation
        );
        assert_eq!(
            classify_sender("Acme Corp", "john@acme.com"),
            SenderKind::Organisation
        );
        assert_eq!(
            classify_sender("Product Team", "product@startup.io"),
            SenderKind::Organisation
        );
    }

    #[test]
    fn test_person() {
        assert_eq!(
            classify_sender("John Smith", "john.smith@example.com"),
            SenderKind::Person
        );
        assert_eq!(
            classify_sender("Alice Johnson", "alice@company.com"),
            SenderKind::Person
        );
        assert_eq!(
            classify_sender("Priya Sharma", "priya.s@startup.io"),
            SenderKind::Person
        );
    }

    #[test]
    fn test_org_by_word_count() {
        assert_eq!(
            classify_sender(
                "Your Acme Delivery Confirmation Email Service",
                "orders@acme.com"
            ),
            SenderKind::Organisation
        );
    }
}
