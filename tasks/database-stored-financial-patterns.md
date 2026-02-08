# Task: Database-Stored Financial Patterns

**Status**: Completed
**Priority**: High
**Created**: 2026-02-02
**Depends On**: tasks/financial-email-text-extractor.md

## Objective

Refactor the financial pattern extraction system to store regex patterns in the database instead of hardcoding them in Rust. This enables runtime customization where users can add, modify, and disable patterns specific to their vendors and use cases.

## Background

Currently, financial patterns are hardcoded in `extractors/src/financial_patterns/mod.rs`. This works well for default patterns but:
- Requires recompilation to add new patterns
- Cannot be customized per-user without code changes
- Makes it difficult for users to handle vendor-specific formats
- No way to disable problematic patterns without code changes

**Solution**: Store patterns in the database with default patterns seeded via migrations.

## Benefits

1. **Runtime Customization** - Users can add patterns for their specific vendors through the API/GUI
2. **No Recompilation** - Pattern changes take effect immediately
3. **Pattern Management** - Enable/disable patterns without deletion
4. **Shareability** - Users could export/import pattern sets
5. **Testability** - Users can test patterns against their emails before activating
6. **Version Control** - Track pattern changes and performance over time

## Database Schema

### New Table: `financial_patterns`

```sql
CREATE TABLE IF NOT EXISTS financial_patterns (
    id INTEGER PRIMARY KEY DEFAULT nextval('seq_financial_patterns_id'),

    -- Pattern identity
    name VARCHAR NOT NULL,                     -- e.g., "payment_to_vendor"
    regex_pattern VARCHAR NOT NULL,            -- The actual regex string
    description VARCHAR,                       -- User-friendly description of what it matches

    -- Pattern metadata
    document_type VARCHAR NOT NULL,            -- invoice, bill, receipt, payment-confirmation
    status VARCHAR NOT NULL,                   -- paid, pending, overdue
    confidence FLOAT NOT NULL,                 -- 0.0 to 1.0 confidence score

    -- Capture group indices (which regex group contains each field)
    amount_group INTEGER NOT NULL,             -- Required: which group has the amount
    vendor_group INTEGER,                      -- Optional: which group has vendor name
    date_group INTEGER,                        -- Optional: which group has transaction date

    -- Management flags
    is_default BOOLEAN DEFAULT false,          -- True for system-provided patterns
    is_active BOOLEAN DEFAULT true,            -- Allow disable without deletion

    -- Usage statistics (optional, for future analytics)
    match_count INTEGER DEFAULT 0,             -- How many times this pattern matched
    last_matched_at BIGINT,                    -- When it last found a match

    -- Timestamps
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,

    -- Uniqueness constraints
    UNIQUE(name),                              -- Prevent duplicate pattern names
    UNIQUE(regex_pattern)                      -- Prevent duplicate regex patterns
);

CREATE INDEX IF NOT EXISTS idx_financial_patterns_active
    ON financial_patterns(is_active);

CREATE INDEX IF NOT EXISTS idx_financial_patterns_type
    ON financial_patterns(document_type);
```

### Migration: Seed Default Patterns

Add to `dwata-api/src/database/migrations.rs`:

```sql
-- Seed default financial patterns
INSERT INTO financial_patterns
    (name, regex_pattern, document_type, status, confidence,
     amount_group, vendor_group, date_group, description, is_default,
     match_count, created_at, updated_at)
VALUES
    -- Payment Confirmation Patterns (5)
    ('payment_to_vendor',
     '(?i)payment of \$?([\d,]+\.?\d{0,2}) to ([A-Za-z\s]+)',
     'payment-confirmation', 'paid', 0.90,
     1, 2, NULL,
     'Matches: "payment of $150.00 to Comcast"',
     true, 0, 0, 0),

    ('paid_amount_to_vendor',
     '(?i)paid \$?([\d,]+\.?\d{0,2}) to ([A-Za-z\s]+)',
     'payment-confirmation', 'paid', 0.88,
     1, 2, NULL,
     'Matches: "paid $99 to Adobe"',
     true, 0, 0, 0),

    ('your_payment_to_vendor',
     '(?i)your \$?([\d,]+\.?\d{0,2}) payment to ([A-Za-z\s]+)',
     'payment-confirmation', 'paid', 0.87,
     1, 2, NULL,
     'Matches: "Your $50.00 payment to Netflix"',
     true, 0, 0, 0),

    ('successfully_paid_to_vendor',
     '(?i)successfully paid \$?([\d,]+\.?\d{0,2}) to ([A-Za-z\s]+)',
     'payment-confirmation', 'paid', 0.92,
     1, 2, NULL,
     'Matches: "successfully paid $1,200.00 to Chase"',
     true, 0, 0, 0),

    ('payment_processed_to_vendor',
     '(?i)payment processed:? \$?([\d,]+\.?\d{0,2}) to ([A-Za-z\s]+)',
     'payment-confirmation', 'paid', 0.91,
     1, 2, NULL,
     'Matches: "payment processed: $45.99 to Spotify"',
     true, 0, 0, 0),

    -- Bill/Invoice Due Patterns (4)
    ('bill_due_explicit',
     '(?i)bill (?:of|for) \$?([\d,]+\.?\d{0,2}) (?:is )?due (?:on )?([A-Za-z]+ \d{1,2})',
     'bill', 'pending', 0.88,
     1, NULL, 2,
     'Matches: "bill of $99.99 is due on Feb 10"',
     true, 0, 0, 0),

    ('invoice_due_date',
     '(?i)invoice for \$?([\d,]+\.?\d{0,2}) due ([A-Za-z]+ \d{1,2})',
     'invoice', 'pending', 0.89,
     1, NULL, 2,
     'Matches: "invoice for $3,500 due January 25"',
     true, 0, 0, 0),

    ('vendor_bill_due',
     '(?i)(?:your )?([A-Za-z]+) bill (?:\(?\$?([\d,]+\.?\d{0,2})\)?) is due ([A-Za-z]+ \d{1,2})',
     'bill', 'pending', 0.87,
     2, 1, 3,
     'Matches: "Your Adobe bill ($99) is due Feb 10"',
     true, 0, 0, 0),

    ('due_amount_by_date',
     '(?i)due:? \$?([\d,]+\.?\d{0,2}) by (\d{2}/\d{2}/\d{4})',
     'bill', 'pending', 0.86,
     1, NULL, 2,
     'Matches: "due: $150.00 by 02/05/2026"',
     true, 0, 0, 0),

    -- Payment Received Patterns (3)
    ('received_payment_from',
     '(?i)received (?:a payment of )?\$?([\d,]+\.?\d{0,2}) from ([A-Za-z\s]+)',
     'invoice', 'paid', 0.92,
     1, 2, NULL,
     'Matches: "received $3,500.00 from Acme Corp"',
     true, 0, 0, 0),

    ('payment_of_from',
     '(?i)payment of \$?([\d,]+\.?\d{0,2}) from ([A-Za-z\s]+)',
     'invoice', 'paid', 0.90,
     1, 2, NULL,
     'Matches: "payment of $2,000 from TechStart Inc"',
     true, 0, 0, 0),

    ('you_received_payment',
     '(?i)you received (?:a payment:? )?\$?([\d,]+\.?\d{0,2}) from ([A-Za-z\s]+)',
     'invoice', 'paid', 0.91,
     1, 2, NULL,
     'Matches: "You received a payment: $1,500 from Client Name"',
     true, 0, 0, 0),

    -- Overdue/Late Patterns (3)
    ('payment_overdue',
     '(?i)payment of \$?([\d,]+\.?\d{0,2}) (?:is |to ([A-Za-z\s]+) )?(?:is )?overdue',
     'bill', 'overdue', 0.93,
     1, 2, NULL,
     'Matches: "payment of $1,200 is overdue" or "payment of $1,200 to Chase is overdue"',
     true, 0, 0, 0),

    ('amount_past_due',
     '(?i)\$?([\d,]+\.?\d{0,2}) payment past due',
     'bill', 'overdue', 0.91,
     1, NULL, NULL,
     'Matches: "$450.00 payment past due"',
     true, 0, 0, 0),

    ('overdue_bill_days',
     '(?i)overdue bill:? \$?([\d,]+\.?\d{0,2})(?: \((\d+) days? late\))?',
     'bill', 'overdue', 0.90,
     1, NULL, NULL,
     'Matches: "overdue bill: $99 (3 days late)"',
     true, 0, 0, 0)

ON CONFLICT (regex_pattern) DO NOTHING;
```

## API Endpoints

**Core Endpoints (Required):** 1-5
**Optional Endpoints (Future Enhancement):** 6-9

### 1. List Patterns

**GET** `/api/financial/patterns`

**Query Parameters:**
- `active_only` (boolean, default: true) - Only return active patterns
- `is_default` (boolean, optional) - Filter by default vs custom patterns
- `document_type` (string, optional) - Filter by document type

**Response:**
```json
{
  "patterns": [
    {
      "id": 1,
      "name": "payment_to_vendor",
      "regex_pattern": "(?i)payment of \\$?([\\d,]+\\.?\\d{0,2}) to ([A-Za-z\\s]+)",
      "description": "Matches: \"payment of $150.00 to Comcast\"",
      "document_type": "payment-confirmation",
      "status": "paid",
      "confidence": 0.90,
      "amount_group": 1,
      "vendor_group": 2,
      "date_group": null,
      "is_default": true,
      "is_active": true,
      "match_count": 45,
      "last_matched_at": 1738540800,
      "created_at": 1738454400,
      "updated_at": 1738454400
    }
  ],
  "total": 15
}
```

### 2. Get Single Pattern

**GET** `/api/financial/patterns/:id`

**Response:**
```json
{
  "pattern": { /* same as list item */ }
}
```

### 3. Create Pattern

**POST** `/api/financial/patterns`

**Request Body:**
```json
{
  "name": "venmo_payment_received",
  "regex_pattern": "(?i)you received \\$([\\d,]+\\.\\d{2}) from (@[A-Za-z0-9_]+)",
  "description": "Matches Venmo payment notifications",
  "document_type": "payment-confirmation",
  "status": "paid",
  "confidence": 0.85,
  "amount_group": 1,
  "vendor_group": 2,
  "date_group": null,
  "is_active": true
}
```

**Validation:**
- `name` must be unique and match `^[a-z0-9_]+$`
- `regex_pattern` must compile successfully
- `regex_pattern` must be unique
- Capture groups must exist in the regex
- `confidence` must be between 0.0 and 1.0
- `document_type` must be valid enum value
- `status` must be valid enum value
- Pattern must not cause catastrophic backtracking (timeout test)

**Response:**
```json
{
  "pattern": { /* created pattern */ },
  "message": "Pattern created successfully"
}
```

**Error Response (400):**
```json
{
  "error": "Invalid regex pattern",
  "details": "regex parse error: unclosed group"
}
```

### 4. Update Pattern

**PUT** `/api/financial/patterns/:id`

**Request Body:** (all fields optional)
```json
{
  "name": "venmo_payment_received_updated",
  "regex_pattern": "(?i)you received \\$([\\d,]+\\.\\d{2}) from (@[A-Za-z0-9_]+)",
  "description": "Updated description",
  "confidence": 0.90,
  "is_active": true
}
```

**Validation:**
- Cannot update `is_default` flag (system-managed)
- Same validation as create for updated fields
- Cannot modify default patterns' core fields (only allow toggling is_active)

**Response:**
```json
{
  "pattern": { /* updated pattern */ },
  "message": "Pattern updated successfully"
}
```

### 5. Toggle Pattern Active Status

**PATCH** `/api/financial/patterns/:id/toggle`

**Request Body:**
```json
{
  "is_active": false
}
```

**Response:**
```json
{
  "pattern": { /* updated pattern */ },
  "message": "Pattern disabled successfully"
}
```

### 6. Delete Pattern (OPTIONAL - Future Enhancement)

**DELETE** `/api/financial/patterns/:id`

**Restrictions:**
- Cannot delete default patterns (`is_default: true`)
- Only custom patterns can be deleted

**Response:**
```json
{
  "message": "Pattern deleted successfully"
}
```

**Error Response (403):**
```json
{
  "error": "Cannot delete default pattern",
  "details": "Default patterns can only be disabled, not deleted"
}
```

### 7. Test Pattern (OPTIONAL - Future Enhancement)

**POST** `/api/financial/patterns/test`

**Request Body:**
```json
{
  "regex_pattern": "(?i)payment of \\$?([\\d,]+\\.?\\d{0,2}) to ([A-Za-z\\s]+)",
  "test_text": "Your payment of $150.00 to Comcast was successful",
  "amount_group": 1,
  "vendor_group": 2,
  "date_group": null
}
```

**Response:**
```json
{
  "matches": true,
  "extracted": {
    "amount": 150.00,
    "vendor": "Comcast",
    "date": null
  },
  "full_match": "payment of $150.00 to Comcast",
  "is_valid": true,
  "execution_time_ms": 0.5
}
```

**Use Case:** Test a pattern before creating it to ensure it works correctly.

### 8. Bulk Import Patterns (OPTIONAL - Future Enhancement)

**POST** `/api/financial/patterns/import`

**Request Body:**
```json
{
  "patterns": [
    { /* pattern object */ },
    { /* pattern object */ }
  ],
  "skip_duplicates": true
}
```

**Response:**
```json
{
  "imported": 5,
  "skipped": 2,
  "errors": []
}
```

### 9. Export Patterns (OPTIONAL - Future Enhancement)

**GET** `/api/financial/patterns/export`

**Query Parameters:**
- `include_defaults` (boolean, default: false) - Include default patterns
- `active_only` (boolean, default: true)

**Response:**
```json
{
  "patterns": [ /* array of patterns */ ],
  "exported_at": 1738540800,
  "version": "1.0"
}
```

## Implementation Phases

### Phase 1: Database Schema & Migration

**Files to modify:**
- `dwata-api/src/database/migrations.rs` - Add table creation and seed data
- `shared-types/src/lib.rs` - Add `FinancialPattern` type for API

**Tasks:**
1. Create `financial_patterns` table schema
2. Add default pattern seed data (15 patterns from original task)
3. Test migration runs successfully
4. Verify patterns are seeded correctly

### Phase 2: Database Operations Module

**New file:** `dwata-api/src/database/financial_patterns.rs`

**Functions to implement:**
```rust
// Read operations
pub async fn list_patterns(
    conn: AsyncDbConnection,
    active_only: bool,
    is_default: Option<bool>,
    document_type: Option<String>,
) -> Result<Vec<FinancialPattern>>;

pub async fn get_pattern(
    conn: AsyncDbConnection,
    id: i64,
) -> Result<FinancialPattern>;

pub async fn list_active_patterns(
    conn: AsyncDbConnection,
) -> Result<Vec<FinancialPattern>>;

// Write operations
pub async fn insert_pattern(
    conn: AsyncDbConnection,
    pattern: &FinancialPattern,
) -> Result<i64>;

pub async fn update_pattern(
    conn: AsyncDbConnection,
    id: i64,
    pattern: &FinancialPattern,
) -> Result<()>;

pub async fn toggle_pattern_active(
    conn: AsyncDbConnection,
    id: i64,
    is_active: bool,
) -> Result<()>;

// OPTIONAL: Delete pattern (can be implemented later)
// pub async fn delete_pattern(
//     conn: AsyncDbConnection,
//     id: i64,
// ) -> Result<()>;

// Validation
pub async fn pattern_name_exists(
    conn: AsyncDbConnection,
    name: &str,
    exclude_id: Option<i64>,
) -> Result<bool>;

pub async fn pattern_regex_exists(
    conn: AsyncDbConnection,
    regex: &str,
    exclude_id: Option<i64>,
) -> Result<bool>;

// Statistics
pub async fn increment_match_count(
    conn: AsyncDbConnection,
    id: i64,
) -> Result<()>;

pub async fn update_last_matched(
    conn: AsyncDbConnection,
    id: i64,
    timestamp: i64,
) -> Result<()>;
```

**Export in** `dwata-api/src/database/mod.rs`

### Phase 3: Pattern Validation Module

**New file:** `dwata-api/src/helpers/pattern_validator.rs`

**Functions to implement:**
```rust
use regex::Regex;
use anyhow::{Result, bail};
use std::time::Duration;

pub struct PatternValidationError {
    pub field: String,
    pub message: String,
}

/// Validates a pattern before saving to database
pub fn validate_pattern(
    name: &str,
    regex_pattern: &str,
    amount_group: usize,
    vendor_group: Option<usize>,
    date_group: Option<usize>,
    confidence: f32,
    document_type: &str,
    status: &str,
) -> Result<()> {
    // Validate name format
    validate_pattern_name(name)?;

    // Validate regex compiles
    let regex = validate_regex_compiles(regex_pattern)?;

    // Validate capture groups exist
    validate_capture_groups(&regex, amount_group, vendor_group, date_group)?;

    // Validate confidence range
    validate_confidence(confidence)?;

    // Validate enum values
    validate_document_type(document_type)?;
    validate_status(status)?;

    // Test for catastrophic backtracking
    validate_regex_performance(&regex)?;

    Ok(())
}

fn validate_pattern_name(name: &str) -> Result<()> {
    if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
        bail!("Pattern name must contain only lowercase letters, numbers, and underscores");
    }
    if name.is_empty() || name.len() > 100 {
        bail!("Pattern name must be 1-100 characters");
    }
    Ok(())
}

fn validate_regex_compiles(pattern: &str) -> Result<Regex> {
    Regex::new(pattern).map_err(|e| {
        anyhow::anyhow!("Invalid regex pattern: {}", e)
    })
}

fn validate_capture_groups(
    regex: &Regex,
    amount_group: usize,
    vendor_group: Option<usize>,
    date_group: Option<usize>,
) -> Result<()> {
    let group_count = regex.captures_len();

    if amount_group >= group_count {
        bail!("amount_group {} does not exist (regex has {} groups)",
              amount_group, group_count);
    }

    if let Some(vg) = vendor_group {
        if vg >= group_count {
            bail!("vendor_group {} does not exist (regex has {} groups)",
                  vg, group_count);
        }
    }

    if let Some(dg) = date_group {
        if dg >= group_count {
            bail!("date_group {} does not exist (regex has {} groups)",
                  dg, group_count);
        }
    }

    Ok(())
}

fn validate_confidence(confidence: f32) -> Result<()> {
    if !(0.0..=1.0).contains(&confidence) {
        bail!("Confidence must be between 0.0 and 1.0");
    }
    Ok(())
}

fn validate_document_type(doc_type: &str) -> Result<()> {
    let valid_types = ["invoice", "bill", "receipt", "payment-confirmation", "other"];
    if !valid_types.contains(&doc_type) {
        bail!("Invalid document_type. Must be one of: {:?}", valid_types);
    }
    Ok(())
}

fn validate_status(status: &str) -> Result<()> {
    let valid_statuses = ["paid", "pending", "overdue", "cancelled"];
    if !valid_statuses.contains(&status) {
        bail!("Invalid status. Must be one of: {:?}", valid_statuses);
    }
    Ok(())
}

fn validate_regex_performance(regex: &Regex) -> Result<()> {
    // Test with pathological input to detect catastrophic backtracking
    let test_input = "a".repeat(1000);

    let start = std::time::Instant::now();
    let _ = regex.is_match(&test_input);
    let duration = start.elapsed();

    if duration > Duration::from_millis(100) {
        bail!("Regex pattern is too slow (potential catastrophic backtracking)");
    }

    Ok(())
}

/// OPTIONAL: Test a pattern against sample text (can be implemented later)
/// This is used by the pattern testing endpoint
pub fn test_pattern(
    regex_pattern: &str,
    test_text: &str,
    amount_group: usize,
    vendor_group: Option<usize>,
    date_group: Option<usize>,
) -> Result<TestPatternResult> {
    let regex = Regex::new(regex_pattern)?;

    let start = std::time::Instant::now();
    let captures = regex.captures(test_text);
    let execution_time = start.elapsed();

    if let Some(caps) = captures {
        let amount = caps.get(amount_group)
            .map(|m| m.as_str().to_string());
        let vendor = vendor_group
            .and_then(|g| caps.get(g))
            .map(|m| m.as_str().to_string());
        let date = date_group
            .and_then(|g| caps.get(g))
            .map(|m| m.as_str().to_string());

        Ok(TestPatternResult {
            matches: true,
            full_match: caps.get(0).map(|m| m.as_str().to_string()),
            extracted: ExtractedFields { amount, vendor, date },
            is_valid: true,
            execution_time_ms: execution_time.as_secs_f64() * 1000.0,
        })
    } else {
        Ok(TestPatternResult {
            matches: false,
            full_match: None,
            extracted: ExtractedFields {
                amount: None,
                vendor: None,
                date: None
            },
            is_valid: true,
            execution_time_ms: execution_time.as_secs_f64() * 1000.0,
        })
    }
}

pub struct TestPatternResult {
    pub matches: bool,
    pub full_match: Option<String>,
    pub extracted: ExtractedFields,
    pub is_valid: bool,
    pub execution_time_ms: f64,
}

pub struct ExtractedFields {
    pub amount: Option<String>,
    pub vendor: Option<String>,
    pub date: Option<String>,
}
```

### Phase 4: API Handlers

**File:** `dwata-api/src/handlers/financial.rs` (add to existing file)

**Note:** Only implement handlers for endpoints 1-5 (core functionality). Handlers for endpoints 6-9 (delete_pattern, test_pattern, import/export) can be skipped for now as they're marked optional.

**Add handler functions:**
```rust
use crate::helpers::pattern_validator;
use crate::database::financial_patterns as patterns_db;

// GET /api/financial/patterns
pub async fn list_patterns(
    db_conn: web::Data<AsyncDbConnection>,
    query: web::Query<ListPatternsQuery>,
) -> ActixResult<HttpResponse> {
    let patterns = patterns_db::list_patterns(
        db_conn.as_ref().clone(),
        query.active_only.unwrap_or(true),
        query.is_default,
        query.document_type.clone(),
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "patterns": patterns,
        "total": patterns.len()
    })))
}

// GET /api/financial/patterns/:id
pub async fn get_pattern(
    db_conn: web::Data<AsyncDbConnection>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let pattern = patterns_db::get_pattern(db_conn.as_ref().clone(), *path)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "pattern": pattern
    })))
}

// POST /api/financial/patterns
pub async fn create_pattern(
    db_conn: web::Data<AsyncDbConnection>,
    request: web::Json<CreatePatternRequest>,
) -> ActixResult<HttpResponse> {
    // Validate pattern
    pattern_validator::validate_pattern(
        &request.name,
        &request.regex_pattern,
        request.amount_group,
        request.vendor_group,
        request.date_group,
        request.confidence,
        &request.document_type,
        &request.status,
    )
    .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    // Check for duplicates
    let name_exists = patterns_db::pattern_name_exists(
        db_conn.as_ref().clone(),
        &request.name,
        None,
    )
    .await
    .unwrap_or(false);

    if name_exists {
        return Err(actix_web::error::ErrorBadRequest("Pattern name already exists"));
    }

    let regex_exists = patterns_db::pattern_regex_exists(
        db_conn.as_ref().clone(),
        &request.regex_pattern,
        None,
    )
    .await
    .unwrap_or(false);

    if regex_exists {
        return Err(actix_web::error::ErrorBadRequest(
            "Pattern with this regex already exists"
        ));
    }

    // Create pattern
    let pattern = FinancialPattern {
        id: 0, // Will be set by DB
        name: request.name.clone(),
        regex_pattern: request.regex_pattern.clone(),
        description: request.description.clone(),
        document_type: request.document_type.clone(),
        status: request.status.clone(),
        confidence: request.confidence,
        amount_group: request.amount_group,
        vendor_group: request.vendor_group,
        date_group: request.date_group,
        is_default: false,
        is_active: request.is_active.unwrap_or(true),
        match_count: 0,
        last_matched_at: None,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };

    let id = patterns_db::insert_pattern(db_conn.as_ref().clone(), &pattern)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let created_pattern = patterns_db::get_pattern(db_conn.as_ref().clone(), id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Created().json(serde_json::json!({
        "pattern": created_pattern,
        "message": "Pattern created successfully"
    })))
}

// PUT /api/financial/patterns/:id
pub async fn update_pattern(
    db_conn: web::Data<AsyncDbConnection>,
    path: web::Path<i64>,
    request: web::Json<UpdatePatternRequest>,
) -> ActixResult<HttpResponse> {
    let pattern_id = *path;

    // Get existing pattern
    let existing = patterns_db::get_pattern(db_conn.as_ref().clone(), pattern_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    // Prevent modifying core fields of default patterns
    if existing.is_default {
        if request.name.is_some() || request.regex_pattern.is_some() {
            return Err(actix_web::error::ErrorForbidden(
                "Cannot modify name or regex of default patterns"
            ));
        }
    }

    // Build updated pattern
    let updated = FinancialPattern {
        id: pattern_id,
        name: request.name.clone().unwrap_or(existing.name),
        regex_pattern: request.regex_pattern.clone().unwrap_or(existing.regex_pattern),
        description: request.description.clone().or(existing.description),
        document_type: request.document_type.clone().unwrap_or(existing.document_type),
        status: request.status.clone().unwrap_or(existing.status),
        confidence: request.confidence.unwrap_or(existing.confidence),
        amount_group: request.amount_group.unwrap_or(existing.amount_group),
        vendor_group: request.vendor_group.or(existing.vendor_group),
        date_group: request.date_group.or(existing.date_group),
        is_default: existing.is_default,
        is_active: request.is_active.unwrap_or(existing.is_active),
        match_count: existing.match_count,
        last_matched_at: existing.last_matched_at,
        created_at: existing.created_at,
        updated_at: chrono::Utc::now().timestamp(),
    };

    // Validate updated pattern
    pattern_validator::validate_pattern(
        &updated.name,
        &updated.regex_pattern,
        updated.amount_group,
        updated.vendor_group,
        updated.date_group,
        updated.confidence,
        &updated.document_type,
        &updated.status,
    )
    .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    // Update in database
    patterns_db::update_pattern(db_conn.as_ref().clone(), pattern_id, &updated)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let updated_pattern = patterns_db::get_pattern(db_conn.as_ref().clone(), pattern_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "pattern": updated_pattern,
        "message": "Pattern updated successfully"
    })))
}

// PATCH /api/financial/patterns/:id/toggle
pub async fn toggle_pattern(
    db_conn: web::Data<AsyncDbConnection>,
    path: web::Path<i64>,
    request: web::Json<TogglePatternRequest>,
) -> ActixResult<HttpResponse> {
    let pattern_id = *path;

    patterns_db::toggle_pattern_active(
        db_conn.as_ref().clone(),
        pattern_id,
        request.is_active,
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let pattern = patterns_db::get_pattern(db_conn.as_ref().clone(), pattern_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let message = if request.is_active {
        "Pattern enabled successfully"
    } else {
        "Pattern disabled successfully"
    };

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "pattern": pattern,
        "message": message
    })))
}

// DELETE /api/financial/patterns/:id
pub async fn delete_pattern(
    db_conn: web::Data<AsyncDbConnection>,
    path: web::Path<i64>,
) -> ActixResult<HttpResponse> {
    let pattern_id = *path;

    // Get pattern to check if it's default
    let pattern = patterns_db::get_pattern(db_conn.as_ref().clone(), pattern_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    if pattern.is_default {
        return Err(actix_web::error::ErrorForbidden(
            "Cannot delete default pattern. Use toggle to disable instead."
        ));
    }

    patterns_db::delete_pattern(db_conn.as_ref().clone(), pattern_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Pattern deleted successfully"
    })))
}

// POST /api/financial/patterns/test
pub async fn test_pattern(
    request: web::Json<TestPatternRequest>,
) -> ActixResult<HttpResponse> {
    let result = pattern_validator::test_pattern(
        &request.regex_pattern,
        &request.test_text,
        request.amount_group,
        request.vendor_group,
        request.date_group,
    )
    .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    Ok(HttpResponse::Ok().json(result))
}

// Request/Response types
#[derive(Deserialize)]
pub struct ListPatternsQuery {
    active_only: Option<bool>,
    is_default: Option<bool>,
    document_type: Option<String>,
}

#[derive(Deserialize)]
pub struct CreatePatternRequest {
    name: String,
    regex_pattern: String,
    description: Option<String>,
    document_type: String,
    status: String,
    confidence: f32,
    amount_group: usize,
    vendor_group: Option<usize>,
    date_group: Option<usize>,
    is_active: Option<bool>,
}

#[derive(Deserialize)]
pub struct UpdatePatternRequest {
    name: Option<String>,
    regex_pattern: Option<String>,
    description: Option<String>,
    document_type: Option<String>,
    status: Option<String>,
    confidence: Option<f32>,
    amount_group: Option<usize>,
    vendor_group: Option<usize>,
    date_group: Option<usize>,
    is_active: Option<bool>,
}

#[derive(Deserialize)]
pub struct TogglePatternRequest {
    is_active: bool,
}

#[derive(Deserialize)]
pub struct TestPatternRequest {
    regex_pattern: String,
    test_text: String,
    amount_group: usize,
    vendor_group: Option<usize>,
    date_group: Option<usize>,
}
```

**Register routes in** `dwata-api/src/main.rs`:
```rust
// Financial pattern management (Core endpoints)
.route("/api/financial/patterns", web::get().to(financial::list_patterns))
.route("/api/financial/patterns", web::post().to(financial::create_pattern))
.route("/api/financial/patterns/{id}", web::get().to(financial::get_pattern))
.route("/api/financial/patterns/{id}", web::put().to(financial::update_pattern))
.route("/api/financial/patterns/{id}/toggle", web::patch().to(financial::toggle_pattern))

// Optional routes (implement later)
// .route("/api/financial/patterns/test", web::post().to(financial::test_pattern))
// .route("/api/financial/patterns/{id}", web::delete().to(financial::delete_pattern))
// .route("/api/financial/patterns/import", web::post().to(financial::import_patterns))
// .route("/api/financial/patterns/export", web::get().to(financial::export_patterns))
```

### Phase 5: Update Extractor to Load from Database

**File:** `extractors/src/financial_patterns/extractor.rs`

**Modify to load patterns from database:**
```rust
use crate::financial_patterns::FinancialPattern;
use shared_types::FinancialTransaction;
use regex::Regex;

pub struct FinancialPatternExtractor {
    patterns: Vec<CompiledPattern>,
}

struct CompiledPattern {
    id: i64,
    name: String,
    regex: Regex,
    document_type: String,
    status: String,
    confidence: f32,
    amount_group: usize,
    vendor_group: Option<usize>,
    date_group: Option<usize>,
}

impl FinancialPatternExtractor {
    /// Create extractor from database patterns
    pub fn from_patterns(db_patterns: Vec<FinancialPattern>) -> anyhow::Result<Self> {
        let mut patterns = Vec::new();

        for pattern in db_patterns {
            let regex = Regex::new(&pattern.regex_pattern)?;

            patterns.push(CompiledPattern {
                id: pattern.id,
                name: pattern.name,
                regex,
                document_type: pattern.document_type,
                status: pattern.status,
                confidence: pattern.confidence,
                amount_group: pattern.amount_group,
                vendor_group: pattern.vendor_group,
                date_group: pattern.date_group,
            });
        }

        Ok(Self { patterns })
    }

    /// Extract transactions from text
    pub fn extract_from_text(&self, text: &str) -> Vec<(FinancialTransaction, i64)> {
        let mut results = Vec::new();

        for pattern in &self.patterns {
            if let Some(caps) = pattern.regex.captures(text) {
                if let Some(transaction) = self.extract_transaction(&pattern, &caps) {
                    results.push((transaction, pattern.id));
                }
            }
        }

        results
    }

    fn extract_transaction(
        &self,
        pattern: &CompiledPattern,
        captures: &regex::Captures,
    ) -> Option<FinancialTransaction> {
        // Extract amount
        let amount_str = captures.get(pattern.amount_group)?.as_str();
        let amount = self.parse_amount(amount_str)?;

        // Extract vendor if specified
        let vendor = pattern.vendor_group
            .and_then(|g| captures.get(g))
            .map(|m| m.as_str().trim().to_string());

        // Extract date if specified
        let date = pattern.date_group
            .and_then(|g| captures.get(g))
            .map(|m| m.as_str().to_string());

        // Build transaction
        Some(FinancialTransaction {
            id: 0,
            source_type: String::new(),
            source_id: String::new(),
            extraction_job_id: None,
            document_type: pattern.document_type.clone(),
            description: captures.get(0)?.as_str().to_string(),
            amount,
            currency: "USD".to_string(),
            transaction_date: date.unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string()),
            category: None,
            vendor,
            status: pattern.status.clone(),
            source_file: None,
            confidence: Some(pattern.confidence),
            requires_review: false,
            extracted_at: chrono::Utc::now().timestamp(),
            created_at: 0,
            updated_at: 0,
            notes: Some(format!("Extracted using pattern: {}", pattern.name)),
        })
    }

    fn parse_amount(&self, amount_str: &str) -> Option<f64> {
        amount_str
            .replace(",", "")
            .replace("$", "")
            .trim()
            .parse()
            .ok()
    }
}
```

### Phase 6: Update Extraction Manager

**File:** `dwata-api/src/jobs/financial_extraction_manager.rs`

**Modify to:**
1. Load patterns from database at startup
2. Track which pattern matched (for statistics)
3. Update pattern match counts

```rust
use crate::database::{financial_patterns as patterns_db, ...};
use extractors::FinancialPatternExtractor;

pub struct FinancialExtractionManager {
    db_conn: AsyncDbConnection,
}

impl FinancialExtractionManager {
    pub fn new(db_conn: AsyncDbConnection) -> Self {
        Self { db_conn }
    }

    /// Extract from emails using current active patterns
    pub async fn extract_from_emails(
        &self,
        email_ids: Option<Vec<i64>>,
    ) -> Result<ExtractionStats> {
        // Load active patterns from database
        let db_patterns = patterns_db::list_active_patterns(self.db_conn.clone()).await?;

        if db_patterns.is_empty() {
            return Err(anyhow::anyhow!("No active patterns found"));
        }

        // Build extractor
        let extractor = FinancialPatternExtractor::from_patterns(db_patterns)?;

        // Get emails to process
        let emails = self.get_emails_to_process(email_ids).await?;

        let mut stats = ExtractionStats::default();

        for email in emails {
            // Check if already processed
            if self.is_already_processed(&email).await? {
                stats.skipped += 1;
                continue;
            }

            // Extract transactions
            let text = format!("{}\n\n{}",
                email.subject.unwrap_or_default(),
                email.body_text.unwrap_or_default()
            );

            let results = extractor.extract_from_text(&text);

            for (mut transaction, pattern_id) in results {
                transaction.source_type = "email".to_string();
                transaction.source_id = email.id.to_string();

                // Save transaction
                db::insert_financial_transaction(
                    self.db_conn.clone(),
                    &transaction,
                    "email",
                    &email.id.to_string(),
                    None,
                ).await?;

                // Update pattern statistics
                patterns_db::increment_match_count(
                    self.db_conn.clone(),
                    pattern_id,
                ).await?;

                patterns_db::update_last_matched(
                    self.db_conn.clone(),
                    pattern_id,
                    chrono::Utc::now().timestamp(),
                ).await?;

                stats.extracted += 1;
            }

            // Mark source as processed
            if !results.is_empty() {
                sources_db::mark_source_processed(
                    self.db_conn.clone(),
                    "email",
                    &email.id.to_string(),
                    None,
                    results.len() as i32,
                ).await?;
            }

            stats.processed += 1;
        }

        Ok(stats)
    }
}

#[derive(Default)]
pub struct ExtractionStats {
    pub processed: usize,
    pub extracted: usize,
    pub skipped: usize,
}
```

### Phase 7: TypeScript Types

Generate TypeScript types for the new `FinancialPattern` type:

**Add to** `shared-types/src/lib.rs`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FinancialPattern {
    pub id: i64,
    pub name: String,
    pub regex_pattern: String,
    pub description: Option<String>,
    pub document_type: String,
    pub status: String,
    pub confidence: f32,
    pub amount_group: usize,
    pub vendor_group: Option<usize>,
    pub date_group: Option<usize>,
    pub is_default: bool,
    pub is_active: bool,
    pub match_count: i32,
    pub last_matched_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}
```

Run:
```bash
cargo run --bin generate_api_types
```

## Testing Strategy

### Unit Tests

**Test pattern validation:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_pattern() {
        let result = validate_pattern(
            "test_pattern",
            r"(?i)payment of \$?([\d,]+\.?\d{0,2}) to ([A-Za-z\s]+)",
            1,
            Some(2),
            None,
            0.90,
            "payment-confirmation",
            "paid",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_regex() {
        let result = validate_pattern(
            "bad_pattern",
            r"(?i)payment of \$?([\d,]+\.?\d{0,2} to ([A-Za-z\s]+)", // Missing )
            1,
            Some(2),
            None,
            0.90,
            "payment-confirmation",
            "paid",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_capture_group() {
        let result = validate_pattern(
            "test_pattern",
            r"(?i)payment of \$?([\d,]+\.?\d{0,2})",
            5, // Group doesn't exist
            None,
            None,
            0.90,
            "payment-confirmation",
            "paid",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_pattern_name_format() {
        assert!(validate_pattern_name("valid_name_123").is_ok());
        assert!(validate_pattern_name("Invalid-Name").is_err());
        assert!(validate_pattern_name("Invalid Name").is_err());
        assert!(validate_pattern_name("UPPERCASE").is_err());
    }
}
```

### Integration Tests

**Test database operations:**
```rust
#[tokio::test]
async fn test_create_and_retrieve_pattern() {
    let db = setup_test_db().await;

    let pattern = FinancialPattern {
        name: "test_pattern".to_string(),
        regex_pattern: r"test \$?([\d,]+\.?\d{0,2})".to_string(),
        // ... other fields
    };

    let id = patterns_db::insert_pattern(db.clone(), &pattern).await.unwrap();
    let retrieved = patterns_db::get_pattern(db.clone(), id).await.unwrap();

    assert_eq!(retrieved.name, "test_pattern");
}

#[tokio::test]
async fn test_duplicate_pattern_rejected() {
    let db = setup_test_db().await;

    let pattern = FinancialPattern { /* ... */ };

    patterns_db::insert_pattern(db.clone(), &pattern).await.unwrap();

    // Should fail on second insert
    let result = patterns_db::insert_pattern(db.clone(), &pattern).await;
    assert!(result.is_err());
}
```

### API Tests

**Test endpoints:**
```bash
# List patterns
curl http://localhost:8080/api/financial/patterns

# Create pattern
curl -X POST http://localhost:8080/api/financial/patterns \
  -H "Content-Type: application/json" \
  -d '{
    "name": "venmo_payment",
    "regex_pattern": "(?i)you received \\$([\\d,]+\\.\\d{2}) from (@[A-Za-z0-9_]+)",
    "description": "Venmo payment received",
    "document_type": "payment-confirmation",
    "status": "paid",
    "confidence": 0.85,
    "amount_group": 1,
    "vendor_group": 2
  }'

# Test pattern
curl -X POST http://localhost:8080/api/financial/patterns/test \
  -H "Content-Type: application/json" \
  -d '{
    "regex_pattern": "(?i)payment of \\$?([\\d,]+\\.?\\d{0,2}) to ([A-Za-z\\s]+)",
    "test_text": "Your payment of $150.00 to Comcast was successful",
    "amount_group": 1,
    "vendor_group": 2
  }'

# Toggle pattern
curl -X PATCH http://localhost:8080/api/financial/patterns/1/toggle \
  -H "Content-Type: application/json" \
  -d '{"is_active": false}'
```

## Migration Path from Hardcoded Patterns

1. **Run migration** - Creates table and seeds default patterns
2. **Update extractor** - Load patterns from DB instead of hardcoded
3. **Verify extraction still works** - Test with existing emails
4. **Remove hardcoded patterns** - Clean up old code
5. **Document for users** - How to add custom patterns

## Success Criteria

**Core Requirements:**
- ✅ Database schema created and migrations run successfully
- ✅ 15+ default patterns seeded from migration
- ✅ Core API endpoints (1-5) implemented and tested
- ✅ Pattern validation prevents invalid patterns
- ✅ Extractor successfully loads patterns from database
- ✅ Extraction produces same results as hardcoded version
- ✅ Users can create custom patterns via API
- ✅ Users can update and toggle patterns via API
- ✅ Pattern statistics (match_count, last_matched_at) update correctly
- ✅ Duplicate patterns rejected by unique constraints
- ✅ TypeScript types generated for frontend

**Optional (Future Enhancement):**
- Pattern testing endpoint (test before saving)
- Delete custom patterns endpoint
- Bulk import/export functionality

## File Checklist

- [ ] `dwata-api/src/database/migrations.rs` - Add table and seed data
- [ ] `dwata-api/src/database/financial_patterns.rs` - Database operations (NEW)
- [ ] `dwata-api/src/database/mod.rs` - Export financial_patterns module
- [ ] `dwata-api/src/helpers/pattern_validator.rs` - Pattern validation (NEW)
- [ ] `dwata-api/src/helpers/mod.rs` - Export pattern_validator module
- [ ] `dwata-api/src/handlers/financial.rs` - Add pattern management endpoints
- [ ] `dwata-api/src/main.rs` - Register pattern routes
- [ ] `extractors/src/financial_patterns/extractor.rs` - Load patterns from DB
- [ ] `extractors/src/financial_patterns/mod.rs` - Update types
- [ ] `dwata-api/src/jobs/financial_extraction_manager.rs` - Update to use DB patterns
- [ ] `shared-types/src/lib.rs` - Add FinancialPattern type
- [ ] `shared-types/src/bin/generate_api_types.rs` - Run to generate TS types

## Future Enhancements

**Deferred from this task (optional endpoints):**
1. **Pattern Testing API** - Test pattern endpoint before saving (POST /api/financial/patterns/test)
2. **Delete Pattern API** - Delete custom patterns endpoint (DELETE /api/financial/patterns/:id)
3. **Bulk Import/Export** - Import/export pattern sets (POST /import, GET /export)

**Additional enhancements:**
4. **Pattern Testing UI** - GUI for testing patterns against sample text
5. **Pattern Analytics** - Dashboard showing which patterns are most effective
6. **Pattern Suggestions** - Analyze failed extractions and suggest new patterns
7. **Pattern Optimization** - Identify slow patterns and suggest improvements
8. **Multi-language Support** - Patterns for non-English financial emails
9. **Pattern Versioning** - Track pattern changes over time
10. **A/B Testing** - Test new patterns without affecting production
11. **Machine Learning Integration** - Learn new patterns from user corrections
12. **Pattern Library** - Community-contributed patterns for popular services
