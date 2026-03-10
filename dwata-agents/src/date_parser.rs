/// Deterministic date parsing from raw strings extracted from financial documents.
///
/// No LLM involvement — this is pure rule-based parsing using `dateparser`,
/// which handles the wide variety of date formats found in bills and receipts:
///   "15 Jan 2025", "January 15, 2025", "15/01/2025", "2025-01-15",
///   "15th January 2025", "Jan 15", "01-15-2025", etc.
///
/// Output is always ISO 8601 `YYYY-MM-DD` (TEXT in SQLite), suitable for
/// use with SQLite's date() / strftime() functions and lexicographic sorting.
///
/// ## Ambiguity note
///
/// Formats like "01/02/03" are ambiguous (DD/MM/YY vs MM/DD/YY). `dateparser`
/// defaults to MM/DD/YYYY (US convention). For senders known to use DD/MM
/// ordering (e.g., Indian utilities), callers should pre-process or add a
/// locale hint at the caller level — not here.
use chrono::Datelike;

/// Parse a raw date string into ISO 8601 `YYYY-MM-DD`.
///
/// Returns `None` if the input is empty, whitespace-only, or unparseable.
/// Never panics.
pub fn parse_to_iso(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // dateparser::parse returns a DateTime<Utc>. We extract the date part only
    // (bills are calendar dates, not instants).
    dateparser::parse(trimmed)
        .ok()
        .map(|dt| {
            let d = dt.date_naive();
            format!("{}-{:02}-{:02}", d.year(), d.month(), d.day())
        })
        // Reject obviously-wrong years (dateparser sometimes guesses current year
        // for bare day/month strings in ambiguous contexts).
        .filter(|iso| iso.starts_with("19") || iso.starts_with("20"))
}

/// Parse a raw date string into UTC epoch milliseconds.
///
/// For date-only values, timestamp is normalized to 00:00:00 UTC on that date.
pub fn parse_to_utc_timestamp_ms(raw: &str) -> Option<i64> {
    let iso = parse_to_iso(raw)?;
    let d = chrono::NaiveDate::parse_from_str(&iso, "%Y-%m-%d").ok()?;
    let dt = d.and_hms_opt(0, 0, 0)?.and_utc();
    Some(dt.timestamp_millis())
}

/// Parse a raw date string, returning both the raw value and the ISO 8601 result.
///
/// Designed for use when building `Bill` or `FinancialTransaction` structs:
/// ```
/// let (raw, parsed) = parse_date_pair("15 Jan 2025");
/// // raw   = Some("15 Jan 2025")
/// // parsed = Some("2025-01-15")
/// ```
pub fn parse_date_pair(raw: &str) -> (Option<String>, Option<String>) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return (None, None);
    }
    let parsed = parse_to_iso(trimmed);
    (Some(trimmed.to_string()), parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iso_format() {
        assert_eq!(parse_to_iso("2025-01-15"), Some("2025-01-15".to_string()));
    }

    #[test]
    fn test_month_name() {
        assert_eq!(
            parse_to_iso("January 15, 2025"),
            Some("2025-01-15".to_string())
        );
        assert_eq!(parse_to_iso("15 Jan 2025"), Some("2025-01-15".to_string()));
    }

    #[test]
    fn test_empty_returns_none() {
        assert_eq!(parse_to_iso(""), None);
        assert_eq!(parse_to_iso("   "), None);
    }

    #[test]
    fn test_date_pair() {
        let (raw, parsed) = parse_date_pair("15 Jan 2025");
        assert_eq!(raw, Some("15 Jan 2025".to_string()));
        assert_eq!(parsed, Some("2025-01-15".to_string()));
    }

    #[test]
    fn test_unparseable_returns_none() {
        assert_eq!(parse_to_iso("not a date"), None);
    }

    #[test]
    fn test_parse_to_utc_timestamp_ms() {
        let ts = parse_to_utc_timestamp_ms("15 Jan 2025").unwrap();
        let dt = chrono::DateTime::from_timestamp_millis(ts).unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2025-01-15");
    }
}
