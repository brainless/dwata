use regex::Regex;
use shared_types::email::Email;
use std::collections::HashSet;

pub mod conversation;
pub mod financial_content;
pub mod multi_factor;
pub mod sender;
pub mod tantivy_ranking;
pub mod temporal;
pub mod user_engagement;

/// Keywords that indicate financial/transaction content
pub const FINANCIAL_KEYWORDS: &[&str] = &[
    // Orders
    "order",
    "purchase",
    "confirmation",
    "order number",
    "order id",
    "order #",
    "purchase order",
    "po number",
    // Bills and Invoices
    "invoice",
    "bill",
    "billing",
    "payment due",
    "amount due",
    "total due",
    "balance due",
    "pay now",
    "remittance",
    // Receipts and Payments
    "receipt",
    "payment",
    "paid",
    "purchase receipt",
    "payment confirmation",
    "transaction",
    "completed",
    "successful",
    // Financial
    "statement",
    "subscription",
    "renewal",
    "charge",
    "charged",
    "debit",
    "credit",
    "refund",
    // E-commerce
    "shipping",
    "delivery",
    "tracking",
    "shipped",
    "delivered",
    "amazon",
    "ebay",
    "shopify",
    "stripe",
    "paypal",
    // Bank/Financial Institutions
    "wire transfer",
    "ach",
    "direct debit",
    "auto-pay",
    "autopay",
    "recurring",
];

/// Regex patterns for detecting amounts/currency
pub fn amount_patterns() -> Vec<Regex> {
    vec![
        // Currency symbols BEFORE numbers: $50, €100, £20, ¥1000, ₹500
        Regex::new(r"[$€£¥₹]\s*[\d,]+(?:\.\d{2})?").unwrap(),
        // Currency symbols AFTER numbers: 50$, 100€, etc. (less common but valid)
        Regex::new(r"[\d,]+(?:\.\d{2})?\s*[$€£¥₹]").unwrap(),
        // Indian Rupees text forms with word boundaries: Rs 500, Rs. 500, INR 500
        Regex::new(r"\b(?:Rs\.?|INR)\s*[\d,]+(?:\.\d{2})?\b").unwrap(),
        // Numbers followed by currency codes: 50 USD, 100 EUR, 500 INR
        Regex::new(r"\b[\d,]+(?:\.\d{2})?\s*(?:USD|EUR|GBP|JPY|INR|CAD|AUD|CHF|CNY|SGD|NZD|SEK|NOK|DKK|ZAR|MXN|BRL|KRW|RUB|TRY|AED|SAR)\b").unwrap(),
        // Currency codes followed by numbers: USD 50, EUR 100 (common in formal documents)
        Regex::new(r"\b(?:USD|EUR|GBP|JPY|INR|CAD|AUD|CHF|CNY|SGD|NZD|SEK|NOK|DKK|ZAR|MXN|BRL|KRW|RUB|TRY|AED|SAR)\s*[\d,]+(?:\.\d{2})?\b").unwrap(),
        // Amount keywords with optional currency symbol/code
        Regex::new(r"(?:amount|total|sum|price|cost|fee)[:\s]+(?:[$€£¥₹]|Rs\.?|INR|USD|EUR|GBP)?\s*[\d,]+(?:\.\d{2})?").unwrap(),
        // Standalone dollar amounts with word boundaries
        Regex::new(r"\$[\d,]+(?:\.\d{2})?\b").unwrap(),
        // Total/Subtotal lines with optional currency
        Regex::new(r"(?:total|subtotal|tax|shipping|discount)[:\s]+(?:[$€£¥₹]|Rs\.?|INR|USD|EUR|GBP)?\s*[\d,]+(?:\.\d{2})?").unwrap(),
    ]
}

/// Check if text contains a financial keyword
pub fn contains_financial_keyword(text: &str) -> bool {
    let text_lower = text.to_lowercase();
    FINANCIAL_KEYWORDS
        .iter()
        .any(|keyword| text_lower.contains(keyword))
}

/// Count how many financial keywords are found in text
pub fn count_financial_keywords(text: &str) -> usize {
    let text_lower = text.to_lowercase();
    FINANCIAL_KEYWORDS
        .iter()
        .filter(|keyword| text_lower.contains(*keyword))
        .count()
}

/// Check if text contains an amount/currency pattern
pub fn contains_amount(text: &str) -> bool {
    let patterns = amount_patterns();
    patterns.iter().any(|regex| regex.is_match(text))
}

/// Find all amounts in text
pub fn find_amounts(text: &str) -> Vec<String> {
    let patterns = amount_patterns();
    let mut amounts = HashSet::new();

    for pattern in &patterns {
        for capture in pattern.find_iter(text) {
            amounts.insert(capture.as_str().to_string());
        }
    }

    amounts.into_iter().collect()
}

/// Check if text contains a date using fuzzy date parsing
pub fn contains_date(text: &str) -> bool {
    // Use dateparser to check for any parseable dates

    // Try various date patterns
    let date_patterns = [
        // Full date patterns to try
        text,
        // Try first 50 chars, then 100 chars
        &text.chars().take(50).collect::<String>(),
        &text.chars().take(100).collect::<String>(),
    ];

    for pattern in &date_patterns {
        if let Ok(_) = dateparser::parse(pattern) {
            return true;
        }
    }

    // Try to find date-like substrings
    let date_regexes = [
        Regex::new(r"\b\d{1,2}[/-]\d{1,2}[/-]\d{2,4}\b").unwrap(), // 12/31/2023 or 31-12-2023
        Regex::new(r"\b\d{4}[/-]\d{1,2}[/-]\d{1,2}\b").unwrap(),   // 2023-12-31
        Regex::new(
            r"\b(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*\.?\s+\d{1,2},?\s+\d{4}\b",
        )
        .unwrap(), // December 31, 2023
        Regex::new(
            r"\b\d{1,2}\s+(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*\.?\s+\d{4}\b",
        )
        .unwrap(), // 31 December 2023
    ];

    for regex in &date_regexes {
        if regex.is_match(text) {
            // Verify with dateparser
            for capture in regex.find_iter(text) {
                if let Ok(_) = dateparser::parse(capture.as_str()) {
                    return true;
                }
            }
        }
    }

    false
}

/// Score an email based on financial relevance
/// Returns a score from 0-100, where higher is more likely a financial email
pub fn score_email(email: &Email) -> u32 {
    let mut score: u32 = 0;

    // Combine subject and body for analysis
    let subject = email.subject.as_deref().unwrap_or("");
    let body_text = email.body_text.as_deref().unwrap_or("");
    let body_html = email.body_html.as_deref().unwrap_or("");

    let combined_text = format!("{} {} {}", subject, body_text, body_html);

    // Check for financial keywords (up to 40 points)
    let keyword_count = count_financial_keywords(&combined_text);
    score += (keyword_count as u32 * 5).min(40);

    // Check for amounts (up to 30 points)
    let amounts = find_amounts(&combined_text);
    if !amounts.is_empty() {
        score += 10 + (amounts.len() as u32 * 2).min(20);
    }

    // Check for dates (up to 20 points)
    if contains_date(&combined_text) {
        score += 20;
    }

    // Subject contains keywords (bonus 10 points)
    if contains_financial_keyword(subject) {
        score += 10;
    }

    score
}

/// Check if an email meets the minimum criteria for financial content
pub fn is_likely_financial_email(email: &Email) -> bool {
    let subject = email.subject.as_deref().unwrap_or("");
    let body_text = email.body_text.as_deref().unwrap_or("");
    let body_html = email.body_html.as_deref().unwrap_or("");

    let combined_text = format!("{} {} {}", subject, body_text, body_html);

    // Must have at least one financial keyword
    if !contains_financial_keyword(&combined_text) {
        return false;
    }

    // Must have a date
    if !contains_date(&combined_text) {
        return false;
    }

    // Must have an amount
    if !contains_amount(&combined_text) {
        return false;
    }

    true
}

/// Result of email ranking
#[derive(Debug, Clone)]
pub struct RankedEmail {
    pub email_id: i64,
    pub credential_id: i64,
    pub from_address: String,
    pub subject: Option<String>,
    pub date_received: i64,
    pub score: u32,
    pub keywords_found: Vec<String>,
    pub amounts_found: Vec<String>,
    pub has_date: bool,
}

/// Rank a list of emails by financial relevance
pub fn rank_emails(emails: Vec<Email>) -> Vec<RankedEmail> {
    let mut ranked: Vec<RankedEmail> = emails
        .into_iter()
        .filter(|email| is_likely_financial_email(email))
        .map(|email| {
            let subject = email.subject.as_deref().unwrap_or("");
            let body_text = email.body_text.as_deref().unwrap_or("");
            let body_html = email.body_html.as_deref().unwrap_or("");
            let combined_text = format!("{} {} {}", subject, body_text, body_html);

            let keywords_found: Vec<String> = FINANCIAL_KEYWORDS
                .iter()
                .filter(|k| combined_text.to_lowercase().contains(*k))
                .map(|k| k.to_string())
                .collect();

            let amounts_found = find_amounts(&combined_text);

            RankedEmail {
                email_id: email.id,
                credential_id: email.credential_id,
                from_address: email.from_address.clone(),
                subject: email.subject.clone(),
                date_received: email.date_received,
                score: score_email(&email),
                keywords_found,
                amounts_found,
                has_date: contains_date(&combined_text),
            }
        })
        .collect();

    // Sort by date_received descending (most recent first)
    ranked.sort_by(|a, b| b.date_received.cmp(&a.date_received));

    ranked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_financial_keyword() {
        assert!(contains_financial_keyword("Your order has been confirmed"));
        assert!(contains_financial_keyword("Invoice #12345"));
        assert!(contains_financial_keyword("Payment received"));
        assert!(!contains_financial_keyword("How are you today?"));
    }

    #[test]
    fn test_contains_amount() {
        assert!(contains_amount("Total: $123.45"));
        assert!(contains_amount("Amount due: $1,234.00"));
        assert!(contains_amount("Price: 99.99 USD"));
        assert!(!contains_amount("Hello world"));
    }

    #[test]
    fn test_find_amounts() {
        let amounts = find_amounts("Total: $123.45 and tax: $5.67");
        assert!(!amounts.is_empty());
    }

    #[test]
    fn test_contains_date() {
        assert!(contains_date("Date: 12/31/2023"));
        assert!(contains_date("On January 15, 2024"));
        assert!(contains_date("2023-12-31"));
        assert!(!contains_date("Hello world"));
    }
}
