use chrono::{Datelike, NaiveDate};
use dateparser::parse as parse_datetime;
use regex::Regex;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TemplateEmailContent {
    pub subject: String,
    pub body: String,
}

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

fn extract_by_sequential_search(template: &str, email_text: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();
    let parsed = parse_template(template);

    if parsed.is_empty() {
        return values;
    }

    let mut search_from = 0usize;

    for item in &parsed {
        if search_from >= email_text.len() {
            break;
        }

        let remaining = &email_text[search_from..];

        let flexible_prefix = make_flexible_regex(&item.prefix);
        let flexible_suffix = if item.suffix.is_empty() {
            String::new()
        } else {
            make_flexible_regex(&item.suffix)
        };

        let pattern = if item.suffix.is_empty() {
            format!(r"{}(.+)", flexible_prefix)
        } else {
            format!(r"{}(.+?){}", flexible_prefix, flexible_suffix)
        };

        if let Ok(re) = Regex::new(&pattern) {
            if let Some(caps) = re.captures(remaining) {
                if let Some(m) = caps.get(1) {
                    let value = m.as_str().trim().to_string();
                    if !value.is_empty() {
                        values.insert(item.name.clone(), value);
                    }
                    search_from = search_from + m.end();
                }
            }
        }
    }

    values
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
            template[suffix_start..suffix_start + next_brace].to_string()
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
    let re = Regex::new(r"[^\d,\.\-]").ok()?;
    let cleaned = re.replace_all(raw, "").replace(',', "");
    if cleaned.is_empty() || cleaned == "-" || cleaned == "." || cleaned == "-." {
        return None;
    }
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
