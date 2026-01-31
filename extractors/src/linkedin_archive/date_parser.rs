use chrono::{Datelike, NaiveDate};

pub fn parse_linkedin_date(date_str: &str) -> Option<i64> {
    if date_str.trim().is_empty() {
        return None;
    }

    if let Ok(date) = NaiveDate::parse_from_str(date_str, "%d %b %Y") {
        return Some(date.and_hms_opt(0, 0, 0)?.and_utc().timestamp());
    }

    if let Ok(date) = NaiveDate::parse_from_str(&format!("01 {}", date_str), "%d %b %Y") {
        return Some(date.and_hms_opt(0, 0, 0)?.and_utc().timestamp());
    }

    None
}

pub fn is_current_position(finished_on: &str) -> bool {
    finished_on.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_month_year() {
        let timestamp = parse_linkedin_date("Jan 2024");
        assert!(timestamp.is_some());

        let date = chrono::DateTime::from_timestamp(timestamp.unwrap(), 0).unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 1);
    }

    #[test]
    fn test_parse_full_date() {
        let timestamp = parse_linkedin_date("22 Jan 2026");
        assert!(timestamp.is_some());

        let date = chrono::DateTime::from_timestamp(timestamp.unwrap(), 0).unwrap();
        assert_eq!(date.year(), 2026);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 22);
    }

    #[test]
    fn test_empty_date() {
        assert!(parse_linkedin_date("").is_none());
        assert!(parse_linkedin_date("   ").is_none());
    }

    #[test]
    fn test_is_current() {
        assert!(is_current_position(""));
        assert!(!is_current_position("Dec 2023"));
    }
}
