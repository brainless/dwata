# Task: Financial Email Text Pattern Extractor

**Status**: Implementation Complete (Needs Testing)
**Priority**: High
**Created**: 2026-02-02

## Objective

Implement a pattern-based extractor to extract financial transaction data from email body text. Focus on high-reliability patterns for common financial notifications (bills, payments, invoices).

**Target**: 15-20 reliable patterns covering the most common financial email types.

## Background

Many financial services send plain-text email notifications:
- Payment confirmations: "Your payment of $150.00 to Comcast was successful"
- Bill notifications: "Your Adobe bill of $99.99 is due on Feb 10"
- Payment received: "You received $3,500.00 from Acme Corp"
- Overdue notices: "Your Chase payment of $1,200 is overdue by 3 days"

These follow predictable patterns ideal for regex-based extraction with high confidence.

## Scope

**In Scope:**
- Extract from email body text (subject + body_text)
- 15-20 high-confidence patterns
- Core fields: amount, vendor, date, transaction type, status
- Individual transaction records (one per match)
- Storage in database

**Out of Scope:**
- PDF attachments (future)
- Image OCR (future)
- HTML-only emails (future)
- Currency conversion
- Categorization logic

## Database Schema

### Add to migrations.rs

```sql
CREATE TABLE IF NOT EXISTS financial_transactions (
    id INTEGER PRIMARY KEY DEFAULT nextval('seq_financial_transactions_id'),

    -- Source tracking (agnostic to source type)
    source_type VARCHAR NOT NULL,              -- 'email', 'document', 'chat', 'file', etc.
    source_id VARCHAR NOT NULL,                -- ID in the source system (email_id, file_path, etc.)
    extraction_job_id INTEGER,                 -- Extraction job that created this

    -- Transaction data
    document_type VARCHAR NOT NULL,            -- invoice, bill, receipt, payment-confirmation, etc.
    description VARCHAR NOT NULL,
    amount FLOAT NOT NULL,
    currency VARCHAR NOT NULL DEFAULT 'USD',
    transaction_date VARCHAR NOT NULL,         -- ISO date string

    -- Additional fields
    category VARCHAR,                          -- income, expense, subscription, etc.
    vendor VARCHAR,
    status VARCHAR NOT NULL,                   -- paid, pending, overdue, etc.

    -- Metadata
    source_file VARCHAR,                       -- Original file name/path if applicable
    confidence FLOAT,
    requires_review BOOLEAN DEFAULT false,

    -- Timestamps
    extracted_at BIGINT NOT NULL,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,

    notes VARCHAR
);

CREATE INDEX IF NOT EXISTS idx_financial_transactions_source
    ON financial_transactions(source_type, source_id);

CREATE INDEX IF NOT EXISTS idx_financial_transactions_date
    ON financial_transactions(transaction_date DESC);

CREATE INDEX IF NOT EXISTS idx_financial_transactions_vendor
    ON financial_transactions(vendor);
```

## Implementation

### Phase 1: Pattern Definitions

**File**: `extractors/src/financial_patterns/mod.rs`

Create 15-20 patterns organized by transaction type:

#### Payment Confirmation Patterns (5 patterns)
```rust
// "payment of $150.00 to Comcast"
// "paid $99 to Adobe"
// "Your $50.00 payment to Netflix"
// "successfully paid $1,200.00 to Chase"
// "payment processed: $45.99 to Spotify"
```

#### Bill/Invoice Due Patterns (4 patterns)
```rust
// "bill of $99.99 is due on Feb 10"
// "invoice for $3,500 due January 25"
// "Your Adobe bill ($99) is due Feb 10"
// "due: $150.00 by 02/05/2026"
```

#### Payment Received Patterns (3 patterns)
```rust
// "received $3,500.00 from Acme Corp"
// "payment of $2,000 from TechStart Inc"
// "You received a payment: $1,500 from Client Name"
```

#### Overdue/Late Patterns (3 patterns)
```rust
// "payment of $1,200 is overdue"
// "$450.00 payment past due"
// "overdue bill: $99 (3 days late)"
```

#### Pattern Structure

```rust
pub struct FinancialPattern {
    pub name: String,
    pub regex: Regex,
    pub transaction_type: FinancialDocumentType,
    pub status: TransactionStatus,
    pub confidence: f32,
    pub amount_group: usize,      // Capture group index for amount
    pub vendor_group: Option<usize>,
    pub date_group: Option<usize>,
}

impl FinancialPattern {
    pub fn extract(&self, text: &str) -> Option<FinancialTransaction> {
        // Extract using regex capture groups
        // Return transaction with extracted fields
    }
}
```

### Phase 2: Financial Pattern Extractor

**File**: `extractors/src/financial_patterns/extractor.rs`

```rust
use shared_types::{
    FinancialTransaction, FinancialDocumentType, TransactionStatus,
    TransactionCategory, ExtractionInput, ExtractionResult,
};
use regex::Regex;
use chrono::Utc;

pub struct FinancialPatternExtractor {
    patterns: Vec<FinancialPattern>,
}

impl FinancialPatternExtractor {
    pub fn new() -> Self {
        Self {
            patterns: create_financial_patterns(),
        }
    }

    pub fn extract_from_text(
        &self,
        text: &str,
    ) -> Vec<FinancialTransaction> {
        let mut transactions = Vec::new();

        for pattern in &self.patterns {
            if let Some(transaction) = pattern.extract(text) {
                let mut txn = transaction;
                txn.id = 0; // Will be set by DB
                txn.extracted_at = Utc::now().timestamp();
                transactions.push(txn);
            }
        }

        transactions
    }

    pub fn extract_from_email(
        &self,
        subject: &str,
        body_text: &str,
    ) -> Vec<FinancialTransaction> {
        let text = format!("{}\n\n{}", subject, body_text);
        self.extract_from_text(&text)
    }
}

fn create_financial_patterns() -> Vec<FinancialPattern> {
    vec![
        // Pattern 1: "payment of $X to Vendor"
        FinancialPattern {
            name: "payment_to_vendor".to_string(),
            regex: Regex::new(
                r"(?i)payment of \$?([\d,]+\.?\d{0,2}) to ([A-Za-z\s]+)"
            ).unwrap(),
            transaction_type: FinancialDocumentType::PaymentConfirmation,
            status: TransactionStatus::Paid,
            confidence: 0.90,
            amount_group: 1,
            vendor_group: Some(2),
            date_group: None,
        },

        // Pattern 2: "bill of $X due on Date"
        FinancialPattern {
            name: "bill_due_explicit".to_string(),
            regex: Regex::new(
                r"(?i)bill (?:of|for) \$?([\d,]+\.?\d{0,2}) (?:is )?due (?:on )?([A-Za-z]+ \d{1,2})"
            ).unwrap(),
            transaction_type: FinancialDocumentType::Bill,
            status: TransactionStatus::Pending,
            confidence: 0.88,
            amount_group: 1,
            vendor_group: None,
            date_group: Some(2),
        },

        // Pattern 3: "received $X from Client"
        FinancialPattern {
            name: "payment_received".to_string(),
            regex: Regex::new(
                r"(?i)received (?:a payment of )?\$?([\d,]+\.?\d{0,2}) from ([A-Za-z\s]+)"
            ).unwrap(),
            transaction_type: FinancialDocumentType::Invoice,
            status: TransactionStatus::Paid,
            confidence: 0.92,
            amount_group: 1,
            vendor_group: Some(2),
            date_group: None,
        },

        // Add 12-17 more patterns here...
    ]
}
```

### Phase 3: Database Integration

**File**: `dwata-api/src/database/financial_transactions.rs`

```rust
use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::FinancialTransaction;

pub async fn insert_financial_transaction(
    conn: AsyncDbConnection,
    transaction: &FinancialTransaction,
    source_type: &str,
    source_id: &str,
    extraction_job_id: Option<i64>,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    let document_type = match transaction.document_type {
        FinancialDocumentType::Invoice => "invoice",
        FinancialDocumentType::Bill => "bill",
        FinancialDocumentType::Receipt => "receipt",
        FinancialDocumentType::PaymentConfirmation => "payment-confirmation",
        _ => "other",
    };

    let status = match transaction.status {
        TransactionStatus::Paid => "paid",
        TransactionStatus::Pending => "pending",
        TransactionStatus::Overdue => "overdue",
        _ => "pending",
    };

    let category = transaction.category.as_ref().map(|c| match c {
        TransactionCategory::Income => "income",
        TransactionCategory::Expense => "expense",
        TransactionCategory::Subscription => "subscription",
        _ => "other",
    });

    let id: i64 = conn.query_row(
        "INSERT INTO financial_transactions
         (source_type, source_id, extraction_job_id, document_type, description, amount, currency,
          transaction_date, category, vendor, status, source_file, confidence,
          requires_review, extracted_at, created_at, updated_at, notes)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
        duckdb::params![
            source_type,
            source_id,
            extraction_job_id,
            document_type,
            &transaction.description,
            transaction.amount,
            &transaction.currency,
            &transaction.transaction_date,
            category,
            transaction.vendor.as_ref(),
            status,
            transaction.source_file.as_ref(),
            0.85, // Default confidence for pattern matching
            false,
            transaction.extracted_at,
            now,
            now,
            transaction.notes.as_ref(),
        ],
        |row| row.get(0),
    )?;

    Ok(id)
}

pub async fn list_financial_transactions(
    conn: AsyncDbConnection,
    limit: usize,
) -> Result<Vec<FinancialTransaction>> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, document_type, description, amount, currency, transaction_date,
                category, vendor, status, source_file, extracted_at, notes
         FROM financial_transactions
         ORDER BY transaction_date DESC
         LIMIT ?"
    )?;

    let rows = stmt.query_map([limit], |row| {
        // Map row to FinancialTransaction
        // Parse document_type, status, category strings back to enums
        Ok(FinancialTransaction {
            id: row.get(0)?,
            // ... map other fields
        })
    })?;

    let mut transactions = Vec::new();
    for row_result in rows {
        transactions.push(row_result?);
    }

    Ok(transactions)
}

pub async fn get_financial_summary(
    conn: AsyncDbConnection,
    start_date: &str,
    end_date: &str,
) -> Result<FinancialSummary> {
    let conn = conn.lock().await;

    // Query for income
    let total_income: f64 = conn.query_row(
        "SELECT COALESCE(SUM(amount), 0.0)
         FROM financial_transactions
         WHERE category = 'income'
           AND transaction_date >= ?
           AND transaction_date <= ?",
        [start_date, end_date],
        |row| row.get(0),
    )?;

    // Query for expenses
    let total_expenses: f64 = conn.query_row(
        "SELECT COALESCE(SUM(ABS(amount)), 0.0)
         FROM financial_transactions
         WHERE category = 'expense'
           AND transaction_date >= ?
           AND transaction_date <= ?",
        [start_date, end_date],
        |row| row.get(0),
    )?;

    // Count pending/overdue
    let (pending_bills, overdue_payments): (i32, i32) = conn.query_row(
        "SELECT
            SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END) as pending,
            SUM(CASE WHEN status = 'overdue' THEN 1 ELSE 0 END) as overdue
         FROM financial_transactions
         WHERE transaction_date >= ?",
        [start_date],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    Ok(FinancialSummary {
        total_income,
        total_expenses,
        net_balance: total_income - total_expenses,
        pending_bills,
        overdue_payments,
        currency: "USD".to_string(),
        period_start: start_date.to_string(),
        period_end: end_date.to_string(),
    })
}
```

### Phase 4: Extraction Job Integration

**File**: `dwata-api/src/jobs/financial_extraction_manager.rs`

```rust
use crate::database::{financial_transactions as db, emails as emails_db};
use crate::database::AsyncDbConnection;
use anyhow::Result;
use extractors::FinancialPatternExtractor;

pub struct FinancialExtractionManager {
    db_conn: AsyncDbConnection,
    extractor: FinancialPatternExtractor,
}

impl FinancialExtractionManager {
    pub fn new(db_conn: AsyncDbConnection) -> Self {
        Self {
            db_conn,
            extractor: FinancialPatternExtractor::new(),
        }
    }

    /// Extract financial transactions from all emails
    pub async fn extract_from_emails(
        &self,
        email_ids: Option<Vec<i64>>,
    ) -> Result<usize> {
        let emails = if let Some(ids) = email_ids {
            // Get specific emails
            let mut emails = Vec::new();
            for id in ids {
                if let Ok(email) = emails_db::get_email(self.db_conn.clone(), id).await {
                    emails.push(email);
                }
            }
            emails
        } else {
            // Get recent emails (last 1000)
            emails_db::list_emails(self.db_conn.clone(), 1000).await?
        };

        let mut total_extracted = 0;

        for email in emails {
            let transactions = self.extractor.extract_from_email(
                &email.subject,
                &email.body_text,
            );

            for transaction in transactions {
                db::insert_financial_transaction(
                    self.db_conn.clone(),
                    &transaction,
                    "email",                      // source_type
                    &email.id.to_string(),        // source_id
                    None,                         // No extraction job for now
                ).await?;

                total_extracted += 1;
            }
        }

        Ok(total_extracted)
    }
}
}
```

### Phase 5: API Endpoints

**File**: `dwata-api/src/handlers/financial.rs`

```rust
use actix_web::{web, HttpResponse, Result as ActixResult};
use crate::database::financial_transactions as db;
use crate::database::AsyncDbConnection;
use crate::jobs::financial_extraction_manager::FinancialExtractionManager;

/// GET /api/financial/transactions - List transactions
pub async fn list_transactions(
    db_conn: web::Data<AsyncDbConnection>,
) -> ActixResult<HttpResponse> {
    let transactions = db::list_financial_transactions(db_conn.as_ref().clone(), 100)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "transactions": transactions
    })))
}

/// GET /api/financial/summary - Get summary
pub async fn get_summary(
    db_conn: web::Data<AsyncDbConnection>,
    query: web::Query<SummaryQuery>,
) -> ActixResult<HttpResponse> {
    let summary = db::get_financial_summary(
        db_conn.as_ref().clone(),
        &query.start_date,
        &query.end_date,
    )
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(summary))
}

/// POST /api/financial/extract - Trigger extraction
pub async fn trigger_extraction(
    db_conn: web::Data<AsyncDbConnection>,
    request: web::Json<ExtractionRequest>,
) -> ActixResult<HttpResponse> {
    let manager = FinancialExtractionManager::new(db_conn.as_ref().clone());

    let count = manager
        .extract_from_emails(request.email_ids.clone())
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "extracted_count": count,
        "status": "completed"
    })))
}

#[derive(Deserialize)]
pub struct SummaryQuery {
    start_date: String,
    end_date: String,
}

#[derive(Deserialize)]
pub struct ExtractionRequest {
    email_ids: Option<Vec<i64>>,
}
```

**Register routes in main.rs:**

```rust
.route("/api/financial/transactions", web::get().to(financial::list_transactions))
.route("/api/financial/summary", web::get().to(financial::get_summary))
.route("/api/financial/extract", web::post().to(financial::trigger_extraction))
```

## Testing Strategy

1. **Unit tests** for each pattern with sample text
2. **Integration test** with real email data
3. **End-to-end** test: extract → store → API → GUI

## Success Criteria

- ✅ 15-20 reliable patterns implemented
- ✅ Transactions stored in database with email reference
- ✅ API endpoints return financial data
- ✅ GUI Financial Health page displays real data
- ✅ No false positives (high precision)
- ✅ Extraction completes within seconds for 1000 emails

## File Checklist

- [ ] `extractors/src/financial_patterns/mod.rs` - Pattern definitions
- [ ] `extractors/src/financial_patterns/extractor.rs` - Extractor implementation
- [ ] `dwata-api/src/database/migrations.rs` - Add financial_transactions table
- [ ] `dwata-api/src/database/financial_transactions.rs` - DB operations
- [ ] `dwata-api/src/database/mod.rs` - Export financial_transactions module
- [ ] `dwata-api/src/jobs/financial_extraction_manager.rs` - Extraction manager
- [ ] `dwata-api/src/handlers/financial.rs` - API handlers
- [ ] `dwata-api/src/handlers/mod.rs` - Export financial module
- [ ] `dwata-api/src/main.rs` - Register routes
- [ ] `extractors/src/lib.rs` - Export financial_patterns module

## Next Steps After Completion

Once this is working reliably:
1. Add more patterns based on actual email corpus
2. Enhance date parsing for various formats
3. Add vendor name normalization
4. Implement transaction categorization logic
