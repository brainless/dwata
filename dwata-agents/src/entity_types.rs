use serde::{Deserialize, Deserializer};

// Re-export all extracted entity types from shared_types
pub use shared_types::{
    ConfirmEntitiesParams, ExtractedBill, ExtractedEntitiesParams, ExtractedEvent,
    ExtractedLocation, ExtractedOrder, ExtractedOrganisation, ExtractedPerson,
    ExtractedSubscription, ExtractedTransaction,
};

/// Deserialise a field that the schema declares as a string but small models
/// sometimes emit as a bare JSON number (e.g. `5.41` instead of `"5.41"`).
fn deserialize_number_or_string<'de, D>(d: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(d)?;
    Ok(match v {
        serde_json::Value::Null => None,
        serde_json::Value::String(s) => Some(s),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}

// ---------------------------------------------------------------------------
// Parsed value types (agent-side only, not sent to LLM)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ParsedValue {
    Int(i64),
    Float(f64),
    Date(chrono::NaiveDate),
    Text(String),
}

impl std::fmt::Display for ParsedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParsedValue::Int(v) => write!(f, "{} (int)", v),
            ParsedValue::Float(v) => write!(f, "{:.4} (float)", v),
            ParsedValue::Date(d) => write!(f, "{} (date)", d.format("%Y-%m-%d")),
            ParsedValue::Text(s) => write!(f, "{}", s),
        }
    }
}

pub fn parse_value(raw: &str) -> ParsedValue {
    let trimmed = raw.trim();
    if let Ok(v) = trimmed.parse::<i64>() {
        return ParsedValue::Int(v);
    }
    // Strip commas before float parse (e.g. "1,299.50")
    let no_comma = trimmed.replace(',', "");
    if let Ok(v) = no_comma.parse::<f64>() {
        return ParsedValue::Float(v);
    }
    if let Ok(dt) = dateparser::parse(trimmed) {
        return ParsedValue::Date(dt.date_naive());
    }
    ParsedValue::Text(trimmed.to_string())
}
