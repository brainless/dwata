mod ics;
mod vcf;

pub use ics::{IcsParser, IcsParserConfig, TimezoneHandling};
pub use vcf::VcfParser;

use shared_types::{
    DataType, ExtractionError, ExtractionInput, ExtractionMethod, ExtractionResult, Extractor,
};

/// Main attachment parser extractor that handles ICS and VCF files
pub struct AttachmentParserExtractor {
    ics_parser: IcsParser,
    vcf_parser: VcfParser,
}

impl AttachmentParserExtractor {
    /// Create a new attachment parser with custom ICS configuration
    pub fn new(ics_config: IcsParserConfig) -> Self {
        Self {
            ics_parser: IcsParser::new(ics_config),
            vcf_parser: VcfParser::new(),
        }
    }

    /// Create a new attachment parser with default configuration
    pub fn with_defaults() -> Self {
        Self::new(IcsParserConfig::default())
    }
}

impl Extractor for AttachmentParserExtractor {
    fn extract(
        &self,
        input: &ExtractionInput,
    ) -> Result<Vec<ExtractionResult>, ExtractionError> {
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
                        eprintln!(
                            "Failed to parse ICS attachment {}: {}",
                            attachment.filename, e
                        );
                    }
                }
            } else if content_type.contains("text/vcard")
                || content_type.contains("text/x-vcard")
                || attachment.filename.ends_with(".vcf")
            {
                // Parse VCF file
                match self.vcf_parser.parse(&attachment.content, &attachment.filename) {
                    Ok(mut extracted) => results.append(&mut extracted),
                    Err(e) => {
                        eprintln!(
                            "Failed to parse VCF attachment {}: {}",
                            attachment.filename, e
                        );
                    }
                }
            }
        }

        Ok(results)
    }

    fn data_type(&self) -> DataType {
        DataType::Event // Primary target, but also extracts contacts
    }

    fn method(&self) -> ExtractionMethod {
        ExtractionMethod::AttachmentParsing
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::{extraction::{Attachment, EmailAddress, UserPreferences}};

    #[test]
    fn test_attachment_parser_integration() {
        let extractor = AttachmentParserExtractor::with_defaults();

        let ics_content = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
DTSTART:20260125T140000Z
SUMMARY:Team Meeting
END:VEVENT
END:VCALENDAR"#;

        let vcf_content = r#"BEGIN:VCARD
VERSION:3.0
FN:Jane Doe
EMAIL:jane@example.com
END:VCARD"#;

        let input = ExtractionInput {
            email_id: "test-123".to_string(),
            subject: "Meeting and Contact".to_string(),
            body_text: "Please see attached".to_string(),
            body_html: None,
            attachments: vec![
                Attachment {
                    filename: "meeting.ics".to_string(),
                    content_type: "text/calendar".to_string(),
                    content: ics_content.as_bytes().to_vec(),
                },
                Attachment {
                    filename: "contact.vcf".to_string(),
                    content_type: "text/vcard".to_string(),
                    content: vcf_content.as_bytes().to_vec(),
                },
            ],
            sender: EmailAddress {
                email: "sender@example.com".to_string(),
                name: Some("Sender".to_string()),
            },
            recipients: vec![],
            timestamp: chrono::Utc::now().timestamp(),
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
                default_task_priority: shared_types::TaskPriority::Medium,
                default_project_status: shared_types::ProjectStatus::Active,
                auto_link_threshold: 0.8,
            },
            target_data_type: DataType::Event,
            min_confidence: 0.5,
            max_results: None,
        };

        let results = extractor.extract(&input).unwrap();

        // Should extract 1 event and 1 contact
        assert_eq!(results.len(), 2);

        // Check event extraction
        let event_result = results.iter().find(|r| r.data_type == DataType::Event);
        assert!(event_result.is_some());
        assert!(event_result.unwrap().confidence > 0.9);

        // Check contact extraction
        let contact_result = results.iter().find(|r| r.data_type == DataType::Contact);
        assert!(contact_result.is_some());
        assert!(contact_result.unwrap().confidence > 0.9);
    }

    #[test]
    fn test_no_attachments() {
        let extractor = AttachmentParserExtractor::with_defaults();

        let input = ExtractionInput {
            email_id: "test-456".to_string(),
            subject: "No attachments".to_string(),
            body_text: "Just text".to_string(),
            body_html: None,
            attachments: vec![],
            sender: EmailAddress {
                email: "sender@example.com".to_string(),
                name: None,
            },
            recipients: vec![],
            timestamp: chrono::Utc::now().timestamp(),
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
                default_task_priority: shared_types::TaskPriority::Medium,
                default_project_status: shared_types::ProjectStatus::Active,
                auto_link_threshold: 0.8,
            },
            target_data_type: DataType::Event,
            min_confidence: 0.5,
            max_results: None,
        };

        let results = extractor.extract(&input).unwrap();
        assert_eq!(results.len(), 0);
    }
}
