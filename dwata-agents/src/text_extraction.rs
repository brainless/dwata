use chrono::{Datelike, NaiveDate};
use dateparser::parse as parse_datetime;
use regex::Regex;

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
