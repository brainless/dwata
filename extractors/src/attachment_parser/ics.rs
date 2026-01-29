use chrono::{DateTime, NaiveDateTime, Utc};
use ical::parser::ical::component::IcalEvent;
use shared_types::{
    DataType, ExtractionMethod, ExtractionResult, ExtractedEntity, ExtractedEvent, TextSource,
    TextSpan,
};
use std::collections::HashMap;

pub struct IcsParser {
    config: IcsParserConfig,
}

#[derive(Debug, Clone)]
pub struct IcsParserConfig {
    pub extract_attendees: bool,
    pub extract_location: bool,
    pub extract_recurrence: bool,
    pub timezone_handling: TimezoneHandling,
}

#[derive(Debug, Clone)]
pub enum TimezoneHandling {
    PreferUtc,
    PreserveOriginal,
    ConvertToUser,
}

impl Default for IcsParserConfig {
    fn default() -> Self {
        Self {
            extract_attendees: true,
            extract_location: true,
            extract_recurrence: false, // Future feature
            timezone_handling: TimezoneHandling::ConvertToUser,
        }
    }
}

impl IcsParser {
    pub fn new(config: IcsParserConfig) -> Self {
        Self { config }
    }

    pub fn parse(
        &self,
        content: &[u8],
        filename: &str,
        user_timezone: &str,
    ) -> anyhow::Result<Vec<ExtractionResult>> {
        let content_str = String::from_utf8(content.to_vec())?;
        let reader = ical::IcalParser::new(content_str.as_bytes());

        let mut results = Vec::new();

        for calendar in reader {
            let calendar = calendar?;

            for event in calendar.events {
                if let Some(result) = self.parse_event(&event, filename, user_timezone)? {
                    results.push(result);
                }
            }
        }

        Ok(results)
    }

    fn parse_event(
        &self,
        event: &IcalEvent,
        filename: &str,
        user_timezone: &str,
    ) -> anyhow::Result<Option<ExtractionResult>> {
        // Extract required fields
        let name = event
            .properties
            .iter()
            .find(|p| p.name == "SUMMARY")
            .and_then(|p| p.value.clone())
            .ok_or_else(|| anyhow::anyhow!("Event missing SUMMARY"))?;

        let date_str = event
            .properties
            .iter()
            .find(|p| p.name == "DTSTART")
            .and_then(|p| p.value.clone())
            .ok_or_else(|| anyhow::anyhow!("Event missing DTSTART"))?;

        // Parse date
        let date = self.parse_ics_date(&date_str, user_timezone)?;

        // Extract optional fields
        let description = event
            .properties
            .iter()
            .find(|p| p.name == "DESCRIPTION")
            .and_then(|p| p.value.clone());

        let location = if self.config.extract_location {
            event
                .properties
                .iter()
                .find(|p| p.name == "LOCATION")
                .and_then(|p| p.value.clone())
        } else {
            None
        };

        let attendees = if self.config.extract_attendees {
            self.extract_attendees(event)
        } else {
            Vec::new()
        };

        // Build extracted entity
        let entity = ExtractedEntity::Event(ExtractedEvent {
            name: name.clone(),
            description,
            date: date.to_rfc3339(),
            location,
            attendees,
            project_id: None, // Will be linked later
            task_id: None,    // Will be linked later
        });

        // Build confidence breakdown
        let mut confidence_breakdown = HashMap::new();
        confidence_breakdown.insert("name".to_string(), 1.0);
        confidence_breakdown.insert("date".to_string(), 1.0);
        confidence_breakdown.insert("location".to_string(), 0.95);
        confidence_breakdown.insert("attendees".to_string(), 0.90);

        // Create text span evidence
        let evidence = vec![TextSpan {
            source: TextSource::Attachment(filename.to_string()),
            start: 0,
            end: 0,
            text: format!("ICS Event: {}", name),
        }];

        Ok(Some(ExtractionResult {
            entity,
            data_type: DataType::Event,
            confidence: 0.98, // High confidence for valid ICS
            confidence_breakdown,
            method: ExtractionMethod::AttachmentParsing,
            evidence,
            relationships: Vec::new(), // Will be populated later
            requires_review: false,
            ambiguities: Vec::new(),
            extracted_at: Utc::now().timestamp(),
            extractor_version: env!("CARGO_PKG_VERSION").to_string(),
        }))
    }

    fn parse_ics_date(&self, date_str: &str, _user_timezone: &str) -> anyhow::Result<DateTime<Utc>> {
        // Parse ICS date format: 20260125T140000Z or 20260125T140000
        // Remove separators
        let date_str = date_str.replace([':', '-'], "");

        if date_str.ends_with('Z') {
            // UTC time
            let dt = NaiveDateTime::parse_from_str(&date_str[..15], "%Y%m%dT%H%M%S")?;
            Ok(DateTime::from_naive_utc_and_offset(dt, Utc))
        } else if date_str.len() >= 15 {
            // Local time - assume UTC for now
            let dt = NaiveDateTime::parse_from_str(&date_str[..15], "%Y%m%dT%H%M%S")?;
            Ok(DateTime::from_naive_utc_and_offset(dt, Utc))
        } else {
            Err(anyhow::anyhow!("Invalid date format: {}", date_str))
        }
    }

    fn extract_attendees(&self, event: &IcalEvent) -> Vec<String> {
        event
            .properties
            .iter()
            .filter(|p| p.name == "ATTENDEE")
            .filter_map(|p| {
                // Extract email from "mailto:email@example.com"
                p.value
                    .as_ref()
                    .and_then(|v| v.strip_prefix("mailto:"))
                    .map(|email| email.to_string())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_ICS: &str = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
UID:test@example.com
DTSTAMP:20260120T120000Z
DTSTART:20260125T140000Z
DTEND:20260125T150000Z
SUMMARY:Project Kickoff Meeting
DESCRIPTION:Initial planning session
LOCATION:Conference Room A
ATTENDEE:mailto:alice@example.com
ATTENDEE:mailto:bob@example.com
END:VEVENT
END:VCALENDAR"#;

    #[test]
    fn test_parse_ics() {
        let parser = IcsParser::new(IcsParserConfig::default());
        let results = parser
            .parse(SAMPLE_ICS.as_bytes(), "meeting.ics", "UTC")
            .unwrap();

        assert_eq!(results.len(), 1);

        let result = &results[0];
        assert_eq!(result.confidence, 0.98);
        assert_eq!(result.data_type, DataType::Event);

        match &result.entity {
            ExtractedEntity::Event(event) => {
                assert_eq!(event.name, "Project Kickoff Meeting");
                assert_eq!(event.location, Some("Conference Room A".to_string()));
                assert_eq!(event.attendees.len(), 2);
            }
            _ => panic!("Expected Event entity"),
        }
    }

    #[test]
    fn test_parse_ics_date() {
        let parser = IcsParser::new(IcsParserConfig::default());

        // Test UTC date
        let date = parser.parse_ics_date("20260125T140000Z", "UTC").unwrap();
        assert_eq!(date.format("%Y-%m-%d").to_string(), "2026-01-25");

        // Test local date
        let date = parser.parse_ics_date("20260125T140000", "UTC").unwrap();
        assert_eq!(date.format("%Y-%m-%d").to_string(), "2026-01-25");
    }
}
