# Database Schema

## Overview

Dwata uses SQLite as its primary data store. The database has two categories of tables:

1. **Core System Tables**: Fixed schema for agent management and execution tracking
2. **Dynamic Project Tables**: Created on-demand based on user projects and domains

## Core System Tables

### agent_sessions

Stores metadata about agent execution sessions.

```sql
CREATE TABLE agent_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_name TEXT NOT NULL,
    provider TEXT NOT NULL,           -- LLM provider (e.g., "anthropic", "openai")
    model TEXT NOT NULL,               -- Model name (e.g., "claude-3-5-sonnet-20241022")
    system_prompt TEXT,                -- Agent's system instructions
    user_prompt TEXT NOT NULL,         -- Initial user request
    config TEXT,                       -- JSON configuration
    status TEXT NOT NULL DEFAULT 'running'
        CHECK (status IN ('running', 'completed', 'failed')),
    started_at INTEGER NOT NULL,       -- Unix timestamp
    ended_at INTEGER,                  -- Unix timestamp
    result TEXT,                       -- Final result/summary
    error TEXT                         -- Error message if failed
)
```

**Indexes:**
- Primary key on `id`

**Status Values:**
- `running`: Agent is currently executing
- `completed`: Agent finished successfully
- `failed`: Agent encountered an error

### agent_messages

Stores conversation messages within agent sessions.

```sql
CREATE TABLE agent_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,       -- Unix timestamp
    FOREIGN KEY (session_id) REFERENCES agent_sessions (id) ON DELETE CASCADE
)
```

**Indexes:**
- `idx_agent_messages_session_created` on `(session_id, created_at)`

**Role Values:**
- `user`: Messages from the user
- `assistant`: Messages from the AI agent
- `system`: System-generated messages
- `tool`: Tool execution results

### agent_tool_calls

Tracks tool/function calls made by agents during execution.

```sql
CREATE TABLE agent_tool_calls (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    message_id INTEGER,                -- Associated message (can be NULL)
    tool_call_id TEXT NOT NULL,        -- Unique identifier for this tool call
    tool_name TEXT NOT NULL,           -- Name of the tool (e.g., "sqlite3_reader", "bash")
    request TEXT NOT NULL,             -- JSON: tool parameters
    response TEXT,                     -- JSON: tool execution result
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'executing', 'completed', 'failed')),
    created_at INTEGER NOT NULL,       -- Unix timestamp
    completed_at INTEGER,              -- Unix timestamp
    execution_time_ms INTEGER,         -- Execution duration in milliseconds
    error_details TEXT,                -- Error information if failed
    FOREIGN KEY (session_id) REFERENCES agent_sessions (id) ON DELETE CASCADE,
    FOREIGN KEY (message_id) REFERENCES agent_messages (id) ON DELETE SET NULL
)
```

**Indexes:**
- `idx_agent_tool_calls_session` on `session_id`
- `idx_agent_tool_calls_status` on `(session_id, status)`

**Status Values:**
- `pending`: Tool call queued but not started
- `executing`: Tool is currently running
- `completed`: Tool executed successfully
- `failed`: Tool execution failed

## Database Configuration

### Foreign Keys

Foreign key constraints are enabled:

```sql
PRAGMA foreign_keys = ON;
```

This ensures referential integrity:
- Deleting a session cascades to delete all messages and tool calls
- Deleting a message sets `message_id` to NULL in related tool calls

### Performance Optimizations

Indexes are created on frequently queried columns:

- Messages ordered by session and creation time
- Tool calls filtered by session and status

## Dynamic Project Tables

Projects can create their own tables as needed. These tables are not part of the core schema and are created dynamically by agents.

### Examples

**Finance Project:**
```sql
-- Created by Finance Requirements Gathering Agent
CREATE TABLE finance_accounts (
    id INTEGER PRIMARY KEY,
    account_name TEXT,
    account_type TEXT,  -- 'checking', 'savings', 'credit_card'
    institution TEXT,
    balance REAL,
    currency TEXT
);

CREATE TABLE finance_transactions (
    id INTEGER PRIMARY KEY,
    account_id INTEGER,
    date INTEGER,
    description TEXT,
    amount REAL,
    category TEXT,
    FOREIGN KEY (account_id) REFERENCES finance_accounts(id)
);
```

**Travel Project:**
```sql
-- Created by Travel Planning Agent
CREATE TABLE travel_destinations (
    id INTEGER PRIMARY KEY,
    name TEXT,
    country TEXT,
    arrival_date INTEGER,
    departure_date INTEGER,
    budget REAL
);

CREATE TABLE travel_activities (
    id INTEGER PRIMARY KEY,
    destination_id INTEGER,
    activity_name TEXT,
    date INTEGER,
    cost REAL,
    booking_status TEXT,
    FOREIGN KEY (destination_id) REFERENCES travel_destinations(id)
);
```

### Naming Convention

Dynamic tables should follow these conventions:
- Prefix with domain: `finance_*`, `travel_*`, `work_*`
- Use lowercase with underscores
- Include primary keys
- Use foreign keys for relationships
- Include relevant indexes

## Querying the Database

### For Agents

Agents use the `sqlite3_reader` tool to query the database:

```json
{
  "tool": "sqlite3_reader",
  "parameters": {
    "query": "SELECT * FROM agent_sessions WHERE status = 'running'",
    "limit": 100
  }
}
```

### For API

The API can query directly via rusqlite:

```rust
use rusqlite::Connection;

let conn = database.connection.lock()?;
let mut stmt = conn.prepare("SELECT * FROM agent_sessions WHERE id = ?")?;
let session = stmt.query_row([session_id], |row| {
    Ok(AgentSession {
        id: row.get(0)?,
        agent_name: row.get(1)?,
        // ... other fields
    })
})?;
```

## Data Lifecycle

### Session Creation
1. Create entry in `agent_sessions` with status `running`
2. Store initial user message in `agent_messages`
3. Agent begins execution

### During Execution
1. Agent messages appended to `agent_messages`
2. Tool calls tracked in `agent_tool_calls`
3. Dynamic tables created/modified as needed

### Session Completion
1. Update `agent_sessions`:
   - Set `status` to `completed` or `failed`
   - Set `ended_at` timestamp
   - Store `result` or `error`
2. All messages and tool calls remain for audit trail

### Data Retention

- Sessions are kept indefinitely by default
- Users can delete old sessions via API
- Cascade deletes clean up related messages and tool calls
- Dynamic project tables persist independently

## Migrations

Database migrations are run on startup:

```rust
// dwata-api/src/database/migrations.rs
pub fn run_migrations(conn: &Connection) -> anyhow::Result<()> {
    // Enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // Create tables with IF NOT EXISTS
    // ...

    // Create indexes
    // ...

    Ok(())
}
```

This ensures the database schema is always up-to-date.

## Backup and Export

### Backup
```bash
# SQLite database is a single file
cp /path/to/dwata.db /path/to/backup/dwata-backup-$(date +%Y%m%d).db
```

### Export to SQL
```bash
sqlite3 dwata.db .dump > dwata-export.sql
```

### Export Specific Tables
```bash
sqlite3 dwata.db "SELECT * FROM agent_sessions" -header -csv > sessions.csv
```

## Future Enhancements

Potential schema additions:

- **Projects Table**: Explicit project tracking
- **Users Table**: Multi-user support
- **Tags/Labels**: Categorization system
- **Attachments**: File references
- **Relationships**: Cross-project links
- **Audit Log**: Detailed change tracking
