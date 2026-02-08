# Task: LinkedIn Data Archive Extractor

## Objective

Implement a one-time extraction system for LinkedIn data archives that processes CSV files (Positions, Connections, Invitations) and extracts structured entities (Companies, Contacts with profile links, Work Positions) into dedicated database tables.

## Background

### LinkedIn Data Archive Structure

LinkedIn provides user data in CSV format, delivered in multiple installments. Reference archive at: `~/Downloads/Basic_LinkedInDataExport_01-30-2026`

**Key Files**:
- `Positions.csv` - Work history with company information
- `Connections.csv` - LinkedIn connections with profile URLs (~1,176 connections)
- `Invitations.csv` - Connection invitations sent/received with profile URLs

**Other Available Files** (not in scope for Phase 1):
- Education.csv, Skills.csv, Endorsements, Messages, etc.

### Current System State

- ✅ Extraction framework with jobs, events, and contacts
- ✅ Email attachment extractors (ICS/VCF)
- ✅ Contact type exists but lacks profile links
- ❌ No Company entity type
- ❌ No CSV-based extractor
- ❌ No support for local archive extraction

### LinkedIn Data Schema

#### Positions.csv
```csv
Company Name,Title,Description,Location,Started On,Finished On
Pixlie,Co-Founder,"Pixlie has been my passion project...",,Jan 2024,
Dwata,Founder,"One app for your emails..."," Himalayas, India",Jan 2024,
```

#### Connections.csv
```csv
First Name,Last Name,URL,Email Address,Company,Position,Connected On
Drishti,Sanghavi,https://www.linkedin.com/in/drishtisanghavi,,Salesflow,Growth Marketing Manager,22 Jan 2026
```

#### Invitations.csv
```csv
From,To,Sent At,Message,Direction,inviterProfileUrl,inviteeProfileUrl
Sumit Datta,Andrew Weiss,"1/19/26, 9:15 AM","Hey Andrew...",OUTGOING,https://www.linkedin.com/in/brainless,https://www.linkedin.com/in/andrew-weiss11
```

## Requirements

### Phase 1: Core Entities

1. **Company Type**: New entity for organizations
   - Name, description, location
   - Used by Positions and Connections
   - Stored in `companies` table

2. **Profile Links**: Extend Contact type
   - Support multiple profile links (LinkedIn, GitHub, Twitter, etc.)
   - Stored in `contact_links` table (many-to-many)
   - Link type enum (linkedin, github, twitter, personal, other)

3. **Work Position Type**: Employment history
   - Links Contact (person), Company, and date range
   - Title, description, location
   - Stored in `positions` table

4. **LinkedIn Connection Metadata**: Extra data for connections
   - Connection date
   - Connection source (via Connections.csv or Invitations.csv)
   - Message if applicable

### Phase 2: Extractor Implementation

1. **CSV Parser**: Generic CSV reader with column mapping
2. **LinkedInArchiveExtractor**: New extractor type for LinkedIn CSVs
3. **LocalArchive Source Type**: User specifies directory path
4. **Multi-Entity Extraction**: Single file can produce multiple entity types

### Phase 3: Job Management

1. **One-Time Job**: User provides archive path via API
2. **Progress Tracking**: Track files processed and entities extracted
3. **Entity Counts**: Track companies, contacts, positions created
4. **Error Handling**: Skip invalid rows, log errors

## Database Schema Changes

### 1. companies Table

```sql
CREATE TABLE IF NOT EXISTS companies (
    id INTEGER PRIMARY KEY DEFAULT nextval('seq_companies_id'),
    extraction_job_id INTEGER,              -- NULL if manually created

    -- Company data
    name VARCHAR NOT NULL,
    description VARCHAR,
    industry VARCHAR,
    location VARCHAR,
    website VARCHAR,

    -- LinkedIn specific
    linkedin_url VARCHAR,                   -- Company LinkedIn page

    -- Deduplication
    is_duplicate BOOLEAN DEFAULT false,
    merged_into_company_id INTEGER,

    -- Metadata
    confidence FLOAT,
    requires_review BOOLEAN DEFAULT false,
    is_confirmed BOOLEAN DEFAULT false,

    -- Timestamps
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,

    UNIQUE(name, location)                  -- Prevent exact duplicates
);

CREATE INDEX IF NOT EXISTS idx_companies_name ON companies(name);
CREATE INDEX IF NOT EXISTS idx_companies_extraction_job ON companies(extraction_job_id);
CREATE INDEX IF NOT EXISTS idx_companies_linkedin_url ON companies(linkedin_url);

CREATE SEQUENCE IF NOT EXISTS seq_companies_id;
```

### 2. contact_links Table

```sql
CREATE TABLE IF NOT EXISTS contact_links (
    id INTEGER PRIMARY KEY DEFAULT nextval('seq_contact_links_id'),
    contact_id INTEGER NOT NULL,

    -- Link data
    link_type VARCHAR NOT NULL,             -- linkedin, github, twitter, personal, other
    url VARCHAR NOT NULL,
    label VARCHAR,                          -- Optional display label

    -- Metadata
    is_primary BOOLEAN DEFAULT false,       -- Primary profile for this type
    is_verified BOOLEAN DEFAULT false,      -- User confirmed

    -- Timestamps
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,

    UNIQUE(contact_id, link_type, url)      -- Prevent duplicate links
);

CREATE INDEX IF NOT EXISTS idx_contact_links_contact ON contact_links(contact_id);
CREATE INDEX IF NOT EXISTS idx_contact_links_type ON contact_links(link_type);

CREATE SEQUENCE IF NOT EXISTS seq_contact_links_id;
```

### 3. positions Table

```sql
CREATE TABLE IF NOT EXISTS positions (
    id INTEGER PRIMARY KEY DEFAULT nextval('seq_positions_id'),
    extraction_job_id INTEGER,              -- NULL if manually created

    -- Relationships
    contact_id INTEGER NOT NULL,            -- Person who held this position
    company_id INTEGER NOT NULL,            -- Company where they worked

    -- Position data
    title VARCHAR NOT NULL,
    description VARCHAR,
    location VARCHAR,

    -- Date range
    started_on VARCHAR,                     -- "Jan 2024" format from LinkedIn
    finished_on VARCHAR,                    -- NULL if current
    started_date BIGINT,                    -- Parsed timestamp (best effort)
    finished_date BIGINT,                   -- Parsed timestamp (best effort)
    is_current BOOLEAN DEFAULT false,       -- Currently working here

    -- Metadata
    confidence FLOAT,
    requires_review BOOLEAN DEFAULT false,
    is_confirmed BOOLEAN DEFAULT false,

    -- Timestamps
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_positions_contact ON positions(contact_id);
CREATE INDEX IF NOT EXISTS idx_positions_company ON positions(company_id);
CREATE INDEX IF NOT EXISTS idx_positions_extraction_job ON positions(extraction_job_id);
CREATE INDEX IF NOT EXISTS idx_positions_dates ON positions(started_date DESC, finished_date DESC);

CREATE SEQUENCE IF NOT EXISTS seq_positions_id;
```

### 4. linkedin_connections Table

```sql
CREATE TABLE IF NOT EXISTS linkedin_connections (
    id INTEGER PRIMARY KEY DEFAULT nextval('seq_linkedin_connections_id'),
    extraction_job_id INTEGER NOT NULL,
    contact_id INTEGER NOT NULL,           -- Links to contacts table

    -- Connection metadata
    connected_on VARCHAR,                   -- "22 Jan 2026" format
    connected_date BIGINT,                  -- Parsed timestamp
    connection_source VARCHAR,              -- "connections" or "invitations"

    -- If from invitations
    direction VARCHAR,                      -- INCOMING or OUTGOING
    invitation_message VARCHAR,
    invitation_sent_at VARCHAR,

    -- Position at time of connection
    company_at_connection VARCHAR,
    position_at_connection VARCHAR,

    -- Timestamps
    created_at BIGINT NOT NULL,

    UNIQUE(contact_id, extraction_job_id)   -- One connection record per extraction
);

CREATE INDEX IF NOT EXISTS idx_linkedin_connections_contact ON linkedin_connections(contact_id);
CREATE INDEX IF NOT EXISTS idx_linkedin_connections_extraction_job ON linkedin_connections(extraction_job_id);
CREATE INDEX IF NOT EXISTS idx_linkedin_connections_date ON linkedin_connections(connected_date DESC);

CREATE SEQUENCE IF NOT EXISTS seq_linkedin_connections_id;
```

## Type Definitions

### Shared Types

**File**: `shared-types/src/company.rs`

```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Company {
    pub id: i64,
    pub extraction_job_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub industry: Option<String>,
    pub location: Option<String>,
    pub website: Option<String>,
    pub linkedin_url: Option<String>,
    pub confidence: Option<f32>,
    pub requires_review: bool,
    pub is_confirmed: bool,
    pub is_duplicate: bool,
    pub merged_into_company_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateCompanyRequest {
    pub name: String,
    pub description: Option<String>,
    pub industry: Option<String>,
    pub location: Option<String>,
    pub website: Option<String>,
    pub linkedin_url: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateCompanyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub industry: Option<String>,
    pub location: Option<String>,
    pub website: Option<String>,
    pub linkedin_url: Option<String>,
    pub is_confirmed: Option<bool>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct CompaniesResponse {
    pub companies: Vec<Company>,
}
```

**File**: `shared-types/src/contact_link.rs`

```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum ContactLinkType {
    Linkedin,
    Github,
    Twitter,
    Personal,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ContactLink {
    pub id: i64,
    pub contact_id: i64,
    pub link_type: ContactLinkType,
    pub url: String,
    pub label: Option<String>,
    pub is_primary: bool,
    pub is_verified: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateContactLinkRequest {
    pub contact_id: i64,
    pub link_type: ContactLinkType,
    pub url: String,
    pub label: Option<String>,
    pub is_primary: Option<bool>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ContactLinksResponse {
    pub links: Vec<ContactLink>,
}
```

**File**: `shared-types/src/position.rs`

```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Position {
    pub id: i64,
    pub extraction_job_id: Option<i64>,
    pub contact_id: i64,
    pub company_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub started_on: Option<String>,        // Original format
    pub finished_on: Option<String>,       // Original format
    pub started_date: Option<i64>,         // Parsed timestamp
    pub finished_date: Option<i64>,        // Parsed timestamp
    pub is_current: bool,
    pub confidence: Option<f32>,
    pub requires_review: bool,
    pub is_confirmed: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreatePositionRequest {
    pub contact_id: i64,
    pub company_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub started_on: Option<String>,
    pub finished_on: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct PositionsResponse {
    pub positions: Vec<Position>,
}
```

**Update**: `shared-types/src/extraction_job.rs`

Add new variants:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum ExtractionSourceType {
    EmailAttachment,
    LocalFile,
    LocalArchive,        // NEW: For LinkedIn/other archives
    EmailBody,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum ExtractorType {
    AttachmentParser,
    LinkedInArchive,     // NEW: For LinkedIn CSV parsing
    GlinerNER,
    LLMBased,
}

// Update ExtractionSourceConfig
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", content = "config")]
pub enum ExtractionSourceConfig {
    EmailAttachments { /* ... */ },
    LocalFile { /* ... */ },
    LocalArchive {       // NEW
        archive_path: String,              // Directory containing CSVs
        archive_type: ArchiveType,
        files_to_process: Vec<String>,     // ["Positions.csv", "Connections.csv"]
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum ArchiveType {
    LinkedIn,
    // Future: Google, Twitter, Facebook, etc.
}

// Update ExtractionProgress
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExtractionProgress {
    pub total_items: u64,
    pub processed_items: u64,
    pub extracted_entities: u64,
    pub failed_items: u64,
    pub events_extracted: u64,
    pub contacts_extracted: u64,
    pub companies_extracted: u64,     // NEW
    pub positions_extracted: u64,     // NEW
    pub percent_complete: f32,
}
```

**Update**: `shared-types/src/extraction.rs`

Add new entity types:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum DataType {
    Project,
    Task,
    Event,
    Contact,
    Location,
    Date,
    Priority,
    Status,
    Company,      // NEW
    Position,     // NEW
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "data")]
pub enum ExtractedEntity {
    Project(ExtractedProject),
    Task(ExtractedTask),
    Event(ExtractedEvent),
    Contact(ExtractedContact),
    Location(ExtractedLocation),
    Company(ExtractedCompany),        // NEW
    Position(ExtractedPosition),      // NEW
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ExtractedCompany {
    pub name: String,
    pub description: Option<String>,
    pub industry: Option<String>,
    pub location: Option<String>,
    pub website: Option<String>,
    pub linkedin_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ExtractedPosition {
    pub contact_name: String,              // To link to contact
    pub company_name: String,              // To link to company
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub started_on: Option<String>,
    pub finished_on: Option<String>,
    pub is_current: bool,
}

// Update ExtractedContact to include profile URLs
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ExtractedContact {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub organization: Option<String>,
    pub profile_urls: Vec<ProfileUrl>,     // NEW
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ProfileUrl {
    pub url: String,
    pub link_type: String,                 // "linkedin", "github", etc.
}
```

## Extractor Implementation

### CSV Parser Utility

**File**: `extractors/src/linkedin_archive/csv_parser.rs`

```rust
use csv::ReaderBuilder;
use serde::de::DeserializeOwned;
use shared_types::ExtractionError;
use std::collections::HashMap;

/// Generic CSV parser with flexible column mapping
pub struct CsvParser {
    delimiter: u8,
    has_headers: bool,
}

impl CsvParser {
    pub fn new() -> Self {
        Self {
            delimiter: b',',
            has_headers: true,
        }
    }

    pub fn parse_file<T: DeserializeOwned>(
        &self,
        content: &[u8],
    ) -> Result<Vec<T>, ExtractionError> {
        let mut reader = ReaderBuilder::new()
            .delimiter(self.delimiter)
            .has_headers(self.has_headers)
            .from_reader(content);

        let mut records = Vec::new();

        for result in reader.deserialize() {
            match result {
                Ok(record) => records.push(record),
                Err(e) => {
                    eprintln!("Failed to parse CSV row: {}", e);
                    // Continue processing other rows
                }
            }
        }

        Ok(records)
    }

    /// Parse CSV and return as vector of HashMaps (for flexible processing)
    pub fn parse_to_maps(
        &self,
        content: &[u8],
    ) -> Result<Vec<HashMap<String, String>>, ExtractionError> {
        let mut reader = ReaderBuilder::new()
            .delimiter(self.delimiter)
            .has_headers(self.has_headers)
            .from_reader(content);

        let headers = reader
            .headers()
            .map_err(|e| ExtractionError::ParseError(e.to_string()))?
            .clone();

        let mut records = Vec::new();

        for result in reader.records() {
            match result {
                Ok(record) => {
                    let mut map = HashMap::new();
                    for (i, field) in record.iter().enumerate() {
                        if let Some(header) = headers.get(i) {
                            map.insert(header.to_string(), field.to_string());
                        }
                    }
                    records.push(map);
                }
                Err(e) => {
                    eprintln!("Failed to parse CSV row: {}", e);
                }
            }
        }

        Ok(records)
    }
}
```

### LinkedIn Date Parser

**File**: `extractors/src/linkedin_archive/date_parser.rs`

```rust
use chrono::{Datelike, NaiveDate, Utc};
use shared_types::ExtractionError;

/// Parse LinkedIn date formats:
/// - "Jan 2024" -> first day of month
/// - "22 Jan 2026" -> exact date
/// - "" or missing -> None
pub fn parse_linkedin_date(date_str: &str) -> Option<i64> {
    if date_str.trim().is_empty() {
        return None;
    }

    // Try "DD Mon YYYY" format first (22 Jan 2026)
    if let Ok(date) = NaiveDate::parse_from_str(date_str, "%d %b %Y") {
        return Some(date.and_hms_opt(0, 0, 0)?.and_utc().timestamp());
    }

    // Try "Mon YYYY" format (Jan 2024)
    if let Ok(date) = NaiveDate::parse_from_str(&format!("01 {}", date_str), "%d %b %Y") {
        return Some(date.and_hms_opt(0, 0, 0)?.and_utc().timestamp());
    }

    None
}

/// Check if a position is current (no finished_on date)
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

        // Should be Jan 1, 2024
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
```

### LinkedIn Archive Extractor

**File**: `extractors/src/linkedin_archive/mod.rs`

```rust
mod csv_parser;
mod date_parser;
mod processors;

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

    /// Process a LinkedIn archive directory
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

            let content = fs::read(&file_path)
                .map_err(|e| ExtractionError::ParseError(format!("Failed to read {}: {}", filename, e)))?;

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
}

impl Extractor for LinkedInArchiveExtractor {
    fn extract(
        &self,
        input: &ExtractionInput,
    ) -> Result<Vec<ExtractionResult>, ExtractionError> {
        // Not used for archive extraction
        // Archive extraction is triggered via process_archive()
        Ok(Vec::new())
    }

    fn data_type(&self) -> DataType {
        DataType::Contact // Primary target
    }

    fn method(&self) -> ExtractionMethod {
        ExtractionMethod::PatternBased // CSV parsing
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}
```

### File Processors

**File**: `extractors/src/linkedin_archive/processors.rs`

```rust
use crate::linkedin_archive::date_parser::{is_current_position, parse_linkedin_date};
use shared_types::{
    extraction::{
        ExtractedCompany, ExtractedContact, ExtractedEntity, ExtractedPosition, ProfileUrl,
    },
    DataType, ExtractionError, ExtractionMethod, ExtractionResult,
};
use std::collections::HashMap;

/// Process Positions.csv
pub fn process_positions(
    records: &[HashMap<String, String>],
) -> Result<Vec<ExtractionResult>, ExtractionError> {
    let mut results = Vec::new();

    for record in records {
        let company_name = record.get("Company Name").map(|s| s.trim().to_string());
        let title = record.get("Title").map(|s| s.trim().to_string());

        if company_name.is_none() || title.is_none() {
            continue; // Skip rows without essential data
        }

        let company_name = company_name.unwrap();
        let title = title.unwrap();
        let description = record.get("Description").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let location = record.get("Location").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let started_on = record.get("Started On").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let finished_on = record.get("Finished On").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

        // Extract Company
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
            confidence: 1.0, // High confidence from structured data
            confidence_breakdown: HashMap::from([("source".to_string(), 1.0)]),
            method: ExtractionMethod::PatternBased,
            evidence: vec![],
            relationships: vec![],
            requires_review: false,
            ambiguities: vec![],
            extracted_at: chrono::Utc::now().timestamp(),
            extractor_version: "1.0.0".to_string(),
        });

        // Extract Position
        let is_current = finished_on.is_none();

        let position = ExtractedPosition {
            contact_name: String::new(), // Will be filled in by extraction manager with user context
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

/// Process Connections.csv
pub fn process_connections(
    records: &[HashMap<String, String>],
) -> Result<Vec<ExtractionResult>, ExtractionError> {
    let mut results = Vec::new();

    for record in records {
        let first_name = record.get("First Name").map(|s| s.trim().to_string()).unwrap_or_default();
        let last_name = record.get("Last Name").map(|s| s.trim().to_string()).unwrap_or_default();
        let url = record.get("URL").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let email = record.get("Email Address").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let company = record.get("Company").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let position = record.get("Position").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

        if first_name.is_empty() && last_name.is_empty() {
            continue; // Skip rows without name
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

        // Also store connection metadata for linkedin_connections table
        // This will be handled by the extraction manager
    }

    Ok(results)
}

/// Process Invitations.csv
pub fn process_invitations(
    records: &[HashMap<String, String>],
) -> Result<Vec<ExtractionResult>, ExtractionError> {
    let mut results = Vec::new();

    for record in records {
        let from = record.get("From").map(|s| s.trim().to_string()).unwrap_or_default();
        let to = record.get("To").map(|s| s.trim().to_string()).unwrap_or_default();
        let direction = record.get("Direction").map(|s| s.trim().to_string()).unwrap_or_default();

        let inviter_url = record.get("inviterProfileUrl").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let invitee_url = record.get("inviteeProfileUrl").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

        // For OUTGOING invitations, "To" is the contact
        // For INCOMING invitations, "From" is the contact
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
```

## Implementation Plan

### Phase 1: Database & Types (Day 1-2)

#### Step 1.1: Add Database Migrations

**File**: `dwata-api/src/database/migrations.rs`

Add sequences and tables for companies, contact_links, positions, linkedin_connections.

#### Step 1.2: Create Shared Types

Create new files:
- `shared-types/src/company.rs`
- `shared-types/src/contact_link.rs`
- `shared-types/src/position.rs`

Update existing files:
- `shared-types/src/extraction.rs` - Add Company and Position variants
- `shared-types/src/extraction_job.rs` - Add LinkedInArchive variants
- `shared-types/src/lib.rs` - Export new types

#### Step 1.3: Add csv Dependency

**File**: `extractors/Cargo.toml`

```toml
[dependencies]
csv = "1.3"
```

### Phase 2: Database Operations (Day 2-3)

Create database operation files:

**Files**:
- `dwata-api/src/database/companies.rs` - CRUD for companies
- `dwata-api/src/database/contact_links.rs` - CRUD for contact links
- `dwata-api/src/database/positions.rs` - CRUD for positions
- `dwata-api/src/database/linkedin_connections.rs` - CRUD for LinkedIn metadata

Add exports to `dwata-api/src/database/mod.rs`.

### Phase 3: LinkedIn Extractor (Day 3-4)

Create extractor files:

**Files**:
- `extractors/src/linkedin_archive/mod.rs`
- `extractors/src/linkedin_archive/csv_parser.rs`
- `extractors/src/linkedin_archive/date_parser.rs`
- `extractors/src/linkedin_archive/processors.rs`

Update `extractors/src/lib.rs` to export `LinkedInArchiveExtractor`.

### Phase 4: Extraction Manager Updates (Day 4-5)

**File**: `dwata-api/src/jobs/extraction_manager.rs`

Add new method:

```rust
async fn extract_from_local_archive(
    db_conn: AsyncDbConnection,
    job_id: i64,
    archive_path: String,
    archive_type: ArchiveType,
    files_to_process: Vec<String>,
) -> Result<()>
```

This method will:
1. Create LinkedInArchiveExtractor
2. Process archive directory
3. For each extracted entity:
   - If Company: Check for duplicates, insert
   - If Contact: Check for duplicates by email/name, insert or update
   - If Contact has profile_urls: Insert into contact_links
   - If Position: Link to contact and company, insert
4. Update job progress counters
5. Mark job as completed

### Phase 5: API Handlers (Day 5-6)

Create handlers:

**Files**:
- `dwata-api/src/handlers/companies.rs`
- `dwata-api/src/handlers/positions.rs`
- Update `dwata-api/src/handlers/contacts.rs` to include links

Add routes to `dwata-api/src/main.rs`:

```rust
// Companies
.route("/api/companies", web::get().to(companies::list_companies))
.route("/api/companies/{id}", web::get().to(companies::get_company))

// Positions
.route("/api/positions", web::get().to(positions::list_positions))
.route("/api/positions/{id}", web::get().to(positions::get_position))
.route("/api/contacts/{id}/positions", web::get().to(positions::list_contact_positions))

// Contact Links
.route("/api/contacts/{id}/links", web::get().to(contacts::get_contact_links))
```

### Phase 6: TypeScript Types (Day 6)

Generate TypeScript types:

```bash
cd shared-types
cargo run --bin generate_api_types
```

### Phase 7: Testing (Day 6-7)

#### Unit Tests

Add tests for:
- Date parser (extractors)
- CSV parser (extractors)
- Database operations (dwata-api)

#### Integration Test

Create extraction job with test LinkedIn archive:

```bash
curl -X POST http://localhost:8080/api/extractions \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "local-archive",
    "extractor_type": "linkedin-archive",
    "source_config": {
      "type": "LocalArchive",
      "config": {
        "archive_path": "/Users/username/Downloads/LinkedIn_Export",
        "archive_type": "linkedin",
        "files_to_process": ["Positions.csv", "Connections.csv", "Invitations.csv"]
      }
    }
  }'

# Start extraction
curl -X POST http://localhost:8080/api/extractions/1/start

# Check progress
curl http://localhost:8080/api/extractions/1

# List extracted companies
curl http://localhost:8080/api/companies

# List extracted contacts
curl http://localhost:8080/api/contacts

# List positions
curl http://localhost:8080/api/positions
```

## Success Criteria

- ✅ Database schema created (companies, contact_links, positions, linkedin_connections)
- ✅ Shared types created and exported to TypeScript
- ✅ LinkedInArchiveExtractor implemented and tested
- ✅ CSV parsing handles LinkedIn date formats
- ✅ Database operations for new entities
- ✅ Extraction manager processes archive directories
- ✅ Companies extracted and deduplicated
- ✅ Contacts extracted with LinkedIn profile URLs
- ✅ Positions extracted and linked to contacts and companies
- ✅ API endpoints for querying extracted data
- ✅ Can process sample LinkedIn archive successfully

## Future Enhancements

### Phase 2 Features

1. **Additional LinkedIn Files**:
   - Education.csv → Education entity type
   - Skills.csv → Skills attached to contacts
   - Endorsements → Skill endorsements
   - Messages.csv → Message archive

2. **Company Enrichment**:
   - Fetch company data from LinkedIn API
   - Auto-populate industry, employee count
   - Company logo from Clearbit/Brandfetch

3. **Contact Deduplication**:
   - Fuzzy name matching
   - Email-based merging
   - Manual merge UI

4. **Position Timeline View**:
   - Career timeline visualization
   - Company overlap detection
   - Gap analysis

5. **Multi-Archive Support**:
   - Google Takeout (Gmail, Calendar, Contacts)
   - Twitter archive
   - Facebook data download
   - Instagram data export

6. **Smart Linking**:
   - Link positions to events (e.g., "Started at Company X")
   - Link contacts to emails
   - Auto-create project from long-term position

## Architecture Considerations

### Deduplication Strategy

**Companies**:
- Match on name + location (exact)
- Flag similar names for review
- Manual merge via UI

**Contacts**:
- Exact match on email (primary key)
- Fuzzy match on name + organization
- LinkedIn URL as secondary identifier
- Allow manual merge

**Positions**:
- No automatic deduplication
- Allow user to mark as duplicate

### Performance

**Large Archives**:
- Process CSVs in batches (1000 rows at a time)
- Use transactions for atomic inserts
- Background job with progress tracking
- Abort/resume capability

**Database Queries**:
- Index on foreign keys
- Index on commonly searched fields (name, email, company)
- Consider full-text search for company/contact names

### Security

**File Access**:
- Validate archive path (prevent path traversal)
- Only read files within specified directory
- Sanitize user input

**Data Privacy**:
- LinkedIn data is personal
- Consider encryption at rest
- Clear job history after completion (keep entities)

---

**Document Version**: 1.0
**Created**: 2026-01-30
**Status**: Ready for Implementation
**Estimated Effort**: 6-7 developer days
