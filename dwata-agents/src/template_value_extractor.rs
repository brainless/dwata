use chrono::{Datelike, NaiveDate};
use dateparser::parse as parse_datetime;
use regex::Regex;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TemplateEmailContent {
    pub subject: String,
    pub body: String,
}

pub use crate::llm_template_variable_extractor::types::TemplateVariable;

pub fn extract_values_from_email_with_values(
    variables: &[TemplateVariable],
    email: &TemplateEmailContent,
) -> HashMap<String, String> {
    let email_text = format!(
        "Subject: {}\n---\n{}",
        email.subject.trim(),
        email.body.trim()
    );

    let mut result = HashMap::new();
    for var in variables {
        // For currency, the LLM value may already be normalized (e.g. "INR") while the email
        // uses a different form (e.g. "Rs."). Normalize the LLM value directly instead of
        // searching for it in the email text.
        if var.variable_name == "currency" {
            let normalized = extract_currency_code(&var.value).unwrap_or_else(|| var.value.clone());
            result.insert(var.variable_name.clone(), normalized);
            continue;
        }
        if let Some(found) = find_value_by_search(&var.value, &email_text) {
            let processed = process_field_value(&var.variable_name, &found);
            result.insert(var.variable_name.clone(), processed);
        }
    }
    result
}

fn process_field_value(field_name: &str, value: &str) -> String {
    match field_name {
        "amount" | "total_amount" | "total-amount" => parse_amount(value)
            .map(|v| v.to_string())
            .unwrap_or_else(|| value.to_string()),
        "currency" => extract_currency_code(value).unwrap_or_else(|| value.to_string()),
        _ => value.to_string(),
    }
}

fn extract_currency_code(value: &str) -> Option<String> {
    let cleaned = value.trim();
    if cleaned.eq_ignore_ascii_case("Rs.") || cleaned.eq_ignore_ascii_case("Rs") {
        Some("INR".to_string())
    } else if cleaned.len() == 3 && cleaned.chars().all(|c| c.is_ascii_alphabetic()) {
        Some(cleaned.to_uppercase())
    } else if let Some(code) = cleaned.get(0..3) {
        Some(code.to_uppercase())
    } else {
        None
    }
}

fn find_value_by_search(value: &str, email_text: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }

    if let Some(pos) = email_text.find(value) {
        let before = &email_text[..pos];
        let after = &email_text[pos + value.len()..];

        let valid_before = !before.is_empty() && !before.ends_with('\n');
        let valid_after = !after.is_empty() && !after.starts_with('\n');

        if valid_before || valid_after {
            Some(value.to_string())
        } else {
            None
        }
    } else {
        None
    }
}

pub fn parse_amount(raw: &str) -> Option<f64> {
    // Capture the first numeric token: optional leading minus, digits with optional
    // comma-grouping, optional single decimal part. Handles "Rs.299.00", "INR 1,299.50", etc.
    let re = Regex::new(r"-?\d[\d,]*(?:\.\d+)?").unwrap();
    let m = re.find(raw)?;
    let cleaned = m.as_str().replace(',', "");
    cleaned.parse::<f64>().ok()
}

fn is_currency_like(raw: &str) -> bool {
    let s = raw.trim();
    if s.is_empty() || s.len() > 8 {
        return false;
    }
    let upper = s.to_ascii_uppercase();
    let is_iso_code = upper.len() == 3 && upper.chars().all(|c| c.is_ascii_alphabetic());
    let is_symbol = matches!(s, "$" | "€" | "£" | "¥" | "₹" | "₩" | "₽" | "₺" | "₫");
    is_iso_code || is_symbol
}

pub fn parse_date(raw: &str) -> Option<NaiveDate> {
    let mut normalized = raw.trim().trim_end_matches(['.', ',', ';', ':']).trim();
    if normalized.is_empty() || normalized.len() > 60 {
        return None;
    }
    if let Some(parsed) = parse_datetime(normalized).ok().map(|dt| dt.date_naive()) {
        return Some(parsed);
    }

    let upper = normalized.to_ascii_uppercase();
    let upper = upper.as_str();
    let explicit_formats = [
        "%d-%b-%Y", "%d-%B-%Y", "%d/%b/%Y", "%d/%B/%Y", "%d %b %Y", "%d %B %Y",
    ];
    for fmt in explicit_formats {
        if let Ok(date) = NaiveDate::parse_from_str(upper, fmt) {
            if (1900..=2100).contains(&date.year()) {
                return Some(date);
            }
        }
    }

    let collapsed = upper.replace(' ', "");
    for fmt in ["%d-%b-%Y", "%d-%B-%Y", "%d/%b/%Y", "%d/%B-%Y"] {
        if let Ok(date) = NaiveDate::parse_from_str(&collapsed, fmt) {
            if (1900..=2100).contains(&date.year()) {
                return Some(date);
            }
        }
    }

    normalized = upper;
    parse_datetime(normalized).ok().map(|dt| dt.date_naive())
}
