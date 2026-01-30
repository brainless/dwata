# Task: Email Storage Schema and Real IMAP Download Implementation with Date Filtering

## Objective

Implement proper email storage with dedicated database schema and real IMAP functionality using the `imap` crate, with configurable date filtering (default: 12 months) to ensure only recent emails are downloaded.

## Background

### Current State - Critical Findings
- **Download Manager EXISTS** but is **NOT FUNCTIONAL**
- `NocodoImapClient` is a **stub** that returns hardcoded data
- `fetch_email()` just returns `EmailContent { uid }` - no actual email download
- `search_emails()` returns `vec![1, 2, 3, 4, 5]` - hardcoded UIDs
- **Database shows**: Job status = 'completed', `downloaded_items=1`, BUT `download_items` table is **EMPTY (0 rows)**
- **No code exists** to INSERT emails into database
- **No date filtering** implemented - would download ALL emails if it worked
- `imap` crate (v3.0.0-alpha.15) is a dependency but **not being used**

### Why This Matters
1. Users have logged into 2 email accounts (credentials stored in OS keychain)
2. Need to download emails from last 12 months ONLY (configurable for future)
3. Emails need proper storage structure (not generic `download_items`)
4. TypeScript types must be auto-generated for GUI

### Requirements
1. **Email Storage Schema**: Dedicated `emails` table with proper columns for email data
2. **Real IMAP Client**: Use `imap` crate to actually connect and download emails
3. **Date Filtering**: Only download emails from last N months (default: 12, configurable)
4. **OAuth Support**: Handle both password-based and OAuth2 IMAP authentication
5. **Email Parsing**: Extract headers, body, attachments, and metadata
6. **Database Relations**: Link emails to download_items for tracking
7. **TypeScript Types**: Auto-generate types for GUI via `shared-types` crate

## Architecture

### Data Flow

```
┌────────────────────────────────────────────────────────────┐
│                  DownloadManager (existing)                 │
│  - Orchestrates download jobs                               │
│  - Tracks progress in download_jobs table                   │
└────────────────────────────────────────────────────────────┘
                          ↓
┌────────────────────────────────────────────────────────────┐
│              RealImapClient (NEW - replaces stub)           │
│  - Uses `imap` crate for actual IMAP protocol              │
│  - Supports OAuth2 (Gmail) and password auth               │
│  - Implements date filtering (SINCE <date>)                │
│  - Parses email with mail-parser crate                     │
└────────────────────────────────────────────────────────────┘
                          ↓
┌────────────────────────────────────────────────────────────┐
│              Email Parser & Storage (NEW)                   │
│  - Parse MIME email structure                               │
│  - Extract headers, body, attachments                       │
│  - Store in `emails` table                                  │
│  - Create download_items entry for tracking                │
└────────────────────────────────────────────────────────────┘
                          ↓
┌────────────────────────────────────────────────────────────┐
│                    DuckDB Database                          │
│  - emails table (email content)                             │
│  - download_items table (tracking relation)                 │
│  - download_jobs table (progress tracking)                  │
└────────────────────────────────────────────────────────────┘
```

### Date Filtering Strategy

```
User Request: "Download emails"
     ↓
Check ImapDownloadState.max_age_months (default: 12)
     ↓
Calculate cutoff date: today - max_age_months
     ↓
IMAP SEARCH query: SINCE <cutoff_date>
     ↓
Only download UIDs matching date criteria
```

### Relationship Design: Emails vs Download Tracking

**Key Principle: Emails are permanent, tracking is ephemeral**

```
download_items (Ephemeral tracking)
     ↓ (soft reference, no FK)
     │
emails (Permanent data)
  └── download_item_id: Option<i64>  (nullable, no CASCADE)
```

**Why this design:**
- `download_items` = Operational tracking (retries, errors, progress)
- `emails` = Business data (permanent record)
- Deleting old tracking records does NOT delete emails
- Emails can optionally reference their tracking for auditing

**Example:**
1. Download creates `download_item` with id=42
2. Email stored with `download_item_id = Some(42)`
3. Later: Clean up old tracking → DELETE from download_items WHERE id=42
4. Result: Email persists with `download_item_id = Some(42)` (orphaned reference, but email intact)

## Database Schema

### New `emails` Table

```sql
CREATE TABLE IF NOT EXISTS emails (
    id INTEGER PRIMARY KEY,                         -- Auto-increment ID
    download_item_id INTEGER,                       -- Soft reference to download_items (no FK constraint)
                                                     -- Emails are permanent; download tracking is ephemeral

    -- IMAP Metadata
    uid INTEGER NOT NULL,                           -- IMAP UID
    folder VARCHAR NOT NULL,                        -- INBOX, Sent, etc.
    message_id VARCHAR,                             -- RFC822 Message-ID header

    -- Email Headers
    subject VARCHAR,
    from_address VARCHAR NOT NULL,                  -- Sender email
    from_name VARCHAR,                              -- Sender display name
    to_addresses VARCHAR,                           -- JSON array of recipients
    cc_addresses VARCHAR,                           -- JSON array of CC
    bcc_addresses VARCHAR,                          -- JSON array of BCC
    reply_to VARCHAR,                               -- Reply-To header

    -- Dates
    date_sent BIGINT,                               -- Date header (timestamp millis)
    date_received BIGINT NOT NULL,                  -- IMAP INTERNALDATE (timestamp millis)

    -- Content
    body_text VARCHAR,                              -- Plain text body
    body_html VARCHAR,                              -- HTML body

    -- Flags & Status
    is_read BOOLEAN DEFAULT false,                  -- IMAP \Seen flag
    is_flagged BOOLEAN DEFAULT false,               -- IMAP \Flagged
    is_draft BOOLEAN DEFAULT false,                 -- IMAP \Draft
    is_answered BOOLEAN DEFAULT false,              -- IMAP \Answered

    -- Metadata
    has_attachments BOOLEAN DEFAULT false,
    attachment_count INTEGER DEFAULT 0,
    size_bytes INTEGER,                             -- RFC822.SIZE
    thread_id VARCHAR,                              -- For threading (future)
    labels VARCHAR,                                 -- JSON array of labels/tags

    -- Timestamps
    created_at BIGINT NOT NULL,                     -- When stored in DB
    updated_at BIGINT NOT NULL

    -- NOTE: No FK constraint - emails persist independently of download tracking
);

CREATE INDEX IF NOT EXISTS idx_emails_download_item
    ON emails(download_item_id);

CREATE INDEX IF NOT EXISTS idx_emails_folder_date
    ON emails(folder, date_received DESC);

CREATE INDEX IF NOT EXISTS idx_emails_message_id
    ON emails(message_id);

CREATE INDEX IF NOT EXISTS idx_emails_from
    ON emails(from_address);

CREATE INDEX IF NOT EXISTS idx_emails_date_sent
    ON emails(date_sent DESC);
```

### New `email_attachments` Table

```sql
CREATE TABLE IF NOT EXISTS email_attachments (
    id INTEGER PRIMARY KEY,                         -- Auto-increment ID
    email_id INTEGER NOT NULL,                      -- FK to emails table

    -- Attachment Metadata
    filename VARCHAR NOT NULL,
    content_type VARCHAR,                           -- MIME type
    size_bytes INTEGER,
    content_id VARCHAR,                             -- For inline images

    -- Storage
    file_path VARCHAR NOT NULL,                     -- Local filesystem path
    checksum VARCHAR,                               -- SHA256 for deduplication

    -- Processing Status
    is_inline BOOLEAN DEFAULT false,                -- Inline vs attachment
    extraction_status VARCHAR DEFAULT 'pending',    -- pending, completed, failed
    extracted_text VARCHAR,                         -- For future extraction pipeline

    -- Timestamps
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,

    FOREIGN KEY (email_id) REFERENCES emails (id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_attachments_email
    ON email_attachments(email_id);

CREATE INDEX IF NOT EXISTS idx_attachments_checksum
    ON email_attachments(checksum);
```

### Update to `ImapDownloadState` Type

Add `max_age_months` field to control date filtering:

```rust
/// IMAP-specific download state (in shared-types/src/download.rs)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ImapDownloadState {
    pub folders: Vec<ImapFolderStatus>,
    pub sync_strategy: ImapSyncStrategy,
    pub last_highest_uid: HashMap<String, u32>,
    pub fetch_batch_size: usize,

    // NEW: Date filtering
    pub max_age_months: Option<u32>,                // Default: 12 months
}
```

## Type Definitions

### Shared Types (shared-types/src/email.rs) - NEW FILE

```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Represents a stored email
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Email {
    pub id: i64,
    pub download_item_id: Option<i64>,  // Soft reference - can be null if tracking deleted

    // IMAP Metadata
    pub uid: u32,
    pub folder: String,
    pub message_id: Option<String>,

    // Headers
    pub subject: Option<String>,
    pub from_address: String,
    pub from_name: Option<String>,
    pub to_addresses: Vec<EmailAddress>,
    pub cc_addresses: Vec<EmailAddress>,
    pub bcc_addresses: Vec<EmailAddress>,
    pub reply_to: Option<String>,

    // Dates
    pub date_sent: Option<i64>,
    pub date_received: i64,

    // Content
    pub body_text: Option<String>,
    pub body_html: Option<String>,

    // Flags
    pub is_read: bool,
    pub is_flagged: bool,
    pub is_draft: bool,
    pub is_answered: bool,

    // Metadata
    pub has_attachments: bool,
    pub attachment_count: i32,
    pub size_bytes: Option<i32>,
    pub thread_id: Option<String>,
    pub labels: Vec<String>,

    // Timestamps
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EmailAddress {
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EmailAttachment {
    pub id: i64,
    pub email_id: i64,
    pub filename: String,
    pub content_type: Option<String>,
    pub size_bytes: Option<i32>,
    pub content_id: Option<String>,
    pub file_path: String,
    pub checksum: Option<String>,
    pub is_inline: bool,
    pub extraction_status: AttachmentExtractionStatus,
    pub extracted_text: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum AttachmentExtractionStatus {
    Pending,
    Completed,
    Failed,
    Skipped,
}

/// Request to list emails
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct ListEmailsRequest {
    pub folder: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub search_query: Option<String>,
}

/// Response for email list
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ListEmailsResponse {
    pub emails: Vec<Email>,
    pub total_count: i64,
    pub has_more: bool,
}
```

**File**: `shared-types/src/lib.rs`

Add:
```rust
pub mod email;

pub use email::{
    Email, EmailAddress, EmailAttachment, AttachmentExtractionStatus,
    ListEmailsRequest, ListEmailsResponse,
};
```

## Implementation Plan

### Phase 1: Database Schema & Migrations

**File**: `dwata-api/src/database/migrations.rs`

Add to `run_migrations()`:

```rust
// Create emails table
conn.execute(
    "CREATE SEQUENCE IF NOT EXISTS seq_emails_id START 1",
    [],
)?;

conn.execute(
    "CREATE TABLE IF NOT EXISTS emails (
        id INTEGER PRIMARY KEY DEFAULT nextval('seq_emails_id'),
        download_item_id INTEGER,
        uid INTEGER NOT NULL,
        folder VARCHAR NOT NULL,
        message_id VARCHAR,
        subject VARCHAR,
        from_address VARCHAR NOT NULL,
        from_name VARCHAR,
        to_addresses VARCHAR,
        cc_addresses VARCHAR,
        bcc_addresses VARCHAR,
        reply_to VARCHAR,
        date_sent BIGINT,
        date_received BIGINT NOT NULL,
        body_text VARCHAR,
        body_html VARCHAR,
        is_read BOOLEAN DEFAULT false,
        is_flagged BOOLEAN DEFAULT false,
        is_draft BOOLEAN DEFAULT false,
        is_answered BOOLEAN DEFAULT false,
        has_attachments BOOLEAN DEFAULT false,
        attachment_count INTEGER DEFAULT 0,
        size_bytes INTEGER,
        thread_id VARCHAR,
        labels VARCHAR,
        created_at BIGINT NOT NULL,
        updated_at BIGINT NOT NULL
    )",
    [],
)?;

conn.execute(
    "CREATE INDEX IF NOT EXISTS idx_emails_download_item ON emails(download_item_id)",
    [],
)?;

conn.execute(
    "CREATE INDEX IF NOT EXISTS idx_emails_folder_date ON emails(folder, date_received DESC)",
    [],
)?;

conn.execute(
    "CREATE INDEX IF NOT EXISTS idx_emails_message_id ON emails(message_id)",
    [],
)?;

conn.execute(
    "CREATE INDEX IF NOT EXISTS idx_emails_from ON emails(from_address)",
    [],
)?;

conn.execute(
    "CREATE INDEX IF NOT EXISTS idx_emails_date_sent ON emails(date_sent DESC)",
    [],
)?;

// Create email_attachments table
conn.execute(
    "CREATE SEQUENCE IF NOT EXISTS seq_email_attachments_id START 1",
    [],
)?;

conn.execute(
    "CREATE TABLE IF NOT EXISTS email_attachments (
        id INTEGER PRIMARY KEY DEFAULT nextval('seq_email_attachments_id'),
        email_id INTEGER NOT NULL,
        filename VARCHAR NOT NULL,
        content_type VARCHAR,
        size_bytes INTEGER,
        content_id VARCHAR,
        file_path VARCHAR NOT NULL,
        checksum VARCHAR,
        is_inline BOOLEAN DEFAULT false,
        extraction_status VARCHAR DEFAULT 'pending',
        extracted_text VARCHAR,
        created_at BIGINT NOT NULL,
        updated_at BIGINT NOT NULL,
        FOREIGN KEY (email_id) REFERENCES emails (id) ON DELETE CASCADE
    )",
    [],
)?;

conn.execute(
    "CREATE INDEX IF NOT EXISTS idx_attachments_email ON email_attachments(email_id)",
    [],
)?;

conn.execute(
    "CREATE INDEX IF NOT EXISTS idx_attachments_checksum ON email_attachments(checksum)",
    [],
)?;
```

### Phase 2: Email Database Operations

**File**: `dwata-api/src/database/emails.rs` - NEW FILE

```rust
use duckdb::Connection;
use shared_types::email::{Email, EmailAddress, EmailAttachment, AttachmentExtractionStatus};
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::Result;

pub type AsyncDbConnection = Arc<Mutex<Connection>>;

/// Insert email into database
pub async fn insert_email(
    conn: AsyncDbConnection,
    download_item_id: Option<i64>,  // Optional - can be None if not tracked
    uid: u32,
    folder: &str,
    message_id: Option<&str>,
    subject: Option<&str>,
    from_address: &str,
    from_name: Option<&str>,
    to_addresses: &[EmailAddress],
    cc_addresses: &[EmailAddress],
    bcc_addresses: &[EmailAddress],
    reply_to: Option<&str>,
    date_sent: Option<i64>,
    date_received: i64,
    body_text: Option<&str>,
    body_html: Option<&str>,
    is_read: bool,
    is_flagged: bool,
    is_draft: bool,
    is_answered: bool,
    has_attachments: bool,
    attachment_count: i32,
    size_bytes: Option<i32>,
    labels: &[String],
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    let to_json = serde_json::to_string(to_addresses)?;
    let cc_json = serde_json::to_string(cc_addresses)?;
    let bcc_json = serde_json::to_string(bcc_addresses)?;
    let labels_json = serde_json::to_string(labels)?;

    conn.execute(
        "INSERT INTO emails
         (download_item_id, uid, folder, message_id, subject, from_address, from_name,
          to_addresses, cc_addresses, bcc_addresses, reply_to, date_sent, date_received,
          body_text, body_html, is_read, is_flagged, is_draft, is_answered,
          has_attachments, attachment_count, size_bytes, labels, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        duckdb::params![
            download_item_id, uid as i32, folder, message_id, subject, from_address, from_name,
            &to_json, &cc_json, &bcc_json, reply_to, date_sent, date_received,
            body_text, body_html, is_read, is_flagged, is_draft, is_answered,
            has_attachments, attachment_count, size_bytes, &labels_json, now, now
        ],
    )?;

    let email_id: i64 = conn.query_row(
        "SELECT last_insert_rowid()",
        [],
        |row| row.get(0)
    )?;

    Ok(email_id)
}

/// Get email by ID
pub async fn get_email(
    conn: AsyncDbConnection,
    email_id: i64,
) -> Result<Email> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, download_item_id, uid, folder, message_id, subject, from_address, from_name,
                to_addresses, cc_addresses, bcc_addresses, reply_to, date_sent, date_received,
                body_text, body_html, is_read, is_flagged, is_draft, is_answered,
                has_attachments, attachment_count, size_bytes, thread_id, labels,
                created_at, updated_at
         FROM emails WHERE id = ?"
    )?;

    let email = stmt.query_row([email_id], |row| {
        let to_json: String = row.get(8)?;
        let cc_json: String = row.get(9)?;
        let bcc_json: String = row.get(10)?;
        let labels_json: String = row.get(24)?;

        Ok(Email {
            id: row.get(0)?,
            download_item_id: row.get(1)?,
            uid: row.get::<_, i32>(2)? as u32,
            folder: row.get(3)?,
            message_id: row.get(4)?,
            subject: row.get(5)?,
            from_address: row.get(6)?,
            from_name: row.get(7)?,
            to_addresses: serde_json::from_str(&to_json).unwrap_or_default(),
            cc_addresses: serde_json::from_str(&cc_json).unwrap_or_default(),
            bcc_addresses: serde_json::from_str(&bcc_json).unwrap_or_default(),
            reply_to: row.get(11)?,
            date_sent: row.get(12)?,
            date_received: row.get(13)?,
            body_text: row.get(14)?,
            body_html: row.get(15)?,
            is_read: row.get(16)?,
            is_flagged: row.get(17)?,
            is_draft: row.get(18)?,
            is_answered: row.get(19)?,
            has_attachments: row.get(20)?,
            attachment_count: row.get(21)?,
            size_bytes: row.get(22)?,
            thread_id: row.get(23)?,
            labels: serde_json::from_str(&labels_json).unwrap_or_default(),
            created_at: row.get(25)?,
            updated_at: row.get(26)?,
        })
    })?;

    Ok(email)
}

/// List emails with pagination
pub async fn list_emails(
    conn: AsyncDbConnection,
    folder: Option<&str>,
    limit: usize,
    offset: usize,
) -> Result<Vec<Email>> {
    let conn = conn.lock().await;

    let query = if let Some(f) = folder {
        format!(
            "SELECT id FROM emails WHERE folder = '{}'
             ORDER BY date_received DESC LIMIT {} OFFSET {}",
            f, limit, offset
        )
    } else {
        format!(
            "SELECT id FROM emails ORDER BY date_received DESC
             LIMIT {} OFFSET {}",
            limit, offset
        )
    };

    let mut stmt = conn.prepare(&query)?;
    let ids: Vec<i64> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    drop(conn);

    let mut emails = Vec::new();
    for id in ids {
        if let Ok(email) = get_email(conn.clone(), id).await {
            emails.push(email);
        }
    }

    Ok(emails)
}

/// Insert attachment
pub async fn insert_attachment(
    conn: AsyncDbConnection,
    email_id: i64,
    filename: &str,
    content_type: Option<&str>,
    size_bytes: Option<i32>,
    content_id: Option<&str>,
    file_path: &str,
    checksum: Option<&str>,
    is_inline: bool,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "INSERT INTO email_attachments
         (email_id, filename, content_type, size_bytes, content_id, file_path, checksum,
          is_inline, extraction_status, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        duckdb::params![
            email_id, filename, content_type, size_bytes, content_id, file_path, checksum,
            is_inline, "pending", now, now
        ],
    )?;

    let attachment_id: i64 = conn.query_row(
        "SELECT last_insert_rowid()",
        [],
        |row| row.get(0)
    )?;

    Ok(attachment_id)
}
```

**File**: `dwata-api/src/database/mod.rs`

Add:
```rust
pub mod emails;
```

### Phase 3: Real IMAP Client Implementation

**File**: `dwata-api/src/integrations/real_imap_client.rs` - NEW FILE

```rust
use anyhow::{Result, Context};
use native_tls::TlsConnector;
use imap::Session;
use mail_parser::MessageParser;
use chrono::{Utc, Duration};
use std::net::TcpStream;

pub struct RealImapClient {
    session: Session<native_tls::TlsStream<TcpStream>>,
}

impl RealImapClient {
    /// Connect with password authentication
    pub async fn connect_with_password(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<Self> {
        let tls = TlsConnector::builder().build()?;
        let client = imap::connect((host, port), host, &tls)
            .context("Failed to connect to IMAP server")?;

        let session = client
            .login(username, password)
            .map_err(|e| anyhow::anyhow!("IMAP login failed: {:?}", e))?;

        Ok(Self { session })
    }

    /// Connect with OAuth2 (for Gmail)
    pub async fn connect_with_oauth(
        host: &str,
        port: u16,
        username: &str,
        access_token: &str,
    ) -> Result<Self> {
        let tls = TlsConnector::builder().build()?;
        let client = imap::connect((host, port), host, &tls)
            .context("Failed to connect to IMAP server")?;

        let session = client
            .authenticate("XOAUTH2", |_challenge| {
                format!(
                    "user={}\x01auth=Bearer {}\x01\x01",
                    username, access_token
                )
            })
            .map_err(|e| anyhow::anyhow!("OAuth2 IMAP auth failed: {:?}", e))?;

        Ok(Self { session })
    }

    /// List all mailboxes
    pub fn list_mailboxes(&mut self) -> Result<Vec<String>> {
        let mailboxes = self.session.list(None, Some("*"))?;
        Ok(mailboxes
            .iter()
            .map(|m| m.name().to_string())
            .collect())
    }

    /// Get mailbox status (total message count)
    pub fn mailbox_status(&mut self, mailbox: &str) -> Result<u32> {
        self.session.select(mailbox)?;
        let status = self.session.status(mailbox, "(MESSAGES)")?;
        Ok(status.messages.unwrap_or(0))
    }

    /// Search emails with date filter
    pub fn search_emails(
        &mut self,
        mailbox: &str,
        since_uid: Option<u32>,
        max_age_months: Option<u32>,
        limit: Option<usize>,
    ) -> Result<Vec<u32>> {
        self.session.select(mailbox)?;

        // Calculate cutoff date
        let cutoff_date = if let Some(months) = max_age_months {
            let date = Utc::now() - Duration::days((months as i64) * 30);
            Some(date)
        } else {
            None
        };

        // Build search query
        let mut query = String::new();

        if let Some(date) = cutoff_date {
            let date_str = date.format("%d-%b-%Y").to_string();
            query.push_str(&format!("SINCE {}", date_str));
        } else {
            query.push_str("ALL");
        }

        if let Some(uid) = since_uid {
            if !query.is_empty() {
                query.push(' ');
            }
            query.push_str(&format!("UID {}:*", uid + 1));
        }

        tracing::info!("IMAP SEARCH query: {}", query);

        let uids = self.session.uid_search(&query)?;

        // Apply limit if specified
        let limited_uids: Vec<u32> = if let Some(lim) = limit {
            uids.into_iter().take(lim).collect()
        } else {
            uids
        };

        Ok(limited_uids)
    }

    /// Fetch email by UID and parse
    pub fn fetch_email(&mut self, mailbox: &str, uid: u32) -> Result<ParsedEmail> {
        self.session.select(mailbox)?;

        let messages = self.session.uid_fetch(uid.to_string(), "RFC822")?;

        let message = messages.iter().next()
            .context("Email not found")?;

        let body = message.body()
            .context("Email has no body")?;

        // Parse email using mail-parser
        let parser = MessageParser::default();
        let parsed = parser.parse(body)
            .context("Failed to parse email")?;

        // Extract fields
        let subject = parsed.subject().map(|s| s.to_string());
        let from = parsed.from()
            .and_then(|addrs| addrs.first())
            .map(|addr| (
                addr.address().map(|a| a.to_string()),
                addr.name().map(|n| n.to_string()),
            ));

        let to_addresses: Vec<(Option<String>, Option<String>)> = parsed.to()
            .map(|addrs| {
                addrs.iter()
                    .map(|addr| (
                        addr.address().map(|a| a.to_string()),
                        addr.name().map(|n| n.to_string()),
                    ))
                    .collect()
            })
            .unwrap_or_default();

        let body_text = parsed.body_text(0).map(|s| s.to_string());
        let body_html = parsed.body_html(0).map(|s| s.to_string());

        let date_sent = parsed.date()
            .map(|dt| dt.to_timestamp() * 1000);

        let message_id = parsed.message_id().map(|s| s.to_string());

        // Extract flags
        let flags = message.flags();
        let is_read = flags.contains(&imap::types::Flag::Seen);
        let is_flagged = flags.contains(&imap::types::Flag::Flagged);
        let is_draft = flags.contains(&imap::types::Flag::Draft);
        let is_answered = flags.contains(&imap::types::Flag::Answered);

        // Get INTERNALDATE
        let date_received = message.internal_date()
            .map(|dt| dt.timestamp_millis())
            .unwrap_or_else(|| Utc::now().timestamp_millis());

        // Get size
        let size_bytes = message.size();

        // Check attachments
        let has_attachments = parsed.attachment_count() > 0;
        let attachment_count = parsed.attachment_count();

        Ok(ParsedEmail {
            uid,
            message_id,
            subject,
            from_address: from.as_ref().and_then(|(addr, _)| addr.clone()),
            from_name: from.and_then(|(_, name)| name),
            to_addresses,
            cc_addresses: vec![],  // TODO: Parse CC
            bcc_addresses: vec![], // TODO: Parse BCC
            reply_to: None,        // TODO: Parse Reply-To
            date_sent,
            date_received,
            body_text,
            body_html,
            is_read,
            is_flagged,
            is_draft,
            is_answered,
            has_attachments,
            attachment_count: attachment_count as i32,
            size_bytes: size_bytes.map(|s| s as i32),
            labels: vec![],
        })
    }
}

pub struct ParsedEmail {
    pub uid: u32,
    pub message_id: Option<String>,
    pub subject: Option<String>,
    pub from_address: Option<String>,
    pub from_name: Option<String>,
    pub to_addresses: Vec<(Option<String>, Option<String>)>,
    pub cc_addresses: Vec<(Option<String>, Option<String>)>,
    pub bcc_addresses: Vec<(Option<String>, Option<String>)>,
    pub reply_to: Option<String>,
    pub date_sent: Option<i64>,
    pub date_received: i64,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub is_read: bool,
    pub is_flagged: bool,
    pub is_draft: bool,
    pub is_answered: bool,
    pub has_attachments: bool,
    pub attachment_count: i32,
    pub size_bytes: Option<i32>,
    pub labels: Vec<String>,
}
```

**File**: `dwata-api/src/integrations/mod.rs`

Add:
```rust
pub mod real_imap_client;
```

### Phase 4: Update Download Manager

**File**: `dwata-api/src/jobs/download_manager.rs`

Replace the stub IMAP client usage with real implementation:

```rust
// Around line 121-145, replace NocodoImapClient with RealImapClient

use crate::integrations::real_imap_client::RealImapClient;
use crate::database::emails as emails_db;
use crate::database::downloads as downloads_db;

// In run_imap_download function:
let imap_client = if credential.credential_type == CredentialType::OAuth {
    let access_token = get_access_token_for_imap(
        credential.id,
        &credential,
        &token_cache,
        &oauth_client,
    )
    .await?;

    RealImapClient::connect_with_oauth(
        &credential.service_name.unwrap_or("imap.gmail.com".to_string()),
        credential.port.unwrap_or(993) as u16,
        &credential.username,
        &access_token,
    )
    .await?
} else {
    let password = KeyringService::get_password(
        &credential.credential_type,
        &credential.identifier,
        &credential.username,
    )?;

    RealImapClient::connect_with_password(
        &credential.service_name.unwrap_or_default(),
        credential.port.unwrap_or(993) as u16,
        &credential.username,
        &password,
    )
    .await?
};

// Get max_age_months from state (default: 12)
let max_age_months = state.max_age_months.or(Some(12));

// In the download loop (around line 164-166):
let uids = imap_client
    .search_emails(
        &folder.name,
        resume_uid,
        max_age_months,  // NEW: Pass date filter
        Some(state.fetch_batch_size)
    )?;

// In fetch_email loop (around line 174-203), ACTUALLY STORE THE EMAIL:
match imap_client.fetch_email(&folder.name, uid) {
    Ok(parsed_email) => {
        // 1. Create download_item entry first
        let download_item_id = downloads_db::insert_download_item(
            db_conn.clone(),
            job.id,
            &uid.to_string(),  // source_identifier
            Some(&folder.name),
            "email",
            "completed",
            parsed_email.size_bytes.map(|s| s as i64),
            Some("message/rfc822"),
            None,
        ).await?;

        // 2. Store email in emails table
        let to_addresses: Vec<EmailAddress> = parsed_email.to_addresses
            .iter()
            .filter_map(|(addr, name)| {
                addr.as_ref().map(|a| EmailAddress {
                    email: a.clone(),
                    name: name.clone(),
                })
            })
            .collect();

        let email_id = emails_db::insert_email(
            db_conn.clone(),
            Some(download_item_id),  // Link to tracking record
            parsed_email.uid,
            &folder.name,
            parsed_email.message_id.as_deref(),
            parsed_email.subject.as_deref(),
            &parsed_email.from_address.unwrap_or_default(),
            parsed_email.from_name.as_deref(),
            &to_addresses,
            &[],  // CC
            &[],  // BCC
            parsed_email.reply_to.as_deref(),
            parsed_email.date_sent,
            parsed_email.date_received,
            parsed_email.body_text.as_deref(),
            parsed_email.body_html.as_deref(),
            parsed_email.is_read,
            parsed_email.is_flagged,
            parsed_email.is_draft,
            parsed_email.is_answered,
            parsed_email.has_attachments,
            parsed_email.attachment_count,
            parsed_email.size_bytes,
            &parsed_email.labels,
        ).await?;

        tracing::info!("Downloaded and stored email UID {} (id: {})", uid, email_id);

        // Update progress
        db::update_job_progress(
            db_conn.clone(),
            job.id,
            None,
            Some(1),
            None,
            None,
            parsed_email.size_bytes.map(|s| s as u64),
        ).await?;
    }
    Err(e) => {
        tracing::error!("Failed to download email UID {}: {}", uid, e);

        // Increment failed count
        db::update_job_progress(
            db_conn.clone(),
            job.id,
            None,
            None,
            Some(1),
            None,
            None,
        ).await?;
    }
}
```

### Phase 5: Add Missing Database Function

**File**: `dwata-api/src/database/downloads.rs`

Add function to insert download_item:

```rust
/// Insert download item
pub async fn insert_download_item(
    conn: Arc<Mutex<duckdb::Connection>>,
    job_id: i64,
    source_identifier: &str,
    source_folder: Option<&str>,
    item_type: &str,
    status: &str,
    size_bytes: Option<i64>,
    mime_type: Option<&str>,
    metadata: Option<serde_json::Value>,
) -> Result<i64, DownloadDbError> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    let metadata_json = metadata.map(|m| serde_json::to_string(&m).ok()).flatten();

    conn.execute(
        "INSERT INTO download_items
         (job_id, source_identifier, source_folder, item_type, status, size_bytes,
          mime_type, metadata, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        duckdb::params![
            job_id as i32,
            source_identifier,
            source_folder,
            item_type,
            status,
            size_bytes.map(|s| s as i32),
            mime_type,
            metadata_json.as_deref(),
            now,
            now
        ],
    ).map_err(|e| DownloadDbError::DatabaseError(e.to_string()))?;

    let item_id: i64 = conn.query_row(
        "SELECT last_insert_rowid()",
        [],
        |row| row.get(0)
    ).map_err(|e| DownloadDbError::DatabaseError(e.to_string()))?;

    Ok(item_id)
}
```

### Phase 6: Add Dependencies

**File**: `dwata-api/Cargo.toml`

Add:
```toml
[dependencies]
mail-parser = "0.9"  # For parsing email MIME structure
```

### Phase 7: Update ImapDownloadState Default

**File**: `shared-types/src/download.rs`

Update the `ImapDownloadState` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ImapDownloadState {
    pub folders: Vec<ImapFolderStatus>,
    pub sync_strategy: ImapSyncStrategy,
    pub last_highest_uid: HashMap<String, u32>,
    pub fetch_batch_size: usize,

    #[serde(default = "default_max_age_months")]
    pub max_age_months: Option<u32>,  // Default: 12 months
}

fn default_max_age_months() -> Option<u32> {
    Some(12)
}
```

### Phase 8: TypeScript Type Generation

Run after all changes:
```bash
cd /Users/brainless/Projects/dwata/shared-types
cargo run --bin generate_api_types
```

## Testing Strategy

### Manual Testing Checklist

1. **Database Schema**
   ```bash
   duckdb ~/Library/Application\ Support/dwata/db.duckdb "SHOW TABLES;"
   # Verify: emails, email_attachments exist

   duckdb ~/Library/Application\ Support/dwata/db.duckdb "DESCRIBE emails;"
   # Verify all columns present
   ```

2. **Start Download with Date Filter**
   - Use existing download job or create new one
   - Start download via API
   - Check logs for "IMAP SEARCH query: SINCE <date>"
   - Verify only emails from last 12 months are downloaded

3. **Verify Email Storage**
   ```bash
   duckdb ~/Library/Application\ Support/dwata/db.duckdb "SELECT COUNT(*) FROM emails;"
   # Should show actual email count, not 0

   duckdb ~/Library/Application\ Support/dwata/db.duckdb "SELECT id, subject, from_address, date_received FROM emails LIMIT 5;"
   # Should show real email data
   ```

4. **Check download_items Relation**
   ```bash
   duckdb ~/Library/Application\ Support/dwata/db.duckdb "
   SELECT di.id, di.source_identifier, e.subject
   FROM download_items di
   JOIN emails e ON e.download_item_id = di.id
   LIMIT 5;"
   # Should show linked data
   ```

5. **Test OAuth vs Password Auth**
   - Test with Gmail (OAuth2)
   - Test with regular IMAP (password)
   - Verify both work correctly

6. **Test Different max_age_months Values**
   - Update job's `source_state` to set `max_age_months: 6`
   - Restart download
   - Verify only 6-month-old emails downloaded

### Error Scenario Testing

1. **IMAP Connection Failure**
   - Use invalid credentials
   - Expected: Job status = 'failed', error_message populated

2. **Date Parsing**
   - Ensure emails with invalid dates don't crash parser
   - Expected: Graceful fallback to INTERNALDATE

3. **Large Emails**
   - Download email > 10MB
   - Expected: Successful storage, correct size_bytes

## Success Criteria

### Must Have
1. ✅ `emails` table created with all columns
2. ✅ `email_attachments` table created
3. ✅ Real IMAP client using `imap` crate
4. ✅ Date filtering working (SINCE query)
5. ✅ Emails actually stored in database
6. ✅ download_items properly linked
7. ✅ OAuth2 and password auth both work
8. ✅ TypeScript types generated
9. ✅ `max_age_months` configurable (default: 12)

### Should Have
1. Attachment extraction and storage
2. Email threading (thread_id)
3. Label/tag support
4. Full-text search on email content

### Quality Checklist
- [ ] No emails older than max_age_months downloaded
- [ ] All email fields properly parsed and stored
- [ ] download_items and emails properly linked via FK
- [ ] OAuth2 token refresh works
- [ ] Error handling for malformed emails
- [ ] Database indexes improve query performance
- [ ] TypeScript types match Rust types exactly
- [ ] No credentials in logs

## Future Enhancements

1. **Attachment Storage**
   - Save attachments to filesystem
   - Calculate checksums for deduplication
   - Integrate with extractors crate for content extraction

2. **Email Threading**
   - Implement conversation threading
   - Group by In-Reply-To and References headers

3. **Full-Text Search**
   - Index email content for search
   - Support advanced queries

4. **Incremental Sync**
   - Only download new emails after initial sync
   - Use last_synced_uid more efficiently

## Configuration Reference

**Default Configuration** (in ImapDownloadState):
```json
{
  "folders": [...],
  "sync_strategy": "inbox-only",
  "last_highest_uid": {},
  "fetch_batch_size": 10,
  "max_age_months": 12
}
```

**To Change Date Filter** (update job's source_state):
```json
{
  "max_age_months": 6  // Download only last 6 months
}
```

**To Remove Date Filter** (download all emails):
```json
{
  "max_age_months": null
}
```

---

**Document Version**: 1.0
**Created**: 2026-01-30
**Status**: Ready for Implementation
**Estimated Effort**: 2-3 days
**Priority**: High (blocks email ingestion)
