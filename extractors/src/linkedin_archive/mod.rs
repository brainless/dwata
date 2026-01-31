mod csv_parser;
mod date_parser;
mod processors;

pub mod linkedin_metadata {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct ConnectionMetadata {
        pub first_name: String,
        pub last_name: String,
        pub connected_on: String,
        pub company_at_connection: Option<String>,
        pub position_at_connection: Option<String>,
        pub email_address: Option<String>,
        pub url: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    pub struct InvitationMetadata {
        pub from: String,
        pub to: String,
        pub sent_at: String,
        pub direction: String,
        pub message: Option<String>,
        pub inviter_profile_url: Option<String>,
        pub invitee_profile_url: Option<String>,
    }
}

use csv_parser::CsvParser;
use processors::{process_connections, process_invitations, process_positions};
use shared_types::{
    DataType, ExtractionError, ExtractionInput, ExtractionMethod, ExtractionResult, Extractor,
};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct LinkedInArchiveExtractor {
    csv_parser: CsvParser,
}

impl LinkedInArchiveExtractor {
    pub fn new() -> Self {
        Self {
            csv_parser: CsvParser::new(),
        }
    }

    pub fn process_archive(
        &self,
        archive_path: &str,
        files_to_process: &[String],
    ) -> Result<Vec<ExtractionResult>, ExtractionError> {
        let archive_dir = Path::new(archive_path);

        if !archive_dir.exists() {
            return Err(ExtractionError::InvalidInput(
                "Archive directory does not exist".to_string(),
            ));
        }

        let mut all_results = Vec::new();

        for filename in files_to_process {
            let file_path = archive_dir.join(filename);

            if !file_path.exists() {
                eprintln!("Skipping missing file: {}", filename);
                continue;
            }

            let content = fs::read(&file_path).map_err(|e| {
                ExtractionError::ParseError(format!("Failed to read {}: {}", filename, e))
            })?;

            let results = self.process_file(filename, &content)?;
            all_results.extend(results);
        }

        Ok(all_results)
    }

    fn process_file(
        &self,
        filename: &str,
        content: &[u8],
    ) -> Result<Vec<ExtractionResult>, ExtractionError> {
        let records = self.csv_parser.parse_to_maps(content)?;

        match filename {
            "Positions.csv" => processors::process_positions(&records),
            "Connections.csv" => processors::process_connections(&records),
            "Invitations.csv" => processors::process_invitations(&records),
            _ => Ok(Vec::new()),
        }
    }

    pub fn extract_connections_metadata(
        &self,
        archive_path: &str,
    ) -> Result<Vec<linkedin_metadata::ConnectionMetadata>, ExtractionError> {
        let file_path = Path::new(archive_path).join("Connections.csv");

        if !file_path.exists() {
            return Err(ExtractionError::InvalidInput(
                "Connections.csv not found".to_string(),
            ));
        }

        let content = fs::read(&file_path).map_err(|e| {
            ExtractionError::ParseError(format!("Failed to read Connections.csv: {}", e))
        })?;

        let mut records = self.csv_parser.parse_to_maps(&content)?;
        let mut metadata = Vec::new();

        for record in records.drain(..) {
            if let Some(first_name) = record.get("First Name") {
                if let Some(last_name) = record.get("Last Name") {
                    if !first_name.trim().is_empty() || !last_name.trim().is_empty() {
                        let connected_on = record
                            .get("Connected On")
                            .unwrap_or(&String::new())
                            .to_string();
                        let company_at_connection = record.get("Company").map(|s| s.clone());
                        let position_at_connection = record.get("Position").map(|s| s.clone());
                        let email_address = record.get("Email Address").map(|s| s.clone());
                        let url = record.get("URL").map(|s| s.clone());

                        metadata.push(linkedin_metadata::ConnectionMetadata {
                            first_name: first_name.clone(),
                            last_name: last_name.clone(),
                            connected_on,
                            company_at_connection,
                            position_at_connection,
                            email_address,
                            url,
                        });
                    }
                }
            }
        }

        Ok(metadata)
    }

    pub fn extract_invitations_metadata(
        &self,
        archive_path: &str,
    ) -> Result<Vec<linkedin_metadata::InvitationMetadata>, ExtractionError> {
        let file_path = Path::new(archive_path).join("Invitations.csv");

        if !file_path.exists() {
            return Err(ExtractionError::InvalidInput(
                "Invitations.csv not found".to_string(),
            ));
        }

        let content = fs::read(&file_path).map_err(|e| {
            ExtractionError::ParseError(format!("Failed to read Invitations.csv: {}", e))
        })?;

        let mut records = self.csv_parser.parse_to_maps(&content)?;
        let mut metadata = Vec::new();

        for record in records.drain(..) {
            if let Some(from) = record.get("From") {
                if let Some(to) = record.get("To") {
                    if !from.trim().is_empty() || !to.trim().is_empty() {
                        let sent_at = record.get("Sent At").unwrap_or(&String::new()).to_string();
                        let direction = record
                            .get("Direction")
                            .unwrap_or(&String::new())
                            .to_string();
                        let message = record.get("Message").map(|s| s.clone());
                        let inviter_profile_url =
                            record.get("inviterProfileUrl").map(|s| s.clone());
                        let invitee_profile_url =
                            record.get("inviteeProfileUrl").map(|s| s.clone());

                        metadata.push(linkedin_metadata::InvitationMetadata {
                            from: from.clone(),
                            to: to.clone(),
                            sent_at,
                            direction,
                            message,
                            inviter_profile_url,
                            invitee_profile_url,
                        });
                    }
                }
            }
        }

        Ok(metadata)
    }
}

impl Extractor for LinkedInArchiveExtractor {
    fn extract(&self, _input: &ExtractionInput) -> Result<Vec<ExtractionResult>, ExtractionError> {
        Ok(Vec::new())
    }

    fn data_type(&self) -> DataType {
        DataType::Contact
    }

    fn method(&self) -> ExtractionMethod {
        ExtractionMethod::PatternBased
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }
}
