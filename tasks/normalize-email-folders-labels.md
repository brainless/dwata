# Task: Normalize Email Folders and Labels Schema

## Objective

Refactor email folder and label storage from VARCHAR strings to normalized relational tables with proper foreign keys. This enables efficient folder/label listing APIs, browsing emails by folder/label, storing metadata (counts, sync state), and proper many-to-many relationships for Gmail labels.

## Background

### Current State
- **`emails.folder`**: VARCHAR NOT NULL - stores folder name as string
- **`emails.labels`**: VARCHAR - stores JSON array of label strings
- **`download_items.source_folder`**: VARCHAR - stores folder name as string
- **No folder/label listing capability** - must scan all emails to find folders
- **No folder metadata** - cannot store unread counts, IMAP sync state, colors, etc.
- **Poor query performance** - string comparisons instead of integer FK lookups
- **No consistency enforcement** - can have "INBOX" vs "Inbox" typos
- **No hierarchy support** - cannot model nested folders

### Why This Matters
1. **API Requirements**: Need to expose folder/label lists via API endpoints
2. **Browse by Folder/Label**: Users want to filter emails by folder or label
3. **IMAP Sync Efficiency**: Need to track per-folder sync state (UIDVALIDITY, last synced UID)
4. **Gmail Label Support**: Emails can have multiple labels (many-to-many relationship)
5. **Metadata & Statistics**: Store unread counts, message counts, colors per folder/label
6. **Performance**: Integer FK lookups are significantly faster than string comparisons

### IMAP vs Gmail Concepts
- **Traditional IMAP**: Uses folders (mailboxes). Email exists in ONE folder. Hierarchical structure.
- **Gmail**: Uses labels. Email can have MULTIPLE labels simultaneously.
- **Gmail's IMAP Implementation**: Maps labels to IMAP folders. Same email appears in multiple folders if it has multiple labels. Provides `X-GM-LABELS` extension for native label access.

## Architecture

### Database Schema Changes

```
┌─────────────────────────────────────────────────────────────┐
│                    email_folders                             │
│  - Stores IMAP folders per credential                        │
│  - Tracks IMAP sync state (UIDVALIDITY, last synced UID)    │
│  - Caches statistics (total, unread counts)                  │
│  - Supports hierarchy (parent_folder_id)                     │
└─────────────────────────────────────────────────────────────┘
                          ↓ (1:N)
┌─────────────────────────────────────────────────────────────┐
│                        emails                                │
│  - folder_id INTEGER FK (was: folder VARCHAR)                │
│  - Remove labels VARCHAR                                     │
└─────────────────────────────────────────────────────────────┘
                          ↓ (N:M)
┌─────────────────────────────────────────────────────────────┐
│              email_label_associations                        │
│  - email_id → emails.id                                      │
│  - label_id → email_labels.id                                │
└─────────────────────────────────────────────────────────────┘
                          ↓ (N:1)
┌─────────────────────────────────────────────────────────────┐
│                    email_labels                              │
│  - Stores labels per credential (Gmail-specific)             │
│  - Caches statistics (message counts)                        │
│  - Stores metadata (color, type)                             │
└─────────────────────────────────────────────────────────────┘
```

### New Tables

#### 1. `email_folders`
```sql
CREATE TABLE email_folders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    credential_id INTEGER NOT NULL,

    -- Identity
    name VARCHAR NOT NULL,              -- e.g., "INBOX", "Sent"
    display_name VARCHAR,                -- User-friendly name
    imap_path VARCHAR NOT NULL,          -- Full IMAP path: "[Gmail]/All Mail"
    folder_type VARCHAR,                 -- inbox, sent, drafts, trash, spam, archive, custom

    -- Hierarchy
    parent_folder_id INTEGER,            -- For nested folders

    -- IMAP sync state
    uidvalidity INTEGER,                 -- IMAP UIDVALIDITY (changes if folder recreated)
    last_synced_uid INTEGER,             -- Highest UID we've synced

    -- Cached statistics
    total_messages INTEGER DEFAULT 0,
    unread_messages INTEGER DEFAULT 0,

    -- Metadata
    is_subscribed BOOLEAN DEFAULT true,
    is_selectable BOOLEAN DEFAULT true,  -- Some folders can't contain messages

    -- Timestamps
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    last_synced_at BIGINT,

    UNIQUE(credential_id, imap_path),
    FOREIGN KEY(credential_id) REFERENCES credentials_metadata(id) ON DELETE CASCADE,
    FOREIGN KEY(parent_folder_id) REFERENCES email_folders(id) ON DELETE SET NULL
);

CREATE INDEX idx_email_folders_credential ON email_folders(credential_id);
CREATE INDEX idx_email_folders_type ON email_folders(credential_id, folder_type);
CREATE INDEX idx_email_folders_parent ON email_folders(parent_folder_id);
```

#### 2. `email_labels`
```sql
CREATE TABLE email_labels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    credential_id INTEGER NOT NULL,

    -- Identity
    name VARCHAR NOT NULL,               -- e.g., "Work", "Important"
    display_name VARCHAR,

    -- Gmail-specific
    label_type VARCHAR NOT NULL,         -- 'system' or 'user'
    color VARCHAR,                       -- Hex color code

    -- Statistics
    message_count INTEGER DEFAULT 0,

    -- Timestamps
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,

    UNIQUE(credential_id, name),
    FOREIGN KEY(credential_id) REFERENCES credentials_metadata(id) ON DELETE CASCADE
);

CREATE INDEX idx_email_labels_credential ON email_labels(credential_id);
CREATE INDEX idx_email_labels_type ON email_labels(credential_id, label_type);
```

#### 3. `email_label_associations`
```sql
CREATE TABLE email_label_associations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email_id INTEGER NOT NULL,
    label_id INTEGER NOT NULL,
    created_at BIGINT NOT NULL,

    UNIQUE(email_id, label_id),
    FOREIGN KEY(email_id) REFERENCES emails(id) ON DELETE CASCADE,
    FOREIGN KEY(label_id) REFERENCES email_labels(id) ON DELETE CASCADE
);

CREATE INDEX idx_email_label_assoc_email ON email_label_associations(email_id);
CREATE INDEX idx_email_label_assoc_label ON email_label_associations(label_id);
```

### Updated Tables

#### `emails` table changes
```sql
-- Before:
folder VARCHAR NOT NULL,
labels VARCHAR,

-- After:
folder_id INTEGER NOT NULL,
FOREIGN KEY(folder_id) REFERENCES email_folders(id)

-- Remove labels VARCHAR column
-- Labels now via email_label_associations join table
```

#### `download_items` table changes
```sql
-- Before:
source_folder VARCHAR,

-- After:
source_folder_id INTEGER,
FOREIGN KEY(source_folder_id) REFERENCES email_folders(id)
```

#### Index updates
```sql
-- Remove old index:
DROP INDEX IF EXISTS idx_emails_folder_date;

-- Add new index:
CREATE INDEX idx_emails_folder_date ON emails(folder_id, date_received DESC);
```

## Implementation Plan

### Phase 1: Database Schema Migration

#### Step 1.1: Add New Tables
- [ ] Add `email_folders` table definition to `dwata-api/src/database/migrations.rs`
- [ ] Add `email_labels` table definition
- [ ] Add `email_label_associations` table definition
- [ ] Add all indexes

#### Step 1.2: Migrate Existing Data
Create data migration function in `migrations.rs`:

```rust
// Pseudocode for migration logic
fn migrate_folders_and_labels(conn: &Connection) -> Result<()> {
    // 1. Extract unique folders from emails table
    // SELECT DISTINCT folder, credential_id FROM emails
    // INSERT INTO email_folders (credential_id, name, imap_path, ...)

    // 2. Extract unique labels from emails table
    // Parse JSON from labels VARCHAR column
    // INSERT INTO email_labels (credential_id, name, ...)

    // 3. Update emails.folder → emails.folder_id
    // ALTER TABLE emails ADD COLUMN folder_id INTEGER
    // UPDATE emails SET folder_id = (SELECT id FROM email_folders WHERE ...)

    // 4. Create label associations
    // Parse labels JSON for each email
    // INSERT INTO email_label_associations (email_id, label_id, ...)

    // 5. Drop old columns
    // ALTER TABLE emails DROP COLUMN folder
    // ALTER TABLE emails DROP COLUMN labels

    // 6. Update download_items.source_folder → source_folder_id
    // Similar process

    // 7. Update indexes
}
```

**Migration Strategy Notes:**
- Run migration in a transaction
- Create new columns before dropping old ones (safety)
- Validate data integrity after migration
- Log migration progress and any issues

#### Step 1.3: Add Database Query Functions
Create new files:
- [ ] `dwata-api/src/database/folders.rs` - Folder CRUD operations
- [ ] `dwata-api/src/database/labels.rs` - Label CRUD operations

Functions needed:
```rust
// folders.rs
pub async fn list_folders(conn: AsyncDbConnection, credential_id: i64) -> Result<Vec<EmailFolder>>
pub async fn get_folder(conn: AsyncDbConnection, folder_id: i64) -> Result<EmailFolder>
pub async fn upsert_folder(conn: AsyncDbConnection, credential_id: i64, imap_path: &str, ...) -> Result<i64>
pub async fn update_folder_stats(conn: AsyncDbConnection, folder_id: i64, total: i32, unread: i32) -> Result<()>
pub async fn update_folder_sync_state(conn: AsyncDbConnection, folder_id: i64, uidvalidity: u32, last_uid: u32) -> Result<()>

// labels.rs
pub async fn list_labels(conn: AsyncDbConnection, credential_id: i64) -> Result<Vec<EmailLabel>>
pub async fn get_label(conn: AsyncDbConnection, label_id: i64) -> Result<EmailLabel>
pub async fn upsert_label(conn: AsyncDbConnection, credential_id: i64, name: &str, ...) -> Result<i64>
pub async fn add_label_to_email(conn: AsyncDbConnection, email_id: i64, label_id: i64) -> Result<()>
pub async fn get_labels_for_email(conn: AsyncDbConnection, email_id: i64) -> Result<Vec<EmailLabel>>
pub async fn get_emails_for_label(conn: AsyncDbConnection, label_id: i64, limit: usize, offset: usize) -> Result<Vec<i64>>
```

#### Step 1.4: Update Existing Database Functions
- [ ] Update `dwata-api/src/database/emails.rs`
  - Change `insert_email()` to take `folder_id: i64` instead of `folder: &str`
  - Remove `labels: &[String]` parameter
  - Update `list_emails()` to filter by `folder_id` instead of `folder` string
  - Add `list_emails_by_label()` function

### Phase 2: Update Shared Types

#### Step 2.1: Add New Types
Edit `shared-types/src/`:

```rust
// folder.rs (NEW FILE)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EmailFolder {
    pub id: i64,
    pub credential_id: i64,
    pub name: String,
    pub display_name: Option<String>,
    pub imap_path: String,
    pub folder_type: Option<String>,
    pub parent_folder_id: Option<i64>,
    pub uidvalidity: Option<u32>,
    pub last_synced_uid: Option<u32>,
    pub total_messages: i32,
    pub unread_messages: i32,
    pub is_subscribed: bool,
    pub is_selectable: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_synced_at: Option<i64>,
}

// label.rs (NEW FILE)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EmailLabel {
    pub id: i64,
    pub credential_id: i64,
    pub name: String,
    pub display_name: Option<String>,
    pub label_type: String, // "system" | "user"
    pub color: Option<String>,
    pub message_count: i32,
    pub created_at: i64,
    pub updated_at: i64,
}
```

#### Step 2.2: Update Email Type
```rust
// email.rs
pub struct Email {
    // ... existing fields ...

    // Change:
    // pub folder: String,
    // pub labels: Vec<String>,

    // To:
    pub folder_id: i64,
    // labels removed from here - accessed via join
}
```

#### Step 2.3: Generate TypeScript Types
```bash
cd shared-types
cargo run --bin generate_api_types
```

### Phase 3: Update IMAP Downloaders

#### Step 3.1: Update IMAP Client to Discover Folders
Edit `dwata-api/src/integrations/imap_client.rs` or wherever IMAP logic lives:

```rust
pub async fn list_folders(&mut self) -> Result<Vec<FolderInfo>> {
    // Use IMAP LIST command to discover folders
    // For Gmail: Also use X-GM-LABELS extension if available
    // Return folder metadata: name, path, selectable, etc.
}
```

#### Step 3.2: Update Download Manager
Edit `dwata-api/src/jobs/download_manager.rs`:

**Before downloading emails:**
1. Discover folders using `list_folders()`
2. Upsert folders into `email_folders` table
3. Store returned folder IDs

**When downloading emails:**
1. Look up folder ID from `email_folders` table
2. Pass `folder_id` to `insert_email()` instead of folder string

**For Gmail with X-GM-LABELS:**
1. Discover labels using IMAP extension
2. Upsert labels into `email_labels` table
3. When parsing email, extract labels
4. Insert associations in `email_label_associations`

#### Step 3.3: Update OAuth IMAP Client
If there's Gmail-specific IMAP code:
- [ ] Add label discovery support
- [ ] Add label fetching for emails (X-GM-LABELS)
- [ ] Store label associations during email download

### Phase 4: Create API Handlers

#### Step 4.1: Folder Endpoints
Create `dwata-api/src/handlers/folders.rs`:

```rust
// GET /api/credentials/{credential_id}/folders
// Returns: Vec<EmailFolder>
pub async fn list_folders_handler(
    credential_id: web::Path<i64>,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse> {
    let folders = database::folders::list_folders(
        db.async_connection.clone(),
        *credential_id
    ).await?;
    Ok(HttpResponse::Ok().json(folders))
}

// GET /api/folders/{folder_id}
// Returns: EmailFolder
pub async fn get_folder_handler(
    folder_id: web::Path<i64>,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse> {
    let folder = database::folders::get_folder(
        db.async_connection.clone(),
        *folder_id
    ).await?;
    Ok(HttpResponse::Ok().json(folder))
}

// GET /api/folders/{folder_id}/emails?limit=50&offset=0
// Returns: Vec<Email>
pub async fn list_folder_emails_handler(
    folder_id: web::Path<i64>,
    query: web::Query<PaginationParams>,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse> {
    let emails = database::emails::list_emails(
        db.async_connection.clone(),
        None, // credential_id
        Some(*folder_id), // folder_id
        query.limit.unwrap_or(50),
        query.offset.unwrap_or(0),
    ).await?;
    Ok(HttpResponse::Ok().json(emails))
}
```

#### Step 4.2: Label Endpoints
Create `dwata-api/src/handlers/labels.rs`:

```rust
// GET /api/credentials/{credential_id}/labels
// Returns: Vec<EmailLabel>
pub async fn list_labels_handler(
    credential_id: web::Path<i64>,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse> {
    let labels = database::labels::list_labels(
        db.async_connection.clone(),
        *credential_id
    ).await?;
    Ok(HttpResponse::Ok().json(labels))
}

// GET /api/labels/{label_id}
// Returns: EmailLabel
pub async fn get_label_handler(
    label_id: web::Path<i64>,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse> {
    let label = database::labels::get_label(
        db.async_connection.clone(),
        *label_id
    ).await?;
    Ok(HttpResponse::Ok().json(label))
}

// GET /api/labels/{label_id}/emails?limit=50&offset=0
// Returns: Vec<Email>
pub async fn list_label_emails_handler(
    label_id: web::Path<i64>,
    query: web::Query<PaginationParams>,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse> {
    let email_ids = database::labels::get_emails_for_label(
        db.async_connection.clone(),
        *label_id,
        query.limit.unwrap_or(50),
        query.offset.unwrap_or(0),
    ).await?;

    // Fetch full email details
    let mut emails = Vec::new();
    for email_id in email_ids {
        let email = database::emails::get_email(
            db.async_connection.clone(),
            email_id
        ).await?;
        emails.push(email);
    }

    Ok(HttpResponse::Ok().json(emails))
}
```

#### Step 4.3: Update Existing Email List Handler
Edit `dwata-api/src/handlers/emails.rs`:

```rust
// GET /api/emails?credential_id=1&folder_id=5&limit=50&offset=0
pub async fn list_emails_handler(
    query: web::Query<EmailListParams>,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse> {
    let emails = database::emails::list_emails(
        db.async_connection.clone(),
        query.credential_id,
        query.folder_id, // NEW: support folder_id filter
        query.limit.unwrap_or(50),
        query.offset.unwrap_or(0),
    ).await?;
    Ok(HttpResponse::Ok().json(emails))
}

#[derive(Deserialize)]
pub struct EmailListParams {
    pub credential_id: Option<i64>,
    pub folder_id: Option<i64>, // NEW
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}
```

**Backward Compatibility:**
- When no `folder_id` is specified, list all emails (current behavior preserved)
- When `folder_id` is specified, filter by folder

#### Step 4.4: Register Routes
Edit `dwata-api/src/main.rs`:

```rust
// Add new routes
.route("/api/credentials/{credential_id}/folders", web::get().to(handlers::folders::list_folders_handler))
.route("/api/folders/{folder_id}", web::get().to(handlers::folders::get_folder_handler))
.route("/api/folders/{folder_id}/emails", web::get().to(handlers::folders::list_folder_emails_handler))
.route("/api/credentials/{credential_id}/labels", web::get().to(handlers::labels::list_labels_handler))
.route("/api/labels/{label_id}", web::get().to(handlers::labels::get_label_handler))
.route("/api/labels/{label_id}/emails", web::get().to(handlers::labels::list_label_emails_handler))
```

### Phase 5: Update GUI (Optional - for completion)

#### Step 5.1: Add Folder/Label List Components
Create `gui/src/components/FolderList.tsx`:
- Fetch folders from `/api/credentials/{id}/folders`
- Display hierarchical folder tree
- Show unread counts
- Click to browse emails in folder

Create `gui/src/components/LabelList.tsx`:
- Fetch labels from `/api/credentials/{id}/labels`
- Display label chips with colors
- Show message counts
- Click to browse emails with label

#### Step 5.2: Update Email List Component
Update `gui/src/pages/Emails.tsx`:
- Add folder/label filter UI
- Pass `folder_id` or `label_id` to email list API
- Show current folder/label context in header

### Phase 6: Testing & Validation

#### Step 6.1: Database Migration Testing
- [ ] Test migration on database with existing emails
- [ ] Verify all folders extracted correctly
- [ ] Verify all labels extracted correctly
- [ ] Verify email associations intact
- [ ] Check foreign key constraints work

#### Step 6.2: IMAP Download Testing
- [ ] Test folder discovery on password-based IMAP account
- [ ] Test folder discovery on Gmail OAuth account
- [ ] Test Gmail label extraction
- [ ] Test email download into new schema
- [ ] Verify folder_id is set correctly
- [ ] Verify label associations created

#### Step 6.3: API Testing
Test all new endpoints:
```bash
# List folders for credential
curl http://localhost:8080/api/credentials/1/folders

# Get folder details
curl http://localhost:8080/api/folders/1

# List emails in folder
curl http://localhost:8080/api/folders/1/emails?limit=10

# List labels for credential
curl http://localhost:8080/api/credentials/1/labels

# List emails with label
curl http://localhost:8080/api/labels/1/emails?limit=10

# List all emails (backward compatibility)
curl http://localhost:8080/api/emails?limit=10

# List emails filtered by folder
curl http://localhost:8080/api/emails?folder_id=1&limit=10
```

#### Step 6.4: Performance Testing
- [ ] Benchmark folder listing query time
- [ ] Benchmark email listing by folder (INT FK vs old VARCHAR)
- [ ] Check index usage with EXPLAIN QUERY PLAN
- [ ] Test pagination performance

## Success Criteria

### Functionality
- [x] New tables created with proper indexes
- [x] Existing data migrated without loss
- [x] IMAP downloader populates folder/label tables
- [x] API endpoints return folder/label lists
- [x] Can browse emails by folder_id
- [x] Can browse emails by label_id
- [x] Backward compatibility: listing all emails still works

### Data Integrity
- [x] All foreign keys enforced
- [x] No orphaned records
- [x] Cascade deletes work correctly
- [x] UNIQUE constraints prevent duplicates

### Performance
- [x] Folder listing is fast (< 100ms for 100 folders)
- [x] Email filtering by folder_id is faster than old string comparison
- [x] Indexes are being used (verify with EXPLAIN)

### API Contract
- [x] TypeScript types generated and match Rust types
- [x] All endpoints return proper HTTP status codes
- [x] Error handling for invalid folder/label IDs
- [x] Pagination works correctly

## Notes

### Design Decisions

**Why separate email_folders and email_labels tables?**
- Folders and labels have different semantics
- Folders are hierarchical (parent_folder_id), labels are flat
- Folders have IMAP sync state (UIDVALIDITY), labels don't
- Keeps concerns separated and schema cleaner

**Why use integer IDs everywhere?**
- Performance: Integer comparison/indexing is faster
- Consistency: Prevents typos and case sensitivity issues
- Foreign keys: Enforces referential integrity
- Flexibility: Can rename folders without updating all emails

**Why nullable download_item_id in emails but NOT nullable folder_id?**
- Every email MUST belong to a folder (IMAP requirement)
- download_item_id is optional tracking metadata

**Cascade Delete Strategy:**
- `credential_id` deletion → CASCADE delete folders/labels → CASCADE delete emails
  - Rationale: If credential is deleted, all its data should be removed
- `folder_id` deletion → Should this CASCADE delete emails?
  - Decision needed: Either CASCADE or prevent deletion of non-empty folders
  - Recommendation: Prevent deletion of non-empty folders (return error)

### Future Enhancements
- [ ] Add folder rename capability (updates imap_path)
- [ ] Add folder archiving (soft delete)
- [ ] Add custom folder ordering (sort_order column)
- [ ] Add folder color/icon customization
- [ ] Implement label autocomplete for quick filtering
- [ ] Add "All Mail" virtual folder aggregating all folders
- [ ] Add smart folders (e.g., "Unread", "Flagged", "Today")

## References

- IMAP RFC 3501: https://tools.ietf.org/html/rfc3501
- Gmail IMAP extensions: https://developers.google.com/gmail/imap/imap-extensions
- Current schema: `dwata-api/src/database/migrations.rs`
- Current email storage: `dwata-api/src/database/emails.rs`
- IMAP download manager: `dwata-api/src/jobs/download_manager.rs`
