# Task: Financial Data Extractor Agent with LLM-Powered Pattern Generation

## Objective

Create an AI agent that generates regex patterns to extract financial transaction data from emails. The agent will use nocodo's LLM SDK and agent framework to analyze email content, propose regex patterns, test them, and save validated patterns to the database.

## Background

### Current State
- **Pattern Storage**: Regex patterns stored in `financial_patterns` table with:
  - Regex pattern string
  - Capture group indices (amount_group, vendor_group, date_group)
  - Document type, status, confidence score
  - Usage statistics (match_count, last_matched_at)

- **Pattern Application**: `FinancialPatternExtractor` in `extractors` crate:
  - Loads patterns from DB or uses hardcoded defaults
  - Applies regex patterns to email text
  - Extracts `FinancialTransaction` objects

- **Manual Pattern Creation**: Currently patterns are hardcoded in `extractors/src/financial_patterns/mod.rs` (~25 patterns)

### Why This Matters

1. **Scalability**: Manual regex pattern creation doesn't scale to the variety of financial emails users receive
2. **Customization**: Different users get emails from different vendors with different formats
3. **Maintenance**: Email formats change over time, requiring pattern updates
4. **User Experience**: Users shouldn't need to write regex patterns manually
5. **Learning System**: Agent can learn from user's actual emails and improve over time

### nocodo Framework Integration

**nocodo-llm-sdk** provides:
- Unified `LlmClient` trait supporting multiple providers (Claude, Gemini, Grok, etc.)
- Type-safe tool calling via `Tool::from_type::<T>()` and `.parse_arguments()`
- Builder pattern for message construction
- Zero-cost abstractions with provider-specific optimizations

**nocodo-agents** provides:
- `Agent` trait with system prompt, tools, and execution loop
- Session storage via `AgentStorage` trait
- Agentic execution: LLM → Tool Call → Tool Response → Continue until done
- Message history tracking for debugging

**nocodo-tools** provides:
- `ToolExecutor` for executing various tools (filesystem, bash, sqlite, etc.)
- Type-safe `ToolRequest` → `ToolResponse` pattern
- Security model with permission checking

## Architecture

### Component Diagram

```
┌────────────────────────────────────────────────────────────────┐
│                         GUI (SolidJS)                          │
│  - User selects email                                          │
│  - Clicks "Generate Pattern" button                            │
│  - Shows agent progress in real-time                           │
│  - Displays extracted data preview                             │
│  - Approve/Reject pattern                                      │
└────────────────────────────────────────────────────────────────┘
                             ↓ HTTP POST
┌────────────────────────────────────────────────────────────────┐
│                    dwata-api (Actix-web)                       │
│  POST /api/extraction/generate-pattern                         │
│  - Fetches email content                                       │
│  - Fetches existing patterns                                   │
│  - Spawns agent execution                                      │
│  - Returns session ID                                          │
└────────────────────────────────────────────────────────────────┘
                             ↓
┌────────────────────────────────────────────────────────────────┐
│             dwata-agents::FinancialExtractorAgent              │
│  - System prompt with email + existing patterns                │
│  - Tools: test_pattern, save_pattern                           │
│  - Iterates: propose → test → refine → save                    │
└────────────────────────────────────────────────────────────────┘
            ↓                                    ↓
┌─────────────────────────┐      ┌─────────────────────────────┐
│  DwataToolExecutor      │      │  SqliteAgentStorage         │
│  - test_pattern tool    │      │  - Persist agent sessions   │
│  - save_pattern tool    │      │  - Store messages           │
│  - Pattern validation   │      │  - Track tool calls         │
└─────────────────────────┘      └─────────────────────────────┘
            ↓
┌────────────────────────────────────────────────────────────────┐
│                    dwata SQLite Database                       │
│  - financial_patterns table                                    │
│  - agent_sessions table (NEW)                                  │
│  - agent_messages table (NEW)                                  │
│  - agent_tool_calls table (NEW)                                │
└────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
1. User Action:
   [User selects email #123] → [Clicks "Generate Pattern"]

2. API Request:
   POST /api/extraction/generate-pattern
   Body: { email_id: 123 }

3. Agent Initialization:
   - Fetch email (subject + body)
   - Fetch existing patterns from DB
   - Create FinancialExtractorAgent
   - Create agent session in DB

4. Agent Execution Loop (max 30 iterations):

   LLM Prompt:
   └─> "You are a pattern generator. Here's an email: [EMAIL]
        Existing patterns: [PATTERNS]
        Generate a regex to extract financial data."

   LLM Response:
   └─> "I'll create a pattern for this payment confirmation..."
       Tool Call: test_pattern(regex="payment of \$?([\d,]+) to ([A-Za-z\s]+)", ...)

   Tool Execution:
   └─> DwataToolExecutor.execute(test_pattern)
       → Compile regex
       → Apply to email text
       → Return extracted transactions

   LLM Sees Result:
   └─> "The pattern extracted: amount=$1,234.56, vendor=Stripe Inc."
       If correct: Tool Call: save_pattern(...)
       If wrong: "Let me refine the pattern..." → test_pattern again

   Pattern Saved:
   └─> Insert into financial_patterns table
       Return pattern ID

5. API Response:
   {
     status: "completed",
     pattern_id: 42,
     extracted_data: [{ amount: 1234.56, vendor: "Stripe Inc", ... }]
   }

6. User Review:
   - User sees extracted data
   - Approves → Pattern activated (is_active=true)
   - Rejects → Pattern marked inactive
```

## Implementation Plan

### Phase 1: Database Schema for Agent Storage

#### Step 1.1: Add Agent Session Tables

Add to `dwata-api/src/database/migrations.rs`:

```sql
-- Agent sessions
CREATE TABLE agent_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Session identity
    agent_type VARCHAR NOT NULL,        -- "financial-extractor"
    objective VARCHAR NOT NULL,

    -- Session context
    user_id INTEGER,                    -- Future: multi-user support
    context_data TEXT,                  -- JSON: email_id, credential_id, etc.

    -- Session state
    status VARCHAR NOT NULL,            -- "running", "completed", "failed"
    result TEXT,                        -- Final result or error message

    -- Timestamps
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    completed_at BIGINT
);

CREATE INDEX idx_agent_sessions_status ON agent_sessions(status, created_at DESC);
CREATE INDEX idx_agent_sessions_type ON agent_sessions(agent_type, created_at DESC);

-- Agent messages (conversation history)
CREATE TABLE agent_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,

    -- Message data
    role VARCHAR NOT NULL,              -- "user", "assistant"
    content TEXT NOT NULL,              -- Message text or JSON

    -- Metadata
    created_at BIGINT NOT NULL,

    FOREIGN KEY(session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_agent_messages_session ON agent_messages(session_id, created_at ASC);

-- Agent tool calls (tracks tool execution)
CREATE TABLE agent_tool_calls (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    message_id INTEGER,                 -- Which message contained the tool call

    -- Tool call data
    tool_name VARCHAR NOT NULL,
    tool_input TEXT NOT NULL,           -- JSON parameters
    tool_output TEXT,                   -- JSON result

    -- Status
    status VARCHAR NOT NULL,            -- "pending", "success", "error"
    error_message TEXT,

    -- Timing
    created_at BIGINT NOT NULL,
    executed_at BIGINT,
    completed_at BIGINT,

    FOREIGN KEY(session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_agent_tool_calls_session ON agent_tool_calls(session_id, created_at ASC);
CREATE INDEX idx_agent_tool_calls_status ON agent_tool_calls(status);
```

#### Step 1.2: Create Database Query Functions

Create `dwata-api/src/database/agent_sessions.rs`:

```rust
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: i64,
    pub agent_type: String,
    pub objective: String,
    pub user_id: Option<i64>,
    pub context_data: Option<String>,
    pub status: String,
    pub result: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: i64,
    pub session_id: i64,
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolCall {
    pub id: i64,
    pub session_id: i64,
    pub message_id: Option<i64>,
    pub tool_name: String,
    pub tool_input: String,
    pub tool_output: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub executed_at: Option<i64>,
    pub completed_at: Option<i64>,
}

pub async fn create_session(
    conn: AsyncDbConnection,
    agent_type: &str,
    objective: &str,
    context_data: Option<&str>,
) -> anyhow::Result<i64>;

pub async fn update_session_status(
    conn: AsyncDbConnection,
    session_id: i64,
    status: &str,
    result: Option<&str>,
) -> anyhow::Result<()>;

pub async fn create_message(
    conn: AsyncDbConnection,
    session_id: i64,
    role: &str,
    content: &str,
) -> anyhow::Result<i64>;

pub async fn get_messages(
    conn: AsyncDbConnection,
    session_id: i64,
) -> anyhow::Result<Vec<AgentMessage>>;

pub async fn create_tool_call(
    conn: AsyncDbConnection,
    session_id: i64,
    tool_name: &str,
    tool_input: &str,
) -> anyhow::Result<i64>;

pub async fn update_tool_call(
    conn: AsyncDbConnection,
    tool_call_id: i64,
    status: &str,
    output: Option<&str>,
    error: Option<&str>,
) -> anyhow::Result<()>;
```

### Phase 2: Create dwata-agents Crate

#### Step 2.1: Initialize Crate

```bash
cd /Users/brainless/Projects/dwata
cargo new --lib dwata-agents
```

Update root `Cargo.toml`:
```toml
[workspace]
members = [
    "dwata-api",
    "extractors",
    "shared-types",
    "dwata-agents",  # NEW
]
```

#### Step 2.2: Configure Dependencies

`dwata-agents/Cargo.toml`:
```toml
[package]
name = "dwata-agents"
version = "0.1.0"
edition = "2021"

[dependencies]
# nocodo framework
nocodo-llm-sdk = { path = "../../nocodo/nocodo-llm-sdk" }

# Internal dependencies
shared-types = { path = "../shared-types" }

# Async runtime
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
schemars = "0.8"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Regex
regex = "1"

# Logging
tracing = "0.1"

# Date/time
chrono = "0.4"
```

#### Step 2.3: Create Crate Structure

```
dwata-agents/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── types.rs                      # Shared types for agents
    ├── storage/
    │   ├── mod.rs
    │   └── sqlite_storage.rs         # SqliteAgentStorage implementation
    ├── tools/
    │   ├── mod.rs
    │   ├── executor.rs               # DwataToolExecutor
    │   ├── test_pattern.rs           # test_pattern tool
    │   └── save_pattern.rs           # save_pattern tool
    └── financial_extractor/
        ├── mod.rs
        ├── agent.rs                  # FinancialExtractorAgent
        ├── system_prompt.rs          # System prompt template
        └── types.rs                  # Tool parameter types
```

### Phase 3: Implement Agent Storage

#### Step 3.1: Define Storage Trait

`dwata-agents/src/storage/mod.rs`:

```rust
use async_trait::async_trait;
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub mod sqlite_storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Option<i64>,
    pub agent_type: String,
    pub objective: String,
    pub context_data: Option<String>,
    pub status: String,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Option<i64>,
    pub session_id: i64,
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: Option<i64>,
    pub session_id: i64,
    pub tool_name: String,
    pub tool_input: String,
    pub tool_output: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
}

#[async_trait]
pub trait AgentStorage: Send + Sync {
    async fn create_session(&self, session: Session) -> Result<i64>;
    async fn get_session(&self, session_id: i64) -> Result<Option<Session>>;
    async fn update_session(&self, session: Session) -> Result<()>;

    async fn create_message(&self, message: Message) -> Result<i64>;
    async fn get_messages(&self, session_id: i64) -> Result<Vec<Message>>;

    async fn create_tool_call(&self, tool_call: ToolCall) -> Result<i64>;
    async fn update_tool_call(&self, tool_call: ToolCall) -> Result<()>;
    async fn get_tool_calls(&self, session_id: i64) -> Result<Vec<ToolCall>>;
}
```

#### Step 3.2: Implement SQLite Storage

`dwata-agents/src/storage/sqlite_storage.rs`:

```rust
use super::{AgentStorage, Message, Session, ToolCall};
use async_trait::async_trait;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SqliteAgentStorage {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteAgentStorage {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl AgentStorage for SqliteAgentStorage {
    async fn create_session(&self, session: Session) -> anyhow::Result<i64> {
        let conn = self.conn.lock().await;
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO agent_sessions
             (agent_type, objective, context_data, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                session.agent_type,
                session.objective,
                session.context_data,
                session.status,
                now,
                now,
            ],
        )?;

        let id = conn.last_insert_rowid();
        Ok(id)
    }

    async fn get_session(&self, session_id: i64) -> anyhow::Result<Option<Session>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT id, agent_type, objective, context_data, status, result
             FROM agent_sessions WHERE id = ?"
        )?;

        let session = stmt.query_row([session_id], |row| {
            Ok(Session {
                id: Some(row.get(0)?),
                agent_type: row.get(1)?,
                objective: row.get(2)?,
                context_data: row.get(3)?,
                status: row.get(4)?,
                result: row.get(5)?,
            })
        }).optional()?;

        Ok(session)
    }

    async fn update_session(&self, session: Session) -> anyhow::Result<()> {
        let conn = self.conn.lock().await;
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "UPDATE agent_sessions
             SET status = ?, result = ?, updated_at = ?, completed_at = ?
             WHERE id = ?",
            rusqlite::params![
                session.status,
                session.result,
                now,
                if session.status == "completed" || session.status == "failed" {
                    Some(now)
                } else {
                    None
                },
                session.id.unwrap(),
            ],
        )?;

        Ok(())
    }

    async fn create_message(&self, message: Message) -> anyhow::Result<i64> {
        let conn = self.conn.lock().await;
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO agent_messages (session_id, role, content, created_at)
             VALUES (?, ?, ?, ?)",
            rusqlite::params![message.session_id, message.role, message.content, now],
        )?;

        Ok(conn.last_insert_rowid())
    }

    async fn get_messages(&self, session_id: i64) -> anyhow::Result<Vec<Message>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, content
             FROM agent_messages
             WHERE session_id = ?
             ORDER BY created_at ASC"
        )?;

        let messages = stmt.query_map([session_id], |row| {
            Ok(Message {
                id: Some(row.get(0)?),
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(messages)
    }

    async fn create_tool_call(&self, tool_call: ToolCall) -> anyhow::Result<i64> {
        let conn = self.conn.lock().await;
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO agent_tool_calls
             (session_id, tool_name, tool_input, status, created_at)
             VALUES (?, ?, ?, ?, ?)",
            rusqlite::params![
                tool_call.session_id,
                tool_call.tool_name,
                tool_call.tool_input,
                tool_call.status,
                now,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    async fn update_tool_call(&self, tool_call: ToolCall) -> anyhow::Result<()> {
        let conn = self.conn.lock().await;
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "UPDATE agent_tool_calls
             SET tool_output = ?, status = ?, error_message = ?,
                 executed_at = ?, completed_at = ?
             WHERE id = ?",
            rusqlite::params![
                tool_call.tool_output,
                tool_call.status,
                tool_call.error_message,
                now,
                now,
                tool_call.id.unwrap(),
            ],
        )?;

        Ok(())
    }

    async fn get_tool_calls(&self, session_id: i64) -> anyhow::Result<Vec<ToolCall>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT id, session_id, tool_name, tool_input, tool_output, status, error_message
             FROM agent_tool_calls
             WHERE session_id = ?
             ORDER BY created_at ASC"
        )?;

        let tool_calls = stmt.query_map([session_id], |row| {
            Ok(ToolCall {
                id: Some(row.get(0)?),
                session_id: row.get(1)?,
                tool_name: row.get(2)?,
                tool_input: row.get(3)?,
                tool_output: row.get(4)?,
                status: row.get(5)?,
                error_message: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(tool_calls)
    }
}
```

### Phase 4: Implement Tool Executor

#### Step 4.1: Define Tool Types

`dwata-agents/src/tools/mod.rs`:

```rust
pub mod executor;
pub mod test_pattern;
pub mod save_pattern;

pub use executor::DwataToolExecutor;
pub use test_pattern::TestPatternTool;
pub use save_pattern::SavePatternTool;

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TestPatternParams {
    pub regex_pattern: String,
    pub amount_group: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor_group: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_group: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SavePatternParams {
    pub name: String,
    pub regex_pattern: String,
    pub description: String,
    pub document_type: String,
    pub status: String,
    pub confidence: f32,
    pub amount_group: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor_group: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_group: Option<usize>,
}
```

#### Step 4.2: Implement Tool Executor

`dwata-agents/src/tools/executor.rs`:

```rust
use super::{SavePatternParams, TestPatternParams};
use anyhow::Result;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;
use shared_types::FinancialTransaction;

pub struct DwataToolExecutor {
    conn: Arc<Mutex<Connection>>,
    email_content: String,  // The email being analyzed
}

impl DwataToolExecutor {
    pub fn new(conn: Arc<Mutex<Connection>>, email_content: String) -> Self {
        Self { conn, email_content }
    }

    pub async fn test_pattern(&self, params: TestPatternParams) -> Result<Vec<FinancialTransaction>> {
        // Compile regex
        let regex = regex::Regex::new(&params.regex_pattern)?;

        // Apply to email content
        let mut transactions = Vec::new();

        for caps in regex.captures_iter(&self.email_content) {
            // Extract amount
            let amount = if let Some(amount_match) = caps.get(params.amount_group) {
                let amount_str = amount_match.as_str()
                    .replace(',', "")
                    .replace('$', "")
                    .trim()
                    .to_string();
                amount_str.parse::<f64>().ok()
            } else {
                None
            };

            // Extract vendor
            let vendor = params.vendor_group
                .and_then(|g| caps.get(g))
                .map(|m| m.as_str().trim().to_string());

            // Extract date
            let transaction_date = params.date_group
                .and_then(|g| caps.get(g))
                .map(|m| m.as_str().trim().to_string());

            if let Some(amount) = amount {
                transactions.push(FinancialTransaction {
                    id: 0,
                    source_type: "email".to_string(),
                    source_id: "test".to_string(),
                    document_type: shared_types::FinancialDocumentType::Receipt,
                    description: caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string(),
                    amount,
                    currency: "USD".to_string(),
                    transaction_date: transaction_date.unwrap_or_else(|| {
                        chrono::Utc::now().format("%Y-%m-%d").to_string()
                    }),
                    category: None,
                    vendor,
                    status: shared_types::TransactionStatus::Pending,
                    source_file: None,
                    extracted_at: chrono::Utc::now().timestamp(),
                    notes: None,
                });
            }
        }

        Ok(transactions)
    }

    pub async fn save_pattern(&self, params: SavePatternParams) -> Result<i64> {
        let conn = self.conn.lock().await;
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO financial_patterns
             (name, regex_pattern, description, document_type, status, confidence,
              amount_group, vendor_group, date_group, is_default, is_active,
              match_count, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, false, false, 0, ?, ?)",
            rusqlite::params![
                params.name,
                params.regex_pattern,
                params.description,
                params.document_type,
                params.status,
                params.confidence,
                params.amount_group as i64,
                params.vendor_group.map(|v| v as i64),
                params.date_group.map(|v| v as i64),
                now,
                now,
            ],
        )?;

        let pattern_id = conn.last_insert_rowid();
        Ok(pattern_id)
    }
}
```

### Phase 5: Implement Financial Extractor Agent

#### Step 5.1: Define System Prompt

`dwata-agents/src/financial_extractor/system_prompt.rs`:

```rust
use shared_types::FinancialPattern;

pub fn build_system_prompt(
    email_subject: &str,
    email_body: &str,
    existing_patterns: &[FinancialPattern],
) -> String {
    format!(
        r#"You are a financial data extraction pattern generator. Your goal is to create regex patterns that extract financial information from emails.

## Target Data Structure

You will extract a FinancialTransaction with these fields:
- **amount** (f64, REQUIRED): The transaction amount (e.g., 1234.56)
- **vendor** (String, OPTIONAL): Who the payment is to/from
- **transaction_date** (String, OPTIONAL): When the transaction occurred
- **document_type**: One of [invoice, bill, receipt, payment-confirmation, bank-statement, tax-document]
- **status**: One of [paid, pending, overdue, cancelled, refunded]

## Regex Pattern Requirements

Your regex pattern must:
1. Use standard Rust regex syntax (the `regex` crate)
2. Use numbered capture groups: (pattern) creates group 1, (pattern) creates group 2, etc.
3. The amount_group must capture numeric amounts (e.g., "1,234.56" or "1234.56")
4. The vendor_group (optional) should capture company/vendor names
5. The date_group (optional) should capture date strings

### Examples of Good Patterns

Pattern: `payment of \$?([\d,]+\.?\d{{0,2}}) to ([A-Za-z\s]+)`
- Group 1 (amount_group): captures "1,234.56"
- Group 2 (vendor_group): captures "Stripe Inc"
- Matches: "payment of $1,234.56 to Stripe Inc"

Pattern: `invoice for \$?([\d,]+\.?\d{{0,2}}) due ([A-Za-z]+ \d{{1,2}})`
- Group 1 (amount_group): captures "500.00"
- Group 2 (date_group): captures "January 15"
- Matches: "invoice for $500.00 due January 15"

## Existing Patterns (for reference)

{}

## Email to Analyze

**Subject:** {}

**Body:**
{}

## Your Task

1. Analyze the email content carefully
2. Identify financial information (amounts, vendors, dates)
3. Create a regex pattern with appropriate capture groups
4. Use the test_pattern tool to validate your regex
5. Iterate until the pattern extracts correct data
6. Use the save_pattern tool to persist the final pattern

## Available Tools

### test_pattern
Test a regex pattern against the email content.
Parameters:
- regex_pattern: The regex to test
- amount_group: Which capture group contains the amount (starting from 1)
- vendor_group: Optional - which capture group contains the vendor
- date_group: Optional - which capture group contains the date

Returns: List of extracted transactions

### save_pattern
Save a validated pattern to the database.
Parameters:
- name: Short name for the pattern (e.g., "stripe_payment_confirmation")
- regex_pattern: The validated regex
- description: What this pattern matches
- document_type: Type of document (payment-confirmation, invoice, bill, receipt, etc.)
- status: Transaction status (paid, pending, overdue, etc.)
- confidence: How confident you are in this pattern (0.0 to 1.0)
- amount_group: Which capture group has the amount
- vendor_group: Optional - which capture group has the vendor
- date_group: Optional - which capture group has the date

Returns: Pattern ID

## Important Notes

- Start with a simple pattern and refine it
- Test the pattern before saving
- If the pattern doesn't match, analyze why and adjust
- Make patterns specific enough to avoid false positives
- But not so specific that they only match one email
- Once you successfully save a pattern, your task is complete"#,
        format_existing_patterns(existing_patterns),
        email_subject,
        email_body,
    )
}

fn format_existing_patterns(patterns: &[FinancialPattern]) -> String {
    if patterns.is_empty() {
        return "No existing patterns yet.".to_string();
    }

    let mut output = String::new();
    output.push_str(&format!("Total patterns: {}\n\n", patterns.len()));

    for pattern in patterns.iter().take(10) {  // Show first 10
        output.push_str(&format!(
            "- **{}**: `{}` (doc_type: {}, status: {}, confidence: {:.2})\n",
            pattern.name,
            pattern.regex_pattern,
            pattern.document_type,
            pattern.status,
            pattern.confidence
        ));
    }

    if patterns.len() > 10 {
        output.push_str(&format!("\n... and {} more patterns\n", patterns.len() - 10));
    }

    output
}
```

#### Step 5.2: Implement Agent

`dwata-agents/src/financial_extractor/agent.rs`:

```rust
use crate::storage::{AgentStorage, Message, Session};
use crate::tools::{DwataToolExecutor, SavePatternParams, TestPatternParams};
use async_trait::async_trait;
use nocodo_llm_sdk::{LlmClient, Tool};
use shared_types::{FinancialPattern, FinancialTransaction};
use std::sync::Arc;

pub struct FinancialExtractorAgent {
    llm_client: Arc<dyn LlmClient>,
    storage: Arc<dyn AgentStorage>,
    tool_executor: Arc<DwataToolExecutor>,
    email_subject: String,
    email_body: String,
    existing_patterns: Vec<FinancialPattern>,
}

impl FinancialExtractorAgent {
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        storage: Arc<dyn AgentStorage>,
        tool_executor: Arc<DwataToolExecutor>,
        email_subject: String,
        email_body: String,
        existing_patterns: Vec<FinancialPattern>,
    ) -> Self {
        Self {
            llm_client,
            storage,
            tool_executor,
            email_subject,
            email_body,
            existing_patterns,
        }
    }

    pub async fn execute(&self, session_id: i64) -> anyhow::Result<String> {
        // Build system prompt
        let system_prompt = super::system_prompt::build_system_prompt(
            &self.email_subject,
            &self.email_body,
            &self.existing_patterns,
        );

        // Create initial user message
        let initial_message = "Please analyze this email and create a regex pattern to extract the financial data.";
        self.storage.create_message(Message {
            id: None,
            session_id,
            role: "user".to_string(),
            content: initial_message.to_string(),
        }).await?;

        // Define tools
        let test_pattern_tool = Tool::from_type::<TestPatternParams>()
            .name("test_pattern")
            .description("Test a regex pattern against the email content")
            .build();

        let save_pattern_tool = Tool::from_type::<SavePatternParams>()
            .name("save_pattern")
            .description("Save a validated pattern to the database")
            .build();

        let tools = vec![test_pattern_tool, save_pattern_tool];

        // Execution loop (max 30 iterations)
        for iteration in 0..30 {
            tracing::info!("Agent iteration {}", iteration);

            // Get conversation history
            let messages = self.storage.get_messages(session_id).await?;

            // Build LLM request
            let mut message_builder = self.llm_client.message_builder();
            message_builder = message_builder
                .system_message(&system_prompt)
                .max_tokens(4096);

            // Add conversation history
            for msg in &messages {
                if msg.role == "user" {
                    message_builder = message_builder.user_message(&msg.content);
                } else {
                    message_builder = message_builder.assistant_message(&msg.content);
                }
            }

            // Add tools
            for tool in &tools {
                message_builder = message_builder.tool(tool.clone());
            }

            // Call LLM
            let response = message_builder.send().await?;

            // Save assistant message
            self.storage.create_message(Message {
                id: None,
                session_id,
                role: "assistant".to_string(),
                content: response.text.clone(),
            }).await?;

            // Check for tool calls
            if let Some(tool_calls) = response.tool_calls {
                for tool_call in tool_calls {
                    // Execute tool
                    let tool_result = match tool_call.name.as_str() {
                        "test_pattern" => {
                            let params: TestPatternParams = tool_call.parse_arguments()?;
                            let transactions = self.tool_executor.test_pattern(params).await?;
                            serde_json::to_string(&transactions)?
                        }
                        "save_pattern" => {
                            let params: SavePatternParams = tool_call.parse_arguments()?;
                            let pattern_id = self.tool_executor.save_pattern(params).await?;
                            format!("Pattern saved with ID: {}", pattern_id)
                        }
                        _ => {
                            format!("Unknown tool: {}", tool_call.name)
                        }
                    };

                    // Save tool result as user message (LLM will see it in next iteration)
                    self.storage.create_message(Message {
                        id: None,
                        session_id,
                        role: "user".to_string(),
                        content: format!("Tool result: {}", tool_result),
                    }).await?;
                }

                // Continue loop to process tool results
                continue;
            }

            // No tool calls - agent is done
            return Ok(response.text);
        }

        Err(anyhow::anyhow!("Agent exceeded maximum iterations"))
    }
}
```

### Phase 6: Create API Handlers

#### Step 6.1: Add Pattern Generation Endpoint

Create `dwata-api/src/handlers/pattern_generation.rs`:

```rust
use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::database::Database;
use dwata_agents::{
    financial_extractor::FinancialExtractorAgent,
    storage::{sqlite_storage::SqliteAgentStorage, Session},
    tools::DwataToolExecutor,
};
use nocodo_llm_sdk::claude::ClaudeClient;

#[derive(Debug, Deserialize)]
pub struct GeneratePatternRequest {
    pub email_id: i64,
}

#[derive(Debug, Serialize)]
pub struct GeneratePatternResponse {
    pub session_id: i64,
    pub status: String,
    pub pattern_id: Option<i64>,
    pub extracted_data: Vec<shared_types::FinancialTransaction>,
}

#[actix_web::post("/api/extraction/generate-pattern")]
pub async fn generate_pattern(
    req: web::Json<GeneratePatternRequest>,
    db: web::Data<Arc<Database>>,
    config: web::Data<crate::config::ApiConfig>,
) -> anyhow::Result<impl Responder> {
    // 1. Fetch email
    let email = crate::database::emails::get_email(
        db.async_connection.clone(),
        req.email_id,
    ).await?;

    // 2. Fetch existing patterns
    let patterns = crate::database::financial_patterns::get_all_active(
        db.async_connection.clone(),
    ).await?;

    // 3. Create LLM client
    let api_key = config.api_keys.claude_api_key
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Claude API key not configured"))?;

    let llm_client: Arc<dyn nocodo_llm_sdk::LlmClient> = Arc::new(
        ClaudeClient::new(api_key)?
    );

    // 4. Create storage
    let storage: Arc<dyn dwata_agents::storage::AgentStorage> = Arc::new(
        SqliteAgentStorage::new(db.connection.clone())
    );

    // 5. Create tool executor
    let email_content = format!("{}\n\n{}", email.subject, email.body_text);
    let tool_executor = Arc::new(
        DwataToolExecutor::new(db.connection.clone(), email_content)
    );

    // 6. Create agent
    let agent = FinancialExtractorAgent::new(
        llm_client,
        storage.clone(),
        tool_executor,
        email.subject.clone(),
        email.body_text.clone(),
        patterns,
    );

    // 7. Create session
    let session_id = storage.create_session(Session {
        id: None,
        agent_type: "financial-extractor".to_string(),
        objective: format!("Generate pattern for email {}", req.email_id),
        context_data: Some(serde_json::json!({
            "email_id": req.email_id,
        }).to_string()),
        status: "running".to_string(),
        result: None,
    }).await?;

    // 8. Execute agent
    let result = agent.execute(session_id).await?;

    // 9. Update session status
    storage.update_session(Session {
        id: Some(session_id),
        agent_type: "financial-extractor".to_string(),
        objective: "".to_string(),
        context_data: None,
        status: "completed".to_string(),
        result: Some(result.clone()),
    }).await?;

    Ok(HttpResponse::Ok().json(GeneratePatternResponse {
        session_id,
        status: "completed".to_string(),
        pattern_id: None,  // Extract from result
        extracted_data: vec![],  // TODO: Extract from tool calls
    }))
}
```

#### Step 6.2: Register Route

Update `dwata-api/src/main.rs`:

```rust
mod handlers {
    // ... existing handlers ...
    pub mod pattern_generation;
}

// In configure_app():
.route(
    "/api/extraction/generate-pattern",
    web::post().to(handlers::pattern_generation::generate_pattern)
)
```

### Phase 7: GUI Integration

#### Step 7.1: Add Generate Pattern Button

Update `gui/src/pages/Emails.tsx`:

```tsx
import { createSignal } from "solid-js";

function EmailList() {
  const [selectedEmail, setSelectedEmail] = createSignal<number | null>(null);
  const [generatingPattern, setGeneratingPattern] = createSignal(false);

  const handleGeneratePattern = async (emailId: number) => {
    setGeneratingPattern(true);

    try {
      const response = await fetch("http://localhost:8080/api/extraction/generate-pattern", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email_id: emailId }),
      });

      const result = await response.json();

      // Show result to user
      alert(`Pattern generated! Session ID: ${result.session_id}`);
    } catch (error) {
      console.error("Failed to generate pattern:", error);
      alert("Failed to generate pattern");
    } finally {
      setGeneratingPattern(false);
    }
  };

  return (
    <div>
      {/* Email list */}
      {emails.map(email => (
        <div class="email-item">
          <div>{email.subject}</div>
          <button
            onClick={() => handleGeneratePattern(email.id)}
            disabled={generatingPattern()}
          >
            {generatingPattern() ? "Generating..." : "Generate Pattern"}
          </button>
        </div>
      ))}
    </div>
  );
}
```

## Success Criteria

### Functionality
- [ ] Agent can analyze emails and propose regex patterns
- [ ] test_pattern tool correctly extracts financial data
- [ ] save_pattern tool persists patterns to database
- [ ] Agent iterates until pattern works correctly
- [ ] Session history is persisted in SQLite
- [ ] API endpoint returns pattern ID and extracted data
- [ ] GUI button triggers pattern generation

### Data Integrity
- [ ] Agent sessions stored in database
- [ ] Message history preserved
- [ ] Tool calls tracked with input/output
- [ ] Generated patterns have all required fields
- [ ] Patterns are created as inactive (is_active=false) until user approves

### Agent Behavior
- [ ] Agent follows system prompt instructions
- [ ] Agent tests patterns before saving
- [ ] Agent handles regex errors gracefully
- [ ] Agent completes within 30 iterations or less
- [ ] Agent provides clear explanations of patterns

### Integration
- [ ] dwata-agents compiles without errors
- [ ] nocodo-llm-sdk integrates correctly
- [ ] SqliteAgentStorage works with dwata's database
- [ ] DwataToolExecutor accesses database correctly
- [ ] API handler creates and executes agent

## Notes

### Design Decisions

**Why separate DwataToolExecutor from nocodo-tools?**
- Our tools need access to dwata's database and types
- nocodo-tools is generic and doesn't know about dwata's schema
- DwataToolExecutor provides dwata-specific tool implementations

**Why SqliteAgentStorage instead of InMemoryStorage?**
- Persist agent sessions for debugging
- User can review agent's thought process
- Enable agent resume functionality in future
- Track which patterns came from which agent sessions

**Why patterns created as inactive?**
- User must review and approve patterns before they're used
- Prevents bad patterns from affecting financial data
- Allows user to test pattern on other emails first

**Why limit to 30 iterations?**
- Prevents infinite loops
- Typical pattern generation should complete in 5-10 iterations
- 30 iterations provides safety margin

### Future Enhancements
- [ ] Stream agent progress to GUI via WebSocket
- [ ] Allow user to provide feedback during generation
- [ ] Support multi-email pattern generation
- [ ] Pattern testing on sample of emails before saving
- [ ] Pattern performance metrics (precision, recall)
- [ ] Agent learns from user corrections
- [ ] Suggest improvements to existing patterns
- [ ] Automatic pattern merging/deduplication

### Security Considerations
- Agent has write access to financial_patterns table
- Need to validate pattern names don't conflict
- Rate limit pattern generation API
- Sanitize regex patterns before compilation
- Consider adding user approval step before activation

## References

- nocodo-llm-sdk: `/Users/brainless/Projects/nocodo/nocodo-llm-sdk/`
- nocodo-agents: `/Users/brainless/Projects/nocodo/nocodo-agents/`
- Current extraction: `extractors/src/financial_patterns/`
- Financial types: `shared-types/src/financial.rs`
- Database: `dwata-api/src/database/migrations.rs`
