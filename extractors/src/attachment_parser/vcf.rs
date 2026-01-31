use chrono::Utc;
use shared_types::{
    DataType, ExtractedContact, ExtractedEntity, ExtractionMethod, ExtractionResult, TextSource,
    TextSpan,
};
use std::collections::HashMap;

pub struct VcfParser;

impl VcfParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, content: &[u8], filename: &str) -> anyhow::Result<Vec<ExtractionResult>> {
        let content_str = String::from_utf8(content.to_vec())?;
        let mut results = Vec::new();

        // Simple vCard parser - split by BEGIN:VCARD blocks
        let vcards: Vec<&str> = content_str.split("BEGIN:VCARD").skip(1).collect();

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
            profile_urls: vec![],
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
            extracted_at: Utc::now().timestamp(),
            extractor_version: env!("CARGO_PKG_VERSION").to_string(),
        }))
    }
}

impl Default for VcfParser {
    fn default() -> Self {
        Self::new()
    }
}

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
        let results = parser.parse(SAMPLE_VCF.as_bytes(), "contact.vcf").unwrap();

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

    #[test]
    fn test_parse_multiple_vcards() {
        let multi_vcf = r#"BEGIN:VCARD
VERSION:3.0
FN:Alice Smith
EMAIL:alice@example.com
END:VCARD
BEGIN:VCARD
VERSION:3.0
FN:Bob Jones
EMAIL:bob@example.com
END:VCARD"#;

        let parser = VcfParser::new();
        let results = parser.parse(multi_vcf.as_bytes(), "contacts.vcf").unwrap();

        assert_eq!(results.len(), 2);

        match &results[0].entity {
            ExtractedEntity::Contact(contact) => {
                assert_eq!(contact.name, "Alice Smith");
            }
            _ => panic!("Expected Contact entity"),
        }

        match &results[1].entity {
            ExtractedEntity::Contact(contact) => {
                assert_eq!(contact.name, "Bob Jones");
            }
            _ => panic!("Expected Contact entity"),
        }
    }
}
