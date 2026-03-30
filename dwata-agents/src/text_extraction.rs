use chrono::{Datelike, NaiveDate};
use dateparser::parse as parse_datetime;
use regex::Regex;
use std::collections::{HashMap, HashSet};

/// Parses a raw amount string into a float value.
/// Captures the first numeric token: optional leading minus, digits with optional
/// comma-grouping, optional single decimal part. Handles "Rs.299.00", "INR 1,299.50", etc.
pub fn parse_amount(raw: &str) -> Option<f64> {
    let re = Regex::new(r"-?\d[\d,]*(?:\.\d+)?").unwrap();
    let m = re.find(raw)?;
    let cleaned = m.as_str().replace(',', "");
    cleaned.parse::<f64>().ok()
}

/// Parses a date string into a NaiveDate.
/// Tries multiple formats and date parsers.
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

/// Extract field values from an email by using a saved template as a positional anchor map.
///
/// For each `{{variable_name}}` placeholder in the template, find the fixed text immediately
/// before and after it (the "anchors"), then locate those anchors in the email and extract
/// whatever text sits between them. This is the extraction half of the reverse-template pipeline
/// and requires no LLM call.
pub fn extract_values_using_template(
    template_body: &str,
    variable_names: &[String],
    email_text: &str,
) -> HashMap<String, String> {
    let re = Regex::new(r"\{\{([a-zA-Z0-9_-]+)\}\}").unwrap();

    // Walk template once, building an ordered list of (var_name, left_segment, right_segment).
    let mut entries: Vec<(&str, &str, &str)> = Vec::new();
    let mut last_end = 0usize;

    for cap in re.captures_iter(template_body) {
        let full_match = cap.get(0).unwrap();
        let var_name = cap.get(1).unwrap().as_str();
        let left_segment = &template_body[last_end..full_match.start()];
        last_end = full_match.end();
        // right_segment will be filled after we know the next capture start
        entries.push((var_name, left_segment, ""));
    }
    // Fill right_segment for each entry using the left_segment of the next entry (or the tail).
    let tail = &template_body[last_end..];
    let entry_count = entries.len();
    let mut entries_with_right: Vec<(&str, &str, &str)> = entries
        .into_iter()
        .enumerate()
        .map(|(i, (var, left, _))| {
            let right = if i + 1 < entry_count {
                // placeholder: we re-derive from the raw template below
                ""
            } else {
                tail
            };
            (var, left, right)
        })
        .collect();

    // Re-derive right segments by scanning captures a second time (simpler than threading state).
    {
        let caps: Vec<_> = re.captures_iter(template_body).collect();
        for i in 0..entry_count {
            let right_start = caps[i].get(0).unwrap().end();
            let right_end = if i + 1 < entry_count {
                caps[i + 1].get(0).unwrap().start()
            } else {
                template_body.len()
            };
            entries_with_right[i].2 = &template_body[right_start..right_end];
        }
    }

    let var_name_set: HashSet<&str> = variable_names.iter().map(|s| s.as_str()).collect();
    let mut result = HashMap::new();
    // Cursor into email_text — each variable search starts from where the previous one ended,
    // preventing short anchors like " at " from matching an earlier occurrence.
    let mut email_cursor = 0usize;

    for (entry_idx, (var_name, left_seg, right_seg)) in entries_with_right.iter().enumerate() {
        if !var_name_set.contains(*var_name) {
            continue;
        }

        // Left anchor: up to 60 chars of the tail of the left segment (crosses line boundaries).
        // When left_seg is empty (adjacent placeholders like {{currency}}{{amount}}), walk back
        // through previous entries to find a non-empty left segment to anchor against.
        let left_trimmed = left_seg.trim_end();
        let effective_left = if left_trimmed.is_empty() {
            let mut found = "";
            for prev_idx in (0..entry_idx).rev() {
                let prev_left = entries_with_right[prev_idx].1.trim_end();
                if !prev_left.is_empty() {
                    found = prev_left;
                    break;
                }
            }
            found
        } else {
            left_trimmed
        };
        let left_anchor = {
            let char_count = effective_left.chars().count();
            if char_count <= 60 {
                effective_left
            } else {
                // Walk to the correct byte offset — slicing by bytes is unsafe on multibyte chars.
                let byte_offset = effective_left
                    .char_indices()
                    .nth(char_count - 60)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                &effective_left[byte_offset..]
            }
        };

        // Right anchor: first line of the right segment with at least 3 chars.
        // Short right anchors (e.g. just ".") cause false positives; using a longer line is safer.
        let right_anchor = right_seg
            .split('\n')
            .map(|l| l.trim_end())
            .find(|l| l.len() >= 3)
            .unwrap_or_else(|| right_seg.split('\n').next().unwrap_or("").trim_end());

        let search_slice = &email_text[email_cursor..];
        if let Some((raw_value, consumed)) =
            find_value_between_anchors(search_slice, left_anchor, right_anchor)
        {
            email_cursor += consumed;
            let processed = process_field_value(var_name, &raw_value);
            result.insert(var_name.to_string(), processed);
        }
    }

    result
}

/// Returns the extracted value and the byte offset into `email` just past the found value,
/// so the caller can advance a cursor and avoid re-matching earlier occurrences.
fn find_value_between_anchors(
    email: &str,
    left_anchor: &str,
    right_anchor: &str,
) -> Option<(String, usize)> {
    let value_start = if left_anchor.is_empty() {
        0
    } else {
        let pos = email.find(left_anchor)?;
        pos + left_anchor.len()
    };

    let search_text = &email[value_start..];

    let (value, consumed_after_start) = if right_anchor.is_empty() {
        let v = search_text.lines().next().unwrap_or("").trim().to_string();
        let c = v.len();
        (v, c)
    } else {
        let end = search_text.find(right_anchor)?;
        let v = search_text[..end].trim().to_string();
        (v, end)
    };

    if value.is_empty() {
        None
    } else {
        Some((value, value_start + consumed_after_start))
    }
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

/// Create a simplified email content structure from raw email parts.
/// Strips HTML and returns clean text suitable for processing.
pub fn simple_email_content(
    subject: Option<&str>,
    body_text: Option<&str>,
    body_html: Option<&str>,
) -> SimpleEmailContent {
    let subject = subject.unwrap_or("").to_string();

    let body = if let Some(text) = body_text {
        text.to_string()
    } else if let Some(html) = body_html {
        // Simple HTML to text conversion
        html_to_text(html)
    } else {
        String::new()
    };

    SimpleEmailContent { subject, body }
}

/// Simple email content structure
#[derive(Debug, Clone)]
pub struct SimpleEmailContent {
    pub subject: String,
    pub body: String,
}

/// Convert HTML to plain text (basic implementation)
fn html_to_text(html: &str) -> String {
    // Remove script and style tags with their content
    let re_script = Regex::new(r"<script[^>]*>[\s\S]*?</script>").unwrap();
    let re_style = Regex::new(r"<style[^>]*>[\s\S]*?</style>").unwrap();
    let text = re_script.replace_all(html, "");
    let text = re_style.replace_all(&text, "");

    // Replace common block elements with newlines
    let re_br = Regex::new(r"<br\s*/?>").unwrap();
    let text = re_br.replace_all(&text, "\n");

    let re_p = Regex::new(r"<p[^>]*>").unwrap();
    let text = re_p.replace_all(&text, "\n\n");

    let re_div = Regex::new(r"<div[^>]*>").unwrap();
    let text = re_div.replace_all(&text, "\n");

    // Remove all remaining HTML tags
    let re_tags = Regex::new(r"<[^>]+>").unwrap();
    let text = re_tags.replace_all(&text, "");

    // Decode common HTML entities
    let text = text.replace("&nbsp;", " ");
    let text = text.replace("&lt;", "<");
    let text = text.replace("&gt;", ">");
    let text = text.replace("&amp;", "&");
    let text = text.replace("&quot;", "\"");
    let text = text.replace("&#39;", "'");

    // Normalize whitespace
    let text = text
        .split('\n')
        .map(|line| line.trim())
        .collect::<Vec<_>>()
        .join("\n");

    // Remove excessive blank lines
    let re_empty = Regex::new(r"\n{3,}").unwrap();
    let text = re_empty.replace_all(&text, "\n\n");

    text.trim().to_string()
}
