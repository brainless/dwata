use crate::email_ranking::multi_factor::{RankingContext, RankingFactor};
use crate::email_ranking::{
    contains_amount, contains_date, contains_financial_keyword, count_financial_keywords,
    find_amounts, FINANCIAL_KEYWORDS,
};
use shared_types::email::Email;

/// Factor: Financial content in email
pub struct FinancialContentFactor;

impl RankingFactor for FinancialContentFactor {
    fn name(&self) -> &str {
        "financial_content"
    }

    fn score_email(&self, email: &Email, _context: &RankingContext) -> f64 {
        let subject = email.subject.as_deref().unwrap_or("");
        let body_text = email.body_text.as_deref().unwrap_or("");
        let body_html = email.body_html.as_deref().unwrap_or("");

        let combined_text = format!("{} {} {}", subject, body_text, body_html);

        let mut score: f64 = 0.0;

        // Check for financial keywords (up to 40 points)
        let keyword_count = count_financial_keywords(&combined_text);
        score += (keyword_count as f64 * 5.0).min(40.0);

        // Check for amounts (up to 30 points)
        let amounts = find_amounts(&combined_text);
        if !amounts.is_empty() {
            score += 10.0 + (amounts.len() as f64 * 2.0).min(20.0);
        }

        // Check for dates (up to 20 points)
        if contains_date(&combined_text) {
            score += 20.0;
        }

        // Subject contains keywords (bonus 10 points)
        if contains_financial_keyword(subject) {
            score += 10.0;
        }

        score
    }
}

/// Check if an email meets the minimum criteria for financial content
pub fn meets_financial_criteria(email: &Email) -> bool {
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
