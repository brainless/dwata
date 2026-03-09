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

pub fn extract_values_from_email(
    template: &str,
    email: &TemplateEmailContent,
) -> HashMap<String, String> {
    let email_text = format!(
        "Subject: {}\n---\n{}",
        email.subject.trim(),
        email.body.trim()
    );

    extract_by_sequential_search(template, &email_text)
}

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

fn extract_by_sequential_search(template: &str, email_text: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();
    let parsed = parse_template(template);

    if parsed.is_empty() {
        return values;
    }

    for item in &parsed {
        if let Some(value) = find_value_in_email(&item.name, &item.prefix, &item.suffix, email_text)
        {
            values.insert(item.name.clone(), value);
        }
    }

    values
}

fn find_value_in_email(
    _name: &str,
    prefix: &str,
    suffix: &str,
    email_text: &str,
) -> Option<String> {
    let clean_prefix = prefix.replace('\n', " ");
    let clean_suffix = suffix.replace('\n', " ");

    if clean_prefix.is_empty() && clean_suffix.is_empty() {
        return None;
    }

    if !clean_suffix.is_empty() {
        if let Some(prefix_pos) = email_text.find(&clean_prefix) {
            let suffix_match = &email_text[prefix_pos..];
            if let Some(suffix_pos) = suffix_match.find(&clean_suffix) {
                let value_start = clean_prefix.len();
                let value_end = suffix_pos;
                let value = &suffix_match[value_start..value_end];
                return Some(value.trim().to_string());
            }
        }
    } else {
        if let Some(prefix_pos) = email_text.find(&clean_prefix) {
            let after_prefix = prefix_pos + clean_prefix.len();
            let remaining = &email_text[after_prefix..];
            let end = remaining.find('\n').unwrap_or(remaining.len());
            let val = &remaining[..end];
            return Some(val.trim().to_string());
        }
    }

    None
}

fn clean_field_value(field_name: &str, value: &str) -> String {
    let cleaned = value.trim().to_string();
    match field_name {
        "document_reference" | "document-reference" => clean_document_reference(&cleaned),
        _ => cleaned,
    }
}

fn clean_document_reference(value: &str) -> String {
    static REFERENCE_PREFIXES: &[&str] = &[
        "BSNL Telephone No.",
        "BSNL Mobile No.",
        "BSNL Phone No.",
        "Airtel Telephone No.",
        "Airtel Mobile No.",
        "Airtel Phone No.",
        "Jio Telephone No.",
        "Jio Mobile No.",
        "Jio Phone No.",
        "Reliance Telephone No.",
        "Reliance Mobile No.",
        "Vodafone Telephone No.",
        "Vodafone Mobile No.",
        "Idea Telephone No.",
        "Idea Mobile No.",
        "Telephone No.",
        "Mobile No.",
        "Phone No.",
        "Phone:",
        "Tel No.",
        "Tel:",
        "Contact No.",
        "Contact:",
        "Reference No.",
        "Reference:",
        "Ref No.",
        "Ref:",
        "Bill No.",
        "Bill No:",
        "Invoice No.",
        "Invoice No:",
        "Account No.",
        "Account No:",
        "Account Number:",
        "Acct No.",
        "Acct No:",
    ];

    let upper = value.to_uppercase();
    for prefix in REFERENCE_PREFIXES {
        let upper_prefix = prefix.to_uppercase();
        if upper.starts_with(&upper_prefix) {
            let remaining = &value[prefix.len()..];
            let trimmed = remaining.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    static REFERENCE_PATTERNS: &[&str] = &[
        r"(?i)^tel(?:ephone)?\s*no\.?\s*",
        r"(?i)^mobile\s*no\.?\s*",
        r"(?i)^phone\s*no\.?\s*",
        r"(?i)^contact\s*no\.?\s*",
        r"(?i)^ref(?:erence)?\s*no\.?\s*",
        r"(?i)^bill\s*no\.?\s*",
        r"(?i)^invoice\s*no\.?\s*",
        r"(?i)^account\s*(?:no|number)?\.?\s*",
    ];

    let mut result = value.to_string();
    for pattern in REFERENCE_PATTERNS {
        if let Ok(re) = Regex::new(pattern) {
            result = re.replace(&result, "").to_string();
        }
    }

    result.trim().to_string()
}

fn make_flexible_regex(literal: &str) -> String {
    let mut result = String::new();
    for ch in literal.chars() {
        match ch {
            '\n' => result.push_str(r"(?:\s*)"),
            ' ' => result.push_str(r"(?:\s+)"),
            _ => {
                if ch.is_ascii_punctuation() && ch != '_' && ch != '-' {
                    result.push('\\');
                }
                result.push(ch);
            }
        }
    }
    result
}

struct ParsedItem {
    name: String,
    prefix: String,
    suffix: String,
}

fn parse_template(template: &str) -> Vec<ParsedItem> {
    let re = Regex::new(r"\{\{\s*(\w+)\s*\}\}").unwrap();
    let mut items = Vec::new();
    let mut last_end = 0usize;

    for cap in re.captures_iter(template) {
        let m = cap.get(0).unwrap();
        let name = cap.get(1).unwrap().as_str().to_string();

        let prefix = &template[last_end..m.start()];
        let suffix_start = m.end();
        let suffix = if suffix_start < template.len() {
            let next_brace = template[suffix_start..]
                .find("{{")
                .unwrap_or(template.len() - suffix_start);
            let raw_suffix = &template[suffix_start..suffix_start + next_brace];
            let truncated_suffix = if raw_suffix.len() > 100 {
                &raw_suffix[..100]
            } else {
                raw_suffix
            };
            truncated_suffix.to_string()
        } else {
            String::new()
        };

        items.push(ParsedItem {
            name,
            prefix: prefix.to_string(),
            suffix,
        });

        last_end = m.end();
    }

    items
}

pub fn is_valid_bill_value(field: &str, value: &str) -> bool {
    match field {
        "total-amount" | "total_amount" => parse_amount(value).is_some(),
        "currency" => is_currency_like(value),
        "issued-date"
        | "issued_date"
        | "due-date"
        | "due_date"
        | "billing-period-start"
        | "billing_period_start"
        | "billing-period-end"
        | "billing_period_end" => value.len() <= 60,
        "document-reference" | "document_reference" => value.len() >= 3 && value.len() <= 80,
        "service-identifier" | "service_identifier" => value.len() >= 3 && value.len() <= 120,
        _ => false,
    }
}

pub fn is_valid_txn_value(field: &str, value: &str) -> bool {
    match field {
        "amount" => parse_amount(value).is_some(),
        "currency" => is_currency_like(value),
        "transaction-date" | "transaction_date" => value.len() <= 60,
        "vendor-name" | "vendor_name" | "vendor" => !value.trim().is_empty(),
        "transaction-reference" | "transaction_reference" => value.len() >= 3 && value.len() <= 80,
        _ => false,
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
