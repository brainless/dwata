# Implement Attachment Parser Extractor

## Objective

Implement the attachment-based extractor focusing on structured data from email attachments. This provides the highest confidence extractions since attachments like ICS (calendar) and VCF (vCard) files contain well-structured, parseable data.

**Confidence Target**: 0.95+ for valid attachment formats

## Dependencies

**Blocked by**: Task #1 - Extraction Framework Foundation

Requires the `Extractor` trait and common types from shared-types.

## Background

Email attachments often contain highly structured data:
- **ICS files**: Calendar invitations with event details (name, date, time, location, attendees, recurrence)
- **VCF files**: Contact information (name, email, phone, organization, address)
- **Future**: PDF documents, forwarded emails, Word documents

Parsing these attachments provides high-confidence entity extraction because the data format is standardized and unambiguous.

## Implementation Plan

### Phase 1: Setup Extractors Crate

#### 1.1 Create extractors crate

```bash
cargo new --lib extractors
```

#### 1.2 Add to workspace

**File: `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = [
    "dwata-api",
    "shared-types",
    "extractors",  # NEW
]
```

#### 1.3 Configure extractors crate

**File: `extractors/Cargo.toml`**

```toml
[package]
name = "extractors"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
shared-types = { path = "../shared-types" }
chrono.workspace = true
serde.workspace = true
serde_json.workspace = true
anyhow.workspace = true
thiserror.workspace = true

# Attachment parsing
ical = "0.10"
mail-parser = "0.9"
```

### Phase 2: ICS (Calendar) Parser

**File: `extractors/src/attachment_parser/ics.rs`**

```rust
use chrono::{DateTime, Utc};
use ical::parser::ical::component::IcalCalendar;
use shared_types::{
    DataType, ExtractionMethod, ExtractionResult, ExtractedEntity, ExtractedEvent,
    TextSource, TextSpan, Relationship, RelationType, EntityRef,
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
            extract_recurrence: false,  // Future feature
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
        event: &ical::parser::ical::component::IcalEvent,
        filename: &str,
        user_timezone: &str,
    ) -> anyhow::Result<Option<ExtractionResult>> {
        // Extract required fields
        let name = event.properties.iter()
            .find(|p| p.name == "SUMMARY")
            .and_then(|p| p.value.clone())
            .ok_or_else(|| anyhow::anyhow!("Event missing SUMMARY"))?;

        let date = event.properties.iter()
            .find(|p| p.name == "DTSTART")
            .and_then(|p| p.value.clone())
            .ok_or_else(|| anyhow::anyhow!("Event missing DTSTART"))?;

        // Parse date
        let date = self.parse_ics_date(&date, user_timezone)?;

        // Extract optional fields
        let description = event.properties.iter()
            .find(|p| p.name == "DESCRIPTION")
            .and_then(|p| p.value.clone());

        let location = if self.config.extract_location {
            event.properties.iter()
                .find(|p| p.name == "LOCATION")
                .and_then(|p| p.value.clone())
        } else {
            None
        };

        let attendees = if self.config.extract_attendees {
            self.extract_attendees(&event)
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
            project_id: None,  // Will be linked later
            task_id: None,     // Will be linked later
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
            end: content_str.len(),
            text: format!("ICS Event: {}", name),
        }];

        Ok(Some(ExtractionResult {
            entity,
            data_type: DataType::Event,
            confidence: 0.98,  // High confidence for valid ICS
            confidence_breakdown,
            method: ExtractionMethod::AttachmentParsing,
            evidence,
            relationships: Vec::new(),  // Will be populated later
            requires_review: false,
            ambiguities: Vec::new(),
            extracted_at: Utc::now(),
            extractor_version: env!("CARGO_PKG_VERSION").to_string(),
        }))
    }

    fn parse_ics_date(&self, date_str: &str, _user_timezone: &str) -> anyhow::Result<DateTime<Utc>> {
        // Parse ICS date format: 20260125T140000Z or 20260125T140000
        // Simplified parser - use proper ICS date parsing library in production
        let date_str = date_str.replace(":", "").replace("-", "");

        if date_str.ends_with('Z') {
            // UTC time
            let dt = chrono::NaiveDateTime::parse_from_str(
                &date_str[..15],
                "%Y%m%dT%H%M%S"
            )?;
            Ok(DateTime::from_naive_utc_and_offset(dt, Utc))
        } else {
            // Local time - assume UTC for now
            let dt = chrono::NaiveDateTime::parse_from_str(
                date_str,
                "%Y%m%dT%H%M%S"
            )?;
            Ok(DateTime::from_naive_utc_and_offset(dt, Utc))
        }
    }

    fn extract_attendees(&self, event: &ical::parser::ical::component::IcalEvent) -> Vec<String> {
        event.properties.iter()
            .filter(|p| p.name == "ATTENDEE")
            .filter_map(|p| {
                // Extract email from "mailto:email@example.com"
                p.value.as_ref()
                    .and_then(|v| v.strip_prefix("mailto:"))
                    .map(|email| email.to_string())
            })
            .collect()
    }
}
```

### Phase 3: VCF (vCard) Parser

**File: `extractors/src/attachment_parser/vcf.rs`**

```rust
use shared_types::{
    DataType, ExtractionMethod, ExtractionResult, ExtractedEntity, ExtractedContact,
    TextSource, TextSpan,
};
use std::collections::HashMap;

pub struct VcfParser;

impl VcfParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse(
        &self,
        content: &[u8],
        filename: &str,
    ) -> anyhow::Result<Vec<ExtractionResult>> {
        let content_str = String::from_utf8(content.to_vec())?;
        let mut results = Vec::new();

        // Simple vCard parser - split by BEGIN:VCARD blocks
        let vcards: Vec<&str> = content_str
            .split("BEGIN:VCARD")
            .skip(1)  // Skip empty first split
            .collect();

        for vcard in vcards {
            if let Some(result) = self.parse_vcard(vcard, filename)? {
                results.push(result);
            }
        }

        Ok(results)
    }

    fn parse_vcard(&self, vcard: &str, filename: &str) -> anyhow::Result<Option<ExtractionResult>> {
        let mut name = None;
        let mut email = None;
        let mut phone = None;
        let mut organization = None;

        for line in vcard.lines() {
            let line = line.trim();

            if line.starts_with("FN:") {
                name = Some(line[3..].to_string());
            } else if line.starts_with("EMAIL") {
                // Handle EMAIL;TYPE=work:email@example.com
                if let Some(value) = line.split(':').nth(1) {
                    email = Some(value.to_string());
                }
            } else if line.starts_with("TEL") {
                if let Some(value) = line.split(':').nth(1) {
                    phone = Some(value.to_string());
                }
            } else if line.starts_with("ORG:") {
                organization = Some(line[4..].to_string());
            }
        }

        let name = name.ok_or_else(|| anyhow::anyhow!("vCard missing FN (Full Name)"))?;

        let entity = ExtractedEntity::Contact(ExtractedContact {
            name: name.clone(),
            email,
            phone,
            organization,
        });

        let mut confidence_breakdown = HashMap::new();
        confidence_breakdown.insert("name".to_string(), 1.0);
        confidence_breakdown.insert("email".to_string(), 0.98);
        confidence_breakdown.insert("phone".to_string(), 0.95);

        let evidence = vec![TextSpan {
            source: TextSource::Attachment(filename.to_string()),
            start: 0,
            end: vcard.len(),
            text: format!("vCard: {}", name),
        }];

        Ok(Some(ExtractionResult {
            entity,
            data_type: DataType::Contact,
            confidence: 0.97,
            confidence_breakdown,
            method: ExtractionMethod::AttachmentParsing,
            evidence,
            relationships: Vec::new(),
            requires_review: false,
            ambiguities: Vec::new(),
            extracted_at: chrono::Utc::now(),
            extractor_version: env!("CARGO_PKG_VERSION").to_string(),
        }))
    }
}
```

### Phase 4: Main Attachment Parser

**File: `extractors/src/attachment_parser/mod.rs`**

```rust
mod ics;
mod vcf;

pub use ics::{IcsParser, IcsParserConfig, TimezoneHandling};
pub use vcf::VcfParser;

use shared_types::{
    Attachment, ExtractionInput, ExtractionResult, ExtractionError,
    DataType, ExtractionMethod, Extractor,
};

pub struct AttachmentParserExtractor {
    ics_parser: IcsParser,
    vcf_parser: VcfParser,
}

impl AttachmentParserExtractor {
    pub fn new(ics_config: IcsParserConfig) -> Self {
        Self {
            ics_parser: IcsParser::new(ics_config),
            vcf_parser: VcfParser::new(),
        }
    }
}

impl Extractor for AttachmentParserExtractor {
    fn extract(&self, input: &ExtractionInput) -> Result<Vec<ExtractionResult>, ExtractionError> {
        let mut results = Vec::new();

        for attachment in &input.attachments {
            let content_type = attachment.content_type.to_lowercase();

            if content_type.contains("text/calendar") || attachment.filename.ends_with(".ics") {
                // Parse ICS file
                match self.ics_parser.parse(
                    &attachment.content,
                    &attachment.filename,
                    &input.user_timezone,
                ) {
                    Ok(mut extracted) => results.append(&mut extracted),
                    Err(e) => {
                        eprintln!("Failed to parse ICS attachment {}: {}", attachment.filename, e);
                    }
                }
            } else if content_type.contains("text/vcard") || attachment.filename.ends_with(".vcf") {
                // Parse VCF file
                match self.vcf_parser.parse(&attachment.content, &attachment.filename) {
                    Ok(mut extracted) => results.append(&mut extracted),
                    Err(e) => {
                        eprintln!("Failed to parse VCF attachment {}: {}", attachment.filename, e);
                    }
                }
            }
        }

        Ok(results)
    }

    fn data_type(&self) -> DataType {
        DataType::Event  // Primary target, but also extracts contacts
    }

    fn method(&self) -> ExtractionMethod {
        ExtractionMethod::AttachmentParsing
    }
}
```

### Phase 5: Library Export

**File: `extractors/src/lib.rs`**

```rust
pub mod attachment_parser;

pub use attachment_parser::{
    AttachmentParserExtractor,
    IcsParserConfig,
    TimezoneHandling,
};
```

## Testing

### Unit Tests

**File: `extractors/src/attachment_parser/ics.rs`**

Add at bottom:

```rust
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
        let results = parser.parse(
            SAMPLE_ICS.as_bytes(),
            "meeting.ics",
            "UTC",
        ).unwrap();

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
}
```

**File: `extractors/src/attachment_parser/vcf.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_VCF: &str = r#"BEGIN:VCARD
VERSION:3.0
FN:John Doe
EMAIL;TYPE=work:john@example.com
TEL;TYPE=cell:+1234567890
ORG:Acme Corp
END:VCARD"#;

    #[test]
    fn test_parse_vcf() {
        let parser = VcfParser::new();
        let results = parser.parse(
            SAMPLE_VCF.as_bytes(),
            "contact.vcf",
        ).unwrap();

        assert_eq!(results.len(), 1);

        let result = &results[0];
        assert_eq!(result.confidence, 0.97);

        match &result.entity {
            ExtractedEntity::Contact(contact) => {
                assert_eq!(contact.name, "John Doe");
                assert_eq!(contact.email, Some("john@example.com".to_string()));
                assert_eq!(contact.organization, Some("Acme Corp".to_string()));
            }
            _ => panic!("Expected Contact entity"),
        }
    }
}
```

### Integration Test

**File: `extractors/tests/attachment_parser_test.rs`**

```rust
use extractors::{AttachmentParserExtractor, IcsParserConfig};
use shared_types::*;
use chrono::Utc;

#[test]
fn test_attachment_parser_integration() {
    let extractor = AttachmentParserExtractor::new(IcsParserConfig::default());

    let input = ExtractionInput {
        email_id: "test-123".to_string(),
        subject: "Meeting Invitation".to_string(),
        body_text: "Please see attached".to_string(),
        body_html: None,
        attachments: vec![
            Attachment {
                filename: "meeting.ics".to_string(),
                content_type: "text/calendar".to_string(),
                content: include_bytes!("fixtures/sample.ics").to_vec(),
            }
        ],
        sender: EmailAddress {
            email: "sender@example.com".to_string(),
            name: Some("Sender".to_string()),
        },
        recipients: vec![],
        timestamp: Utc::now(),
        thread_id: None,
        in_reply_to: None,
        extracted_entities: vec![],
        existing_projects: vec![],
        existing_tasks: vec![],
        existing_contacts: vec![],
        user_timezone: "UTC".to_string(),
        user_language: "en".to_string(),
        user_preferences: UserPreferences {
            date_format: "YYYY-MM-DD".to_string(),
            default_task_priority: TaskPriority::Medium,
            default_project_status: ProjectStatus::Active,
            auto_link_threshold: 0.8,
        },
        target_data_type: DataType::Event,
        min_confidence: 0.5,
        max_results: None,
    };

    let results = extractor.extract(&input).unwrap();
    assert!(!results.is_empty());
    assert!(results[0].confidence > 0.9);
}
```

### Run Tests

```bash
cd extractors
cargo test
```

## Next Steps

1. Add PDF parsing support for project documents
2. Add image OCR for scanned documents
3. Implement recurrence parsing for recurring events
4. Add timezone conversion utilities
5. Create sample fixtures for testing

## References

- ICS (iCalendar) specification: RFC 5545
- vCard specification: RFC 6350
- ical crate: https://crates.io/crates/ical
