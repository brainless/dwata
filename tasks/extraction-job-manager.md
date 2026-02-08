# Task: Extraction Job Manager with Events and Contacts Persistence

## Objective

Implement a comprehensive extraction management system that processes email attachments to extract Events and Contacts using the attachment parser extractor, tracks extraction jobs, and persists results to typed database tables.

## Background

### Current State
- **extractors crate**: Fully functional attachment parser for ICS (events) and VCF (contacts)
- **shared-types**: Has extraction framework types (`Extractor` trait, `ExtractionResult`, etc.)
- **Gap**: No persistence layer, no job management, no integration with dwata-api

### Requirements
1. **Extraction Jobs**: Background job system for processing attachments
2. **Progress Tracking**: Real-time status updates (total attachments, processed, failed)
3. **Source Tracking**: Record where data came from (email_attachments table, local files)
4. **Extractor Tracking**: Record which extractor was used (attachment-parser, future: NER, LLM)
5. **Typed Storage**: Store extracted Events and Contacts in dedicated tables
6. **Job References**: Events and contacts reference the extraction job that created them
7. **Ephemeral Jobs**: Extraction jobs can be deleted without losing extracted data
8. **API Access**: REST endpoints for starting/monitoring extractions

### Architecture Pattern

Following the same pattern as `download_jobs` and `download_items`:
- **extraction_jobs**: Track extraction operations (ephemeral)
- **events**: Store extracted event data (persistent)
- **contacts**: Store extracted contact data (persistent)

## Database Schema

### extraction_jobs Table

```sql
CREATE TABLE IF NOT EXISTS extraction_jobs (
    id INTEGER PRIMARY KEY DEFAULT nextval('seq_extraction_jobs_id'),
    source_type VARCHAR NOT NULL,              -- email_attachment, local_file, etc.
    extractor_type VARCHAR NOT NULL,           -- attachment-parser, gliner-ner, llm-based
    status VARCHAR NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),

    -- Progress tracking
    total_items INTEGER NOT NULL DEFAULT 0,
    processed_items INTEGER NOT NULL DEFAULT 0,
    extracted_entities INTEGER NOT NULL DEFAULT 0,
    failed_items INTEGER NOT NULL DEFAULT 0,

    -- Source configuration (JSON)
    source_config VARCHAR NOT NULL,            -- ExtractionSourceConfig

    -- Results breakdown
    events_extracted INTEGER NOT NULL DEFAULT 0,
    contacts_extracted INTEGER NOT NULL DEFAULT 0,

    -- Error handling
    error_message VARCHAR,

    -- Timestamps
    created_at BIGINT NOT NULL,
    started_at BIGINT,
    updated_at BIGINT NOT NULL,
    completed_at BIGINT
);

CREATE INDEX IF NOT EXISTS idx_extraction_jobs_status
    ON extraction_jobs(status, updated_at);

CREATE INDEX IF NOT EXISTS idx_extraction_jobs_extractor
    ON extraction_jobs(extractor_type);
```

### events Table

```sql
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY DEFAULT nextval('seq_events_id'),
    extraction_job_id INTEGER,                 -- NULL if manually created
    email_id INTEGER,                          -- Source email if applicable

    -- Event data
    name VARCHAR NOT NULL,
    description VARCHAR,
    event_date BIGINT NOT NULL,                -- Timestamp
    location VARCHAR,
    attendees VARCHAR,                         -- JSON array of email addresses

    -- Metadata
    confidence FLOAT,                          -- Extraction confidence (0.0-1.0)
    requires_review BOOLEAN DEFAULT false,     -- Flagged for user review
    is_confirmed BOOLEAN DEFAULT false,        -- User confirmed

    -- Relations (future)
    project_id INTEGER,
    task_id INTEGER,

    -- Timestamps
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_events_date
    ON events(event_date DESC);

CREATE INDEX IF NOT EXISTS idx_events_extraction_job
    ON events(extraction_job_id);

CREATE INDEX IF NOT EXISTS idx_events_email
    ON events(email_id);
```

### contacts Table

```sql
CREATE TABLE IF NOT EXISTS contacts (
    id INTEGER PRIMARY KEY DEFAULT nextval('seq_contacts_id'),
    extraction_job_id INTEGER,                 -- NULL if manually created
    email_id INTEGER,                          -- Source email if applicable

    -- Contact data
    name VARCHAR NOT NULL,
    email VARCHAR,
    phone VARCHAR,
    organization VARCHAR,

    -- Metadata
    confidence FLOAT,                          -- Extraction confidence (0.0-1.0)
    requires_review BOOLEAN DEFAULT false,     -- Flagged for user review
    is_confirmed BOOLEAN DEFAULT false,        -- User confirmed

    -- Deduplication
    is_duplicate BOOLEAN DEFAULT false,
    merged_into_contact_id INTEGER,            -- If merged with another contact

    -- Timestamps
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,

    UNIQUE(email)                              -- Prevent duplicate emails
);

CREATE INDEX IF NOT EXISTS idx_contacts_extraction_job
    ON contacts(extraction_job_id);

CREATE INDEX IF NOT EXISTS idx_contacts_email
    ON contacts(email);

CREATE INDEX IF NOT EXISTS idx_contacts_name
    ON contacts(name);
```

## Type Definitions

### Shared Types (shared-types/src/extraction_job.rs)

```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Extraction job for processing attachments and extracting entities
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ExtractionJob {
    pub id: i64,
    pub source_type: ExtractionSourceType,
    pub extractor_type: ExtractorType,
    pub status: ExtractionJobStatus,
    pub progress: ExtractionProgress,
    #[ts(skip)]
    pub source_config: serde_json::Value,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub updated_at: i64,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum ExtractionSourceType {
    EmailAttachment,      // Extract from email attachments table
    LocalFile,            // Extract from uploaded file
    EmailBody,            // Extract from email body text (future)
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum ExtractorType {
    AttachmentParser,     // ICS, VCF parsing
    GlinerNER,            // Named entity recognition (future)
    LLMBased,             // LLM reasoning (future)
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ExtractionJobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ExtractionProgress {
    pub total_items: u64,              // Total attachments/sources to process
    pub processed_items: u64,          // Processed so far
    pub extracted_entities: u64,       // Total entities extracted
    pub failed_items: u64,             // Failed to process
    pub events_extracted: u64,         // Events found
    pub contacts_extracted: u64,       // Contacts found
    pub percent_complete: f32,
}

/// Configuration for extraction source
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "config")]
pub enum ExtractionSourceConfig {
    EmailAttachments {
        email_ids: Option<Vec<i64>>,           // Specific emails, or None for all pending
        attachment_types: Vec<String>,         // ["text/calendar", "text/vcard"]
        status_filter: AttachmentExtractionFilter,
    },
    LocalFile {
        file_path: String,
        content_type: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum AttachmentExtractionFilter {
    Pending,              // Only process pending attachments
    PendingAndFailed,     // Retry failed ones
    All,                  // Reprocess everything
}

/// Request to create extraction job
#[derive(Debug, Deserialize, TS)]
pub struct CreateExtractionJobRequest {
    pub source_type: ExtractionSourceType,
    pub extractor_type: ExtractorType,
    pub source_config: ExtractionSourceConfig,
}

/// Response for extraction job list
#[derive(Debug, Serialize, TS)]
pub struct ExtractionJobListResponse {
    pub jobs: Vec<ExtractionJob>,
}
```

### Event and Contact Types (shared-types/src/event.rs, contact.rs)

**File**: `shared-types/src/event.rs`

```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Event {
    pub id: i64,
    pub extraction_job_id: Option<i64>,
    pub email_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub event_date: i64,                    // Unix timestamp
    pub location: Option<String>,
    #[ts(skip)]
    pub attendees: serde_json::Value,       // Array of email addresses
    pub confidence: Option<f32>,
    pub requires_review: bool,
    pub is_confirmed: bool,
    pub project_id: Option<i64>,
    pub task_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateEventRequest {
    pub name: String,
    pub description: Option<String>,
    pub event_date: i64,
    pub location: Option<String>,
    pub attendees: Vec<String>,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateEventRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub event_date: Option<i64>,
    pub location: Option<String>,
    pub attendees: Option<Vec<String>>,
    pub is_confirmed: Option<bool>,
}

#[derive(Debug, Serialize, TS)]
pub struct EventsResponse {
    pub events: Vec<Event>,
}
```

**File**: `shared-types/src/contact.rs`

```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Contact {
    pub id: i64,
    pub extraction_job_id: Option<i64>,
    pub email_id: Option<i64>,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub organization: Option<String>,
    pub confidence: Option<f32>,
    pub requires_review: bool,
    pub is_confirmed: bool,
    pub is_duplicate: bool,
    pub merged_into_contact_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateContactRequest {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub organization: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateContactRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub organization: Option<String>,
    pub is_confirmed: Option<bool>,
}

#[derive(Debug, Serialize, TS)]
pub struct ContactsResponse {
    pub contacts: Vec<Contact>,
}
```

## Implementation Plan

### Phase 1: Database Schema & Types

#### 1.1 Add Migrations

**File**: `dwata-api/src/database/migrations.rs`

Add to `run_migrations()`:

```rust
// Create sequences
conn.execute("CREATE SEQUENCE IF NOT EXISTS seq_extraction_jobs_id", [])?;
conn.execute("CREATE SEQUENCE IF NOT EXISTS seq_events_id", [])?;
conn.execute("CREATE SEQUENCE IF NOT EXISTS seq_contacts_id", [])?;

// Create extraction_jobs table
conn.execute(
    "CREATE TABLE IF NOT EXISTS extraction_jobs (
        id INTEGER PRIMARY KEY DEFAULT nextval('seq_extraction_jobs_id'),
        source_type VARCHAR NOT NULL,
        extractor_type VARCHAR NOT NULL,
        status VARCHAR NOT NULL DEFAULT 'pending',
        total_items INTEGER NOT NULL DEFAULT 0,
        processed_items INTEGER NOT NULL DEFAULT 0,
        extracted_entities INTEGER NOT NULL DEFAULT 0,
        failed_items INTEGER NOT NULL DEFAULT 0,
        source_config VARCHAR NOT NULL,
        events_extracted INTEGER NOT NULL DEFAULT 0,
        contacts_extracted INTEGER NOT NULL DEFAULT 0,
        error_message VARCHAR,
        created_at BIGINT NOT NULL,
        started_at BIGINT,
        updated_at BIGINT NOT NULL,
        completed_at BIGINT
    )",
    [],
)?;

conn.execute(
    "CREATE INDEX IF NOT EXISTS idx_extraction_jobs_status
     ON extraction_jobs(status, updated_at)",
    [],
)?;

// Create events table
conn.execute(
    "CREATE TABLE IF NOT EXISTS events (
        id INTEGER PRIMARY KEY DEFAULT nextval('seq_events_id'),
        extraction_job_id INTEGER,
        email_id INTEGER,
        name VARCHAR NOT NULL,
        description VARCHAR,
        event_date BIGINT NOT NULL,
        location VARCHAR,
        attendees VARCHAR,
        confidence FLOAT,
        requires_review BOOLEAN DEFAULT false,
        is_confirmed BOOLEAN DEFAULT false,
        project_id INTEGER,
        task_id INTEGER,
        created_at BIGINT NOT NULL,
        updated_at BIGINT NOT NULL
    )",
    [],
)?;

conn.execute(
    "CREATE INDEX IF NOT EXISTS idx_events_date ON events(event_date DESC)",
    [],
)?;

conn.execute(
    "CREATE INDEX IF NOT EXISTS idx_events_extraction_job ON events(extraction_job_id)",
    [],
)?;

// Create contacts table
conn.execute(
    "CREATE TABLE IF NOT EXISTS contacts (
        id INTEGER PRIMARY KEY DEFAULT nextval('seq_contacts_id'),
        extraction_job_id INTEGER,
        email_id INTEGER,
        name VARCHAR NOT NULL,
        email VARCHAR,
        phone VARCHAR,
        organization VARCHAR,
        confidence FLOAT,
        requires_review BOOLEAN DEFAULT false,
        is_confirmed BOOLEAN DEFAULT false,
        is_duplicate BOOLEAN DEFAULT false,
        merged_into_contact_id INTEGER,
        created_at BIGINT NOT NULL,
        updated_at BIGINT NOT NULL,
        UNIQUE(email)
    )",
    [],
)?;

conn.execute(
    "CREATE INDEX IF NOT EXISTS idx_contacts_extraction_job ON contacts(extraction_job_id)",
    [],
)?;

conn.execute(
    "CREATE INDEX IF NOT EXISTS idx_contacts_email ON contacts(email)",
    [],
)?;
```

#### 1.2 Create Shared Types

Create the following files:
- `shared-types/src/extraction_job.rs` - Extraction job types
- `shared-types/src/contact.rs` - Contact entity types

Update `shared-types/src/event.rs` with the Event types shown above.

**File**: `shared-types/src/lib.rs`

Add exports:
```rust
pub mod contact;
pub mod extraction_job;

pub use contact::*;
pub use extraction_job::*;
```

#### 1.3 Add extractors Dependency

**File**: `dwata-api/Cargo.toml`

Add:
```toml
[dependencies]
extractors = { path = "../extractors" }
```

### Phase 2: Database Operations

**File**: `dwata-api/src/database/extraction_jobs.rs`

```rust
use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::extraction_job::{
    CreateExtractionJobRequest, ExtractionJob, ExtractionJobStatus, ExtractionProgress,
    ExtractionSourceType, ExtractorType,
};

/// Insert new extraction job
pub async fn insert_extraction_job(
    conn: AsyncDbConnection,
    request: &CreateExtractionJobRequest,
) -> Result<ExtractionJob> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    let source_config_json = serde_json::to_string(&request.source_config)?;

    let source_type_str = match request.source_type {
        ExtractionSourceType::EmailAttachment => "email-attachment",
        ExtractionSourceType::LocalFile => "local-file",
        ExtractionSourceType::EmailBody => "email-body",
    };

    let extractor_type_str = match request.extractor_type {
        ExtractorType::AttachmentParser => "attachment-parser",
        ExtractorType::GlinerNER => "gliner-ner",
        ExtractorType::LLMBased => "llm-based",
    };

    let id: i64 = conn.query_row(
        "INSERT INTO extraction_jobs
         (source_type, extractor_type, status, source_config, created_at, updated_at)
         VALUES (?, ?, 'pending', ?, ?, ?)
         RETURNING id",
        duckdb::params![source_type_str, extractor_type_str, &source_config_json, now, now],
        |row| row.get(0),
    )?;

    Ok(ExtractionJob {
        id,
        source_type: request.source_type.clone(),
        extractor_type: request.extractor_type.clone(),
        status: ExtractionJobStatus::Pending,
        progress: ExtractionProgress {
            total_items: 0,
            processed_items: 0,
            extracted_entities: 0,
            failed_items: 0,
            events_extracted: 0,
            contacts_extracted: 0,
            percent_complete: 0.0,
        },
        source_config: serde_json::to_value(&request.source_config)?,
        error_message: None,
        created_at: now,
        started_at: None,
        updated_at: now,
        completed_at: None,
    })
}

/// Get extraction job by ID
pub async fn get_extraction_job(conn: AsyncDbConnection, id: i64) -> Result<ExtractionJob> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, source_type, extractor_type, status, total_items, processed_items,
                extracted_entities, failed_items, events_extracted, contacts_extracted,
                source_config, error_message, created_at, started_at, updated_at, completed_at
         FROM extraction_jobs
         WHERE id = ?",
    )?;

    stmt.query_row([id], |row| {
        let source_type_str: String = row.get(1)?;
        let source_type = match source_type_str.as_str() {
            "email-attachment" => ExtractionSourceType::EmailAttachment,
            "local-file" => ExtractionSourceType::LocalFile,
            "email-body" => ExtractionSourceType::EmailBody,
            _ => ExtractionSourceType::EmailAttachment,
        };

        let extractor_type_str: String = row.get(2)?;
        let extractor_type = match extractor_type_str.as_str() {
            "attachment-parser" => ExtractorType::AttachmentParser,
            "gliner-ner" => ExtractorType::GlinerNER,
            "llm-based" => ExtractorType::LLMBased,
            _ => ExtractorType::AttachmentParser,
        };

        let status_str: String = row.get(3)?;
        let status = match status_str.as_str() {
            "pending" => ExtractionJobStatus::Pending,
            "running" => ExtractionJobStatus::Running,
            "completed" => ExtractionJobStatus::Completed,
            "failed" => ExtractionJobStatus::Failed,
            "cancelled" => ExtractionJobStatus::Cancelled,
            _ => ExtractionJobStatus::Pending,
        };

        let total_items: i64 = row.get(4)?;
        let processed_items: i64 = row.get(5)?;
        let extracted_entities: i64 = row.get(6)?;
        let failed_items: i64 = row.get(7)?;
        let events_extracted: i64 = row.get(8)?;
        let contacts_extracted: i64 = row.get(9)?;

        let percent = if total_items > 0 {
            (processed_items as f32 / total_items as f32) * 100.0
        } else {
            0.0
        };

        let source_config_json: String = row.get(10)?;
        let source_config: serde_json::Value =
            serde_json::from_str(&source_config_json).unwrap_or(serde_json::json!({}));

        Ok(ExtractionJob {
            id: row.get(0)?,
            source_type,
            extractor_type,
            status,
            progress: ExtractionProgress {
                total_items: total_items as u64,
                processed_items: processed_items as u64,
                extracted_entities: extracted_entities as u64,
                failed_items: failed_items as u64,
                events_extracted: events_extracted as u64,
                contacts_extracted: contacts_extracted as u64,
                percent_complete: percent,
            },
            source_config,
            error_message: row.get(11)?,
            created_at: row.get(12)?,
            started_at: row.get(13)?,
            updated_at: row.get(14)?,
            completed_at: row.get(15)?,
        })
    })
    .map_err(|e| anyhow::anyhow!("Failed to get extraction job: {}", e))
}

/// List extraction jobs
pub async fn list_extraction_jobs(
    conn: AsyncDbConnection,
    limit: usize,
) -> Result<Vec<ExtractionJob>> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id FROM extraction_jobs ORDER BY created_at DESC LIMIT ?",
    )?;

    let ids = stmt.query_map([limit], |row| row.get::<_, i64>(0))?;

    drop(stmt);
    drop(conn);

    let mut jobs = Vec::new();
    for id_result in ids {
        let id = id_result?;
        if let Ok(job) = get_extraction_job(conn.clone(), id).await {
            jobs.push(job);
        }
    }

    Ok(jobs)
}

/// Update job status
pub async fn update_job_status(
    conn: AsyncDbConnection,
    job_id: i64,
    status: ExtractionJobStatus,
    error_message: Option<String>,
) -> Result<()> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    let status_str = match status {
        ExtractionJobStatus::Pending => "pending",
        ExtractionJobStatus::Running => "running",
        ExtractionJobStatus::Completed => "completed",
        ExtractionJobStatus::Failed => "failed",
        ExtractionJobStatus::Cancelled => "cancelled",
    };

    conn.execute(
        "UPDATE extraction_jobs
         SET status = ?, error_message = ?, updated_at = ?
         WHERE id = ?",
        duckdb::params![status_str, &error_message, now, job_id],
    )?;

    Ok(())
}

/// Update job progress
pub async fn update_job_progress(
    conn: AsyncDbConnection,
    job_id: i64,
    total_items: Option<u64>,
    processed_items: Option<u64>,
    events_extracted: Option<u64>,
    contacts_extracted: Option<u64>,
    failed_items: Option<u64>,
) -> Result<()> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    let mut updates = vec!["updated_at = ?"];
    let mut params: Vec<Box<dyn duckdb::ToSql>> = vec![Box::new(now)];

    if let Some(total) = total_items {
        updates.push("total_items = ?");
        params.push(Box::new(total as i64));
    }
    if let Some(processed) = processed_items {
        updates.push("processed_items = ?");
        params.push(Box::new(processed as i64));
    }
    if let Some(events) = events_extracted {
        updates.push("events_extracted = ?");
        params.push(Box::new(events as i64));
    }
    if let Some(contacts) = contacts_extracted {
        updates.push("contacts_extracted = ?");
        params.push(Box::new(contacts as i64));
    }
    if let Some(failed) = failed_items {
        updates.push("failed_items = ?");
        params.push(Box::new(failed as i64));
    }

    // Calculate extracted_entities
    if events_extracted.is_some() || contacts_extracted.is_some() {
        updates.push("extracted_entities = events_extracted + contacts_extracted");
    }

    params.push(Box::new(job_id));

    let query = format!(
        "UPDATE extraction_jobs SET {} WHERE id = ?",
        updates.join(", ")
    );

    let params_refs: Vec<&dyn duckdb::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    conn.execute(&query, params_refs.as_slice())?;

    Ok(())
}
```

**File**: `dwata-api/src/database/events.rs`

```rust
use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::{Event, CreateEventRequest, UpdateEventRequest};

/// Insert event from extraction
pub async fn insert_event_from_extraction(
    conn: AsyncDbConnection,
    extraction_job_id: i64,
    email_id: Option<i64>,
    name: String,
    description: Option<String>,
    event_date: i64,
    location: Option<String>,
    attendees: Vec<String>,
    confidence: f32,
    requires_review: bool,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    let attendees_json = serde_json::to_string(&attendees)?;

    let id: i64 = conn.query_row(
        "INSERT INTO events
         (extraction_job_id, email_id, name, description, event_date, location, attendees,
          confidence, requires_review, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
        duckdb::params![
            extraction_job_id,
            email_id,
            &name,
            description.as_ref(),
            event_date,
            location.as_ref(),
            &attendees_json,
            confidence,
            requires_review,
            now,
            now
        ],
        |row| row.get(0),
    )?;

    Ok(id)
}

/// Get event by ID
pub async fn get_event(conn: AsyncDbConnection, id: i64) -> Result<Event> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, extraction_job_id, email_id, name, description, event_date, location,
                attendees, confidence, requires_review, is_confirmed, project_id, task_id,
                created_at, updated_at
         FROM events
         WHERE id = ?",
    )?;

    stmt.query_row([id], |row| {
        let attendees_json: String = row.get(7)?;
        let attendees: serde_json::Value =
            serde_json::from_str(&attendees_json).unwrap_or(serde_json::json!([]));

        Ok(Event {
            id: row.get(0)?,
            extraction_job_id: row.get(1)?,
            email_id: row.get(2)?,
            name: row.get(3)?,
            description: row.get(4)?,
            event_date: row.get(5)?,
            location: row.get(6)?,
            attendees,
            confidence: row.get(8)?,
            requires_review: row.get(9)?,
            is_confirmed: row.get(10)?,
            project_id: row.get(11)?,
            task_id: row.get(12)?,
            created_at: row.get(13)?,
            updated_at: row.get(14)?,
        })
    })
    .map_err(|e| anyhow::anyhow!("Failed to get event: {}", e))
}

/// List events
pub async fn list_events(conn: AsyncDbConnection, limit: usize) -> Result<Vec<Event>> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare("SELECT id FROM events ORDER BY event_date DESC LIMIT ?")?;

    let ids = stmt.query_map([limit], |row| row.get::<_, i64>(0))?;

    drop(stmt);
    drop(conn);

    let mut events = Vec::new();
    for id_result in ids {
        let id = id_result?;
        if let Ok(event) = get_event(conn.clone(), id).await {
            events.push(event);
        }
    }

    Ok(events)
}
```

**File**: `dwata-api/src/database/contacts.rs`

```rust
use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::{Contact, CreateContactRequest, UpdateContactRequest};

/// Insert contact from extraction (with duplicate checking)
pub async fn insert_contact_from_extraction(
    conn: AsyncDbConnection,
    extraction_job_id: i64,
    email_id: Option<i64>,
    name: String,
    email: Option<String>,
    phone: Option<String>,
    organization: Option<String>,
    confidence: f32,
    requires_review: bool,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    // Check for duplicate email
    if let Some(email_addr) = &email {
        let existing: Result<i64, _> = conn.query_row(
            "SELECT id FROM contacts WHERE email = ? LIMIT 1",
            [email_addr],
            |row| row.get(0),
        );

        if existing.is_ok() {
            return Err(anyhow::anyhow!("Contact with email {} already exists", email_addr));
        }
    }

    let id: i64 = conn.query_row(
        "INSERT INTO contacts
         (extraction_job_id, email_id, name, email, phone, organization,
          confidence, requires_review, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
        duckdb::params![
            extraction_job_id,
            email_id,
            &name,
            email.as_ref(),
            phone.as_ref(),
            organization.as_ref(),
            confidence,
            requires_review,
            now,
            now
        ],
        |row| row.get(0),
    )?;

    Ok(id)
}

/// Get contact by ID
pub async fn get_contact(conn: AsyncDbConnection, id: i64) -> Result<Contact> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, extraction_job_id, email_id, name, email, phone, organization,
                confidence, requires_review, is_confirmed, is_duplicate, merged_into_contact_id,
                created_at, updated_at
         FROM contacts
         WHERE id = ?",
    )?;

    stmt.query_row([id], |row| {
        Ok(Contact {
            id: row.get(0)?,
            extraction_job_id: row.get(1)?,
            email_id: row.get(2)?,
            name: row.get(3)?,
            email: row.get(4)?,
            phone: row.get(5)?,
            organization: row.get(6)?,
            confidence: row.get(7)?,
            requires_review: row.get(8)?,
            is_confirmed: row.get(9)?,
            is_duplicate: row.get(10)?,
            merged_into_contact_id: row.get(11)?,
            created_at: row.get(12)?,
            updated_at: row.get(13)?,
        })
    })
    .map_err(|e| anyhow::anyhow!("Failed to get contact: {}", e))
}

/// List contacts
pub async fn list_contacts(conn: AsyncDbConnection, limit: usize) -> Result<Vec<Contact>> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare("SELECT id FROM contacts ORDER BY created_at DESC LIMIT ?")?;

    let ids = stmt.query_map([limit], |row| row.get::<_, i64>(0))?;

    drop(stmt);
    drop(conn);

    let mut contacts = Vec::new();
    for id_result in ids {
        let id = id_result?;
        if let Ok(contact) = get_contact(conn.clone(), id).await {
            contacts.push(contact);
        }
    }

    Ok(contacts)
}
```

**File**: `dwata-api/src/database/mod.rs`

Add:
```rust
pub mod extraction_jobs;
pub mod events;
pub mod contacts;
```

### Phase 3: Extraction Manager

**File**: `dwata-api/src/jobs/extraction_manager.rs`

```rust
use crate::database::extraction_jobs as jobs_db;
use crate::database::events as events_db;
use crate::database::contacts as contacts_db;
use crate::database::emails as emails_db;
use crate::database::AsyncDbConnection;
use anyhow::Result;
use extractors::{AttachmentParserExtractor, Extractor};
use shared_types::extraction::{DataType, ExtractionInput, ExtractionResult, ExtractedEntity};
use shared_types::extraction_job::{ExtractionJob, ExtractionJobStatus, ExtractionSourceType, ExtractionSourceConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub struct ExtractionManager {
    db_conn: AsyncDbConnection,
    active_jobs: Arc<Mutex<HashMap<i64, JoinHandle<()>>>>,
}

impl ExtractionManager {
    pub fn new(db_conn: AsyncDbConnection) -> Self {
        Self {
            db_conn,
            active_jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start_job(&self, job_id: i64) -> Result<()> {
        let job = jobs_db::get_extraction_job(self.db_conn.clone(), job_id).await?;

        let active_jobs = self.active_jobs.lock().await;
        if active_jobs.contains_key(&job_id) {
            drop(active_jobs);
            return Err(anyhow::anyhow!("Job already running"));
        }
        drop(active_jobs);

        jobs_db::update_job_status(
            self.db_conn.clone(),
            job_id,
            ExtractionJobStatus::Running,
            None,
        )
        .await?;

        let db_conn = self.db_conn.clone();

        let handle = tokio::spawn(async move {
            if let Err(e) = Self::run_extraction(db_conn.clone(), &job).await {
                tracing::error!("Extraction failed for job {}: {}", job_id, e);
                let _ = jobs_db::update_job_status(
                    db_conn,
                    job_id,
                    ExtractionJobStatus::Failed,
                    Some(e.to_string()),
                )
                .await;
            }
        });

        let mut active_jobs = self.active_jobs.lock().await;
        active_jobs.insert(job_id, handle);

        Ok(())
    }

    async fn run_extraction(db_conn: AsyncDbConnection, job: &ExtractionJob) -> Result<()> {
        tracing::info!("Starting extraction for job {}", job.id);

        let config: ExtractionSourceConfig = serde_json::from_value(job.source_config.clone())?;

        match config {
            ExtractionSourceConfig::EmailAttachments {
                email_ids,
                attachment_types,
                status_filter,
            } => {
                Self::extract_from_email_attachments(
                    db_conn,
                    job.id,
                    email_ids,
                    attachment_types,
                    status_filter,
                )
                .await?;
            }
            ExtractionSourceConfig::LocalFile {
                file_path,
                content_type,
            } => {
                Self::extract_from_local_file(db_conn, job.id, file_path, content_type).await?;
            }
        }

        jobs_db::update_job_status(
            db_conn,
            job.id,
            ExtractionJobStatus::Completed,
            None,
        )
        .await?;

        tracing::info!("Extraction completed for job {}", job.id);
        Ok(())
    }

    async fn extract_from_email_attachments(
        db_conn: AsyncDbConnection,
        job_id: i64,
        email_ids: Option<Vec<i64>>,
        attachment_types: Vec<String>,
        status_filter: shared_types::extraction_job::AttachmentExtractionFilter,
    ) -> Result<()> {
        // Get attachments to process
        let attachments = if let Some(ids) = email_ids {
            // Process specific emails
            let mut all_attachments = Vec::new();
            for email_id in ids {
                let email_attachments = emails_db::get_email_attachments(db_conn.clone(), email_id).await?;
                all_attachments.extend(email_attachments);
            }
            all_attachments
        } else {
            // Process all pending attachments
            emails_db::list_pending_attachments(db_conn.clone(), 1000).await?
        };

        // Filter by content type
        let filtered_attachments: Vec<_> = attachments
            .into_iter()
            .filter(|att| {
                attachment_types.iter().any(|t| att.content_type.contains(t))
            })
            .collect();

        jobs_db::update_job_progress(
            db_conn.clone(),
            job_id,
            Some(filtered_attachments.len() as u64),
            None,
            None,
            None,
            None,
        )
        .await?;

        // Create extractor
        let extractor = AttachmentParserExtractor::with_defaults();

        let mut processed = 0;
        let mut events_count = 0;
        let mut contacts_count = 0;

        for attachment in filtered_attachments {
            // Build extraction input
            let input = ExtractionInput {
                email_id: format!("email_{}", attachment.email_id),
                subject: String::new(),
                body_text: String::new(),
                body_html: None,
                attachments: vec![shared_types::extraction::Attachment {
                    filename: attachment.filename.clone(),
                    content_type: attachment.content_type.clone(),
                    content: std::fs::read(&attachment.file_path)?,
                }],
                sender: shared_types::extraction::EmailAddress {
                    email: String::new(),
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
                user_preferences: shared_types::extraction::UserPreferences {
                    date_format: "YYYY-MM-DD".to_string(),
                    default_task_priority: shared_types::TaskPriority::Medium,
                    default_project_status: shared_types::ProjectStatus::Active,
                    auto_link_threshold: 0.8,
                },
                target_data_type: DataType::Event,
                min_confidence: 0.7,
                max_results: None,
            };

            // Extract entities
            match extractor.extract(&input) {
                Ok(results) => {
                    for result in results {
                        match result.entity {
                            ExtractedEntity::Event(event) => {
                                // Parse event date
                                let event_date = chrono::DateTime::parse_from_rfc3339(&event.date)
                                    .map(|dt| dt.timestamp())
                                    .unwrap_or_else(|_| chrono::Utc::now().timestamp());

                                events_db::insert_event_from_extraction(
                                    db_conn.clone(),
                                    job_id,
                                    Some(attachment.email_id),
                                    event.name,
                                    event.description,
                                    event_date,
                                    event.location,
                                    event.attendees,
                                    result.confidence,
                                    result.requires_review,
                                )
                                .await?;

                                events_count += 1;
                            }
                            ExtractedEntity::Contact(contact) => {
                                match contacts_db::insert_contact_from_extraction(
                                    db_conn.clone(),
                                    job_id,
                                    Some(attachment.email_id),
                                    contact.name,
                                    contact.email,
                                    contact.phone,
                                    contact.organization,
                                    result.confidence,
                                    result.requires_review,
                                )
                                .await
                                {
                                    Ok(_) => contacts_count += 1,
                                    Err(e) => {
                                        tracing::warn!("Skipping duplicate contact: {}", e);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    // Update attachment status
                    emails_db::update_attachment_extraction_status(
                        db_conn.clone(),
                        attachment.id,
                        "completed",
                    )
                    .await?;
                }
                Err(e) => {
                    tracing::error!("Failed to extract from attachment {}: {}", attachment.id, e);

                    emails_db::update_attachment_extraction_status(
                        db_conn.clone(),
                        attachment.id,
                        "failed",
                    )
                    .await?;
                }
            }

            processed += 1;

            // Update progress
            jobs_db::update_job_progress(
                db_conn.clone(),
                job_id,
                None,
                Some(processed),
                Some(events_count),
                Some(contacts_count),
                None,
            )
            .await?;
        }

        Ok(())
    }

    async fn extract_from_local_file(
        db_conn: AsyncDbConnection,
        job_id: i64,
        file_path: String,
        content_type: String,
    ) -> Result<()> {
        // TODO: Implement local file extraction
        tracing::warn!("Local file extraction not yet implemented");
        Ok(())
    }
}
```

**File**: `dwata-api/src/jobs/mod.rs`

Add:
```rust
pub mod extraction_manager;
```

### Phase 4: API Handlers

**File**: `dwata-api/src/handlers/extraction_jobs.rs`

```rust
use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::extraction_job::{
    CreateExtractionJobRequest, ExtractionJobListResponse,
};

use crate::database::extraction_jobs as db;
use crate::jobs::extraction_manager::ExtractionManager;
use crate::database::AsyncDbConnection;

/// POST /api/extractions - Create extraction job
pub async fn create_extraction_job(
    db_conn: web::Data<AsyncDbConnection>,
    request: web::Json<CreateExtractionJobRequest>,
) -> ActixResult<HttpResponse> {
    let job = db::insert_extraction_job(db_conn.as_ref().clone(), &request)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Created().json(job))
}

/// GET /api/extractions - List extraction jobs
pub async fn list_extraction_jobs(
    db_conn: web::Data<AsyncDbConnection>,
) -> ActixResult<HttpResponse> {
    let jobs = db::list_extraction_jobs(db_conn.as_ref().clone(), 50)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ExtractionJobListResponse { jobs }))
}

/// GET /api/extractions/:id - Get extraction job
pub async fn get_extraction_job(
    db_conn: web::Data<AsyncDbConnection>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let job_id = path.into_inner();

    let job = db::get_extraction_job(db_conn.as_ref().clone(), job_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(job))
}

/// POST /api/extractions/:id/start - Start extraction
pub async fn start_extraction(
    manager: web::Data<Arc<ExtractionManager>>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let job_id = path.into_inner();

    manager
        .start_job(job_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "status": "started" })))
}
```

**File**: `dwata-api/src/handlers/events.rs`

```rust
use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::{EventsResponse};

use crate::database::events as db;
use crate::database::AsyncDbConnection;

/// GET /api/events - List events
pub async fn list_events(
    db_conn: web::Data<AsyncDbConnection>,
) -> ActixResult<HttpResponse> {
    let events = db::list_events(db_conn.as_ref().clone(), 100)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(EventsResponse { events }))
}

/// GET /api/events/:id - Get event
pub async fn get_event(
    db_conn: web::Data<AsyncDbConnection>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let event_id = path.into_inner();

    let event = db::get_event(db_conn.as_ref().clone(), event_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(event))
}
```

**File**: `dwata-api/src/handlers/contacts.rs`

```rust
use actix_web::{web, HttpResponse, Result as ActixResult};
use shared_types::{ContactsResponse};

use crate::database::contacts as db;
use crate::database::AsyncDbConnection;

/// GET /api/contacts - List contacts
pub async fn list_contacts(
    db_conn: web::Data<AsyncDbConnection>,
) -> ActixResult<HttpResponse> {
    let contacts = db::list_contacts(db_conn.as_ref().clone(), 100)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ContactsResponse { contacts }))
}

/// GET /api/contacts/:id - Get contact
pub async fn get_contact(
    db_conn: web::Data<AsyncDbConnection>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let contact_id = path.into_inner();

    let contact = db::get_contact(db_conn.as_ref().clone(), contact_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(contact))
}
```

**File**: `dwata-api/src/handlers/mod.rs`

Add:
```rust
pub mod extraction_jobs;
pub mod events;
pub mod contacts;
```

### Phase 5: Route Registration

**File**: `dwata-api/src/main.rs`

Add routes and initialize extraction manager:

```rust
use crate::handlers::{extraction_jobs, events, contacts};
use crate::jobs::extraction_manager::ExtractionManager;

// In main() function, after database connection:
let extraction_manager = Arc::new(ExtractionManager::new(db_conn.async_connection.clone()));

// Add routes
let app = HttpServer::new(move || {
    App::new()
        // ... existing routes ...
        // Extraction jobs
        .route("/api/extractions", web::post().to(extraction_jobs::create_extraction_job))
        .route("/api/extractions", web::get().to(extraction_jobs::list_extraction_jobs))
        .route("/api/extractions/{id}", web::get().to(extraction_jobs::get_extraction_job))
        .route("/api/extractions/{id}/start", web::post().to(extraction_jobs::start_extraction))

        // Events
        .route("/api/events", web::get().to(events::list_events))
        .route("/api/events/{id}", web::get().to(events::get_event))

        // Contacts
        .route("/api/contacts", web::get().to(contacts::list_contacts))
        .route("/api/contacts/{id}", web::get().to(contacts::get_contact))

        .app_data(web::Data::new(db_conn.async_connection.clone()))
        .app_data(web::Data::new(extraction_manager.clone()))
})
.bind(("127.0.0.1", 8080))?
.run()
.await
```

### Phase 6: TypeScript Type Generation

Run type generation:
```bash
cd shared-types
cargo test
```

This generates TypeScript types for all `#[ts(export)]` annotated types.

## Testing Strategy

### Manual Testing

1. **Create extraction job**:
   ```bash
   curl -X POST http://localhost:8080/api/extractions \
     -H "Content-Type: application/json" \
     -d '{
       "source_type": "email-attachment",
       "extractor_type": "attachment-parser",
       "source_config": {
         "type": "EmailAttachments",
         "config": {
           "email_ids": null,
           "attachment_types": ["text/calendar", "text/vcard"],
           "status_filter": "pending"
         }
       }
     }'
   ```

2. **Start extraction**:
   ```bash
   curl -X POST http://localhost:8080/api/extractions/1/start
   ```

3. **Check progress**:
   ```bash
   curl http://localhost:8080/api/extractions/1
   ```

4. **List extracted events**:
   ```bash
   curl http://localhost:8080/api/events
   ```

5. **List extracted contacts**:
   ```bash
   curl http://localhost:8080/api/contacts
   ```

## Success Criteria

- ✅ Database schema created (extraction_jobs, events, contacts)
- ✅ Shared types created and exported
- ✅ Database operations implemented
- ✅ Extraction manager implemented
- ✅ API handlers implemented
- ✅ Routes registered
- ✅ TypeScript types generated
- ✅ Can create extraction jobs
- ✅ Can start extraction and process attachments
- ✅ Events and contacts saved to database
- ✅ Extraction jobs reference source data
- ✅ Can query extracted entities via API

## Future Enhancements

1. **Additional Extractors**:
   - GLiNER NER extractor for email body text
   - LLM-based extractor for complex reasoning
   - Pattern-based extractor for common formats

2. **Entity Linking**:
   - Link events to projects/tasks
   - Link contacts to emails
   - Automatic deduplication

3. **Review UI**:
   - Show low-confidence extractions for review
   - Allow user corrections
   - Re-training feedback loop

4. **Batch Processing**:
   - Process multiple emails in parallel
   - Configurable batch sizes
   - Priority queue for important emails

---

**Document Version**: 1.1
**Created**: 2026-01-30
**Status**: Implementation Complete
