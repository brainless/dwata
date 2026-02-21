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
    let preprocessed_body = preprocess_body_for_extraction(&email.body);
    let email_text = format!(
        "Subject: {}\n---\n{}",
        email.subject.trim(),
        preprocessed_body
    );

    if let Some(values) = extract_values_from_full_template(template, &email_text) {
        return values;
    }

    let mut all_values = HashMap::new();
    let template_lines: Vec<&str> = template.lines().collect();
    for (idx, template_line) in template_lines.iter().enumerate() {
        if !template_line.contains("{{") {
            continue;
        }
        let next_template_line = template_lines.get(idx + 1).copied();
        for email_line in email_text.lines() {
            let extracted = extract_values_from_line(template_line, email_line, next_template_line);
            if !extracted.is_empty() {
                all_values.extend(extracted);
                break;
            }
        }
    }
    all_values
}

pub fn is_valid_bill_value(field: &str, value: &str) -> bool {
    match field {
        "total-amount" => parse_amount(value).is_some(),
        "currency" => is_currency_like(value),
        "issued-date" | "due-date" | "billing-period-start" | "billing-period-end" => {
            parse_date(value).is_some()
        }
        "document-reference" => is_reference_like(value),
        "service-identifier" => is_identifier_like(value),
        _ => false,
    }
}

pub fn is_valid_txn_value(field: &str, value: &str) -> bool {
    match field {
        "amount" => parse_amount(value).is_some(),
        "currency" => is_currency_like(value),
        "transaction-date" => parse_date(value).is_some(),
        "vendor" => !value.trim().is_empty(),
        "transaction-reference" => is_reference_like(value),
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
    for fmt in ["%d-%b-%Y", "%d-%B-%Y", "%d/%b/%Y", "%d/%B/%Y"] {
        if let Ok(date) = NaiveDate::parse_from_str(&collapsed, fmt) {
            if (1900..=2100).contains(&date.year()) {
                return Some(date);
            }
        }
    }

    normalized = upper;
    parse_datetime(normalized).ok().map(|dt| dt.date_naive())
}

fn extract_values_from_line(
    template_line: &str,
    email_line: &str,
    next_template_line: Option<&str>,
) -> HashMap<String, String> {
    let mut values = HashMap::new();
    let mut fixed_parts: Vec<&str> = Vec::new();
    let mut placeholders: Vec<&str> = Vec::new();
    let mut remaining = template_line;

    loop {
        if let Some(start) = remaining.find("{{") {
            fixed_parts.push(&remaining[..start]);
            remaining = &remaining[start + 2..];
            if let Some(end) = remaining.find("}}") {
                placeholders.push(remaining[..end].trim());
                remaining = &remaining[end + 2..];
            } else {
                break;
            }
        } else {
            fixed_parts.push(remaining);
            break;
        }
    }

    if placeholders.is_empty() {
        return values;
    }

    let mut pos = 0usize;
    for (i, ph) in placeholders.iter().enumerate() {
        let leading = if i < fixed_parts.len() {
            fixed_parts[i]
        } else {
            ""
        };
        if !leading.is_empty() {
            if let Some(p) = email_line[pos..].find(leading) {
                pos += p + leading.len();
            } else {
                return HashMap::new();
            }
        }

        let trailing = if i + 1 < fixed_parts.len() {
            fixed_parts[i + 1]
        } else {
            ""
        };
        let value_end = if !trailing.is_empty() {
            if let Some(p) = email_line[pos..].find(trailing) {
                pos + p
            } else {
                email_line.len()
            }
        } else {
            email_line.len()
        };

        let mut value = email_line[pos..value_end].trim().to_string();
        if trailing.is_empty() {
            if let Some(next_line_raw) = next_template_line {
                let next_line = next_line_raw.trim();
                if !next_line.is_empty() && !next_line.contains("{{") {
                    if let Some(idx) = value.find(next_line) {
                        value = value[..idx].trim().to_string();
                    }
                }
            }
        }
        if !value.is_empty() {
            values.insert(ph.to_string(), value);
        }
        pos = value_end;
    }

    values
}

fn extract_values_from_full_template(
    template: &str,
    email_text: &str,
) -> Option<HashMap<String, String>> {
    let ph_re = Regex::new(r"\{\{\s*([a-zA-Z0-9_]+)\s*\}\}").ok()?;
    let mut pattern = String::from("(?s)^");
    let mut names: Vec<String> = Vec::new();
    let mut last = 0usize;

    for caps in ph_re.captures_iter(template) {
        let m = caps.get(0)?;
        let name = caps.get(1)?.as_str().to_string();
        let fixed = &template[last..m.start()];
        pattern.push_str(&fixed_text_to_regex(fixed));
        pattern.push_str("(.*?)");
        names.push(name);
        last = m.end();
    }
    pattern.push_str(&fixed_text_to_regex(&template[last..]));
    pattern.push('$');

    let re = Regex::new(&pattern).ok()?;
    let caps = re.captures(email_text)?;
    let mut out = HashMap::new();
    for (i, name) in names.iter().enumerate() {
        if let Some(v) = caps.get(i + 1) {
            let value = v.as_str().trim();
            if !value.is_empty() {
                out.insert(name.clone(), value.to_string());
            }
        }
    }
    Some(out)
}

fn fixed_text_to_regex(fixed: &str) -> String {
    let mut out = String::new();
    let mut in_ws = false;
    for ch in fixed.chars() {
        if ch.is_whitespace() {
            if !in_ws {
                out.push_str(r"\s+");
                in_ws = true;
            }
            continue;
        }
        in_ws = false;
        match ch {
            '\\' | '^' | '$' | '.' | '|' | '?' | '*' | '+' | '(' | ')' | '[' | ']' | '{' | '}' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

fn preprocess_body_for_extraction(raw: &str) -> String {
    let mut lines: Vec<String> = raw
        .replace('\r', "")
        .lines()
        .map(normalize_line_for_extraction)
        .filter(|l| !l.is_empty())
        .filter(|l| !is_noise_line_for_extraction(l))
        .collect();

    let merged = lines.join(" ");
    lines = split_into_sentences_for_extraction(&merged)
        .into_iter()
        .map(|s| normalize_line_for_extraction(&s))
        .filter(|s| !s.is_empty())
        .filter(|s| !is_noise_line_for_extraction(s))
        .collect();

    lines.join("\n")
}

fn split_into_sentences_for_extraction(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        buf.push(ch);
        let prev = if i > 0 { Some(chars[i - 1]) } else { None };
        let next = if i + 1 < chars.len() {
            Some(chars[i + 1])
        } else {
            None
        };
        let is_sentence_punct = matches!(ch, '.' | '!' | '?');
        let punct_followed_by_space_or_end = next.is_none_or(|c| c.is_whitespace());
        let is_domain_or_decimal = ch == '.'
            && prev.is_some_and(|c| c.is_ascii_alphanumeric())
            && next.is_some_and(|c| c.is_ascii_alphanumeric());
        let boundary = is_sentence_punct && punct_followed_by_space_or_end && !is_domain_or_decimal;
        if boundary {
            let s = buf.trim();
            if !s.is_empty() {
                out.push(s.to_string());
            }
            buf.clear();
            while i + 1 < chars.len() && chars[i + 1].is_whitespace() {
                i += 1;
            }
        }
        i += 1;
    }
    let tail = buf.trim();
    if !tail.is_empty() {
        out.push(tail.to_string());
    }
    out
}

fn normalize_line_for_extraction(line: &str) -> String {
    let mut s = line
        .replace('\u{00A0}', " ")
        .replace('\u{FFFD}', " ")
        .trim()
        .to_string();
    if let Ok(re) = Regex::new(r"\s+") {
        s = re.replace_all(&s, " ").to_string();
    }
    s
}

fn is_noise_line_for_extraction(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    if lower.len() > 260 {
        return true;
    }
    if lower.chars().all(|c| !c.is_ascii_alphanumeric()) {
        return true;
    }
    lower.contains("confidentiality notice")
        || lower.contains("intended recipient")
        || lower.contains("unauthorized")
        || lower.contains("privileged information")
        || lower.contains("this is an auto generated email")
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

fn is_reference_like(raw: &str) -> bool {
    let s = raw.trim();
    if s.len() < 3 || s.len() > 80 {
        return false;
    }
    s.chars().any(|c| c.is_ascii_alphanumeric())
}

fn is_identifier_like(raw: &str) -> bool {
    let s = raw.trim();
    if s.len() < 3 || s.len() > 120 {
        return false;
    }
    s.chars().any(|c| c.is_ascii_alphanumeric())
}
