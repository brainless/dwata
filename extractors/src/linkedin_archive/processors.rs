use crate::linkedin_archive::date_parser::parse_linkedin_date;
use shared_types::{
    extraction::{
        ExtractedCompany, ExtractedContact, ExtractedEntity, ExtractedPosition, ProfileUrl,
    },
    DataType, ExtractionError, ExtractionMethod, ExtractionResult,
};
use std::collections::HashMap;

pub fn process_positions(
    records: &[HashMap<String, String>],
) -> Result<Vec<ExtractionResult>, ExtractionError> {
    let mut results = Vec::new();

    for record in records {
        let company_name = record.get("Company Name").map(|s| s.trim().to_string());
        let title = record.get("Title").map(|s| s.trim().to_string());

        if company_name.is_none() || title.is_none() {
            continue;
        }

        let company_name = company_name.unwrap();
        let title = title.unwrap();
        let description = record
            .get("Description")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let location = record
            .get("Location")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let started_on = record
            .get("Started On")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let finished_on = record
            .get("Finished On")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let _started_date = started_on.as_ref().and_then(|d| parse_linkedin_date(d));
        let _finished_date = finished_on.as_ref().and_then(|d| parse_linkedin_date(d));

        let company = ExtractedCompany {
            name: company_name.clone(),
            description: None,
            industry: None,
            location: location.clone(),
            website: None,
            linkedin_url: None,
        };

        results.push(ExtractionResult {
            entity: ExtractedEntity::Company(company),
            data_type: DataType::Company,
            confidence: 1.0,
            confidence_breakdown: HashMap::from([("source".to_string(), 1.0)]),
            method: ExtractionMethod::PatternBased,
            evidence: vec![],
            relationships: vec![],
            requires_review: false,
            ambiguities: vec![],
            extracted_at: chrono::Utc::now().timestamp(),
            extractor_version: "1.0.0".to_string(),
        });

        let is_current = finished_on.is_none();

        let position = ExtractedPosition {
            contact_name: String::new(),
            company_name,
            title,
            description,
            location,
            started_on,
            finished_on,
            is_current,
        };

        results.push(ExtractionResult {
            entity: ExtractedEntity::Position(position),
            data_type: DataType::Position,
            confidence: 1.0,
            confidence_breakdown: HashMap::from([("source".to_string(), 1.0)]),
            method: ExtractionMethod::PatternBased,
            evidence: vec![],
            relationships: vec![],
            requires_review: false,
            ambiguities: vec![],
            extracted_at: chrono::Utc::now().timestamp(),
            extractor_version: "1.0.0".to_string(),
        });
    }

    Ok(results)
}

pub fn process_connections(
    records: &[HashMap<String, String>],
) -> Result<Vec<ExtractionResult>, ExtractionError> {
    let mut results = Vec::new();

    for record in records {
        let first_name = record
            .get("First Name")
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let last_name = record
            .get("Last Name")
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let url = record
            .get("URL")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let email = record
            .get("Email Address")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let company = record
            .get("Company")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let _position = record
            .get("Position")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        if first_name.is_empty() && last_name.is_empty() {
            continue;
        }

        let full_name = format!("{} {}", first_name, last_name).trim().to_string();

        let mut profile_urls = Vec::new();
        if let Some(url) = url {
            profile_urls.push(ProfileUrl {
                url,
                link_type: "linkedin".to_string(),
            });
        }

        let contact = ExtractedContact {
            name: full_name,
            email,
            phone: None,
            organization: company,
            profile_urls,
        };

        results.push(ExtractionResult {
            entity: ExtractedEntity::Contact(contact),
            data_type: DataType::Contact,
            confidence: 1.0,
            confidence_breakdown: HashMap::from([("source".to_string(), 1.0)]),
            method: ExtractionMethod::PatternBased,
            evidence: vec![],
            relationships: vec![],
            requires_review: false,
            ambiguities: vec![],
            extracted_at: chrono::Utc::now().timestamp(),
            extractor_version: "1.0.0".to_string(),
        });
    }

    Ok(results)
}

pub fn process_invitations(
    records: &[HashMap<String, String>],
) -> Result<Vec<ExtractionResult>, ExtractionError> {
    let mut results = Vec::new();

    for record in records {
        let from = record
            .get("From")
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let to = record
            .get("To")
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let direction = record
            .get("Direction")
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        let inviter_url = record
            .get("inviterProfileUrl")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let invitee_url = record
            .get("inviteeProfileUrl")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let (contact_name, profile_url) = if direction == "OUTGOING" {
            (to, invitee_url)
        } else {
            (from, inviter_url)
        };

        if contact_name.is_empty() {
            continue;
        }

        let mut profile_urls = Vec::new();
        if let Some(url) = profile_url {
            profile_urls.push(ProfileUrl {
                url,
                link_type: "linkedin".to_string(),
            });
        }

        let contact = ExtractedContact {
            name: contact_name,
            email: None,
            phone: None,
            organization: None,
            profile_urls,
        };

        results.push(ExtractionResult {
            entity: ExtractedEntity::Contact(contact),
            data_type: DataType::Contact,
            confidence: 1.0,
            confidence_breakdown: HashMap::from([("source".to_string(), 1.0)]),
            method: ExtractionMethod::PatternBased,
            evidence: vec![],
            relationships: vec![],
            requires_review: false,
            ambiguities: vec![],
            extracted_at: chrono::Utc::now().timestamp(),
            extractor_version: "1.0.0".to_string(),
        });
    }

    Ok(results)
}
