# API Reference

## Base URL

```
http://localhost:8080
```

## Type Safety

All API types are defined in `shared-types` and automatically generated for TypeScript. This ensures type consistency between Rust backend and TypeScript frontend.

## Common Types

### AgentSession

```rust
pub struct AgentSession {
    pub id: i64,
    pub agent_name: String,
    pub provider: String,          // "anthropic", "openai", etc.
    pub model: String,              // Model identifier
    pub system_prompt: Option<String>,
    pub user_prompt: String,
    pub config: Option<serde_json::Value>,
    pub status: String,             // "running", "completed", "failed"
    pub started_at: i64,            // Unix timestamp
    pub ended_at: Option<i64>,
    pub result: Option<String>,
    pub error: Option<String>,
}
```

### AgentMessage

```rust
pub struct AgentMessage {
    pub id: i64,
    pub session_id: i64,
    pub role: String,               // "user", "assistant", "system", "tool"
    pub content: String,
    pub created_at: i64,
}
```

### AgentToolCall

```rust
pub struct AgentToolCall {
    pub id: i64,
    pub session_id: i64,
    pub message_id: Option<i64>,
    pub tool_call_id: String,
    pub tool_name: String,
    pub request: serde_json::Value,
    pub response: Option<serde_json::Value>,
    pub status: String,             // "pending", "executing", "completed", "failed"
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub execution_time_ms: Option<i64>,
    pub error_details: Option<String>,
}
```

## Endpoints

### Health Check

Check API and database connectivity.

**Endpoint**: `GET /health`

**Response**:
```json
{
  "status": "healthy",
  "database": "connected"
}
```

**Error Response** (503 Service Unavailable):
```json
{
  "status": "unhealthy",
  "database": "disconnected"
}
```

### Root

Basic API information.

**Endpoint**: `GET /`

**Response**:
```json
{
  "message": "Hello World"
}
```

## Session Management

### Create Session

Start a new agent session.

**Endpoint**: `POST /api/sessions`

**Request Body** (`CreateSessionRequest`):
```json
{
  "agent_name": "requirements_gathering",
  "user_prompt": "I want to track my finances",
  "provider": "anthropic",
  "model": "claude-3-5-sonnet-20241022",
  "config": {
    "max_tokens": 4000,
    "temperature": 0.0
  }
}
```

**Fields**:
- `agent_name` (required): Name of the agent to use
  - `"requirements_gathering"`: Gather user requirements
  - `"sqlite_analysis"`: Analyze database data
  - `"user_clarification"`: Clarify user intent
  - `"codebase_analysis"`: Analyze code structure
  - `"structured_json"`: Generate typed JSON
  - `"tesseract"`: OCR text extraction
- `user_prompt` (required): Initial user request
- `provider` (optional): LLM provider, defaults to configured provider
- `model` (optional): Model name, defaults to configured model
- `config` (optional): Additional configuration

**Response** (`CreateSessionResponse`):
```json
{
  "session_id": 42,
  "agent_name": "requirements_gathering",
  "status": "running"
}
```

**Status Codes**:
- `201 Created`: Session created successfully
- `400 Bad Request`: Invalid request body
- `500 Internal Server Error`: Server error

### Get Session

Retrieve detailed session information including messages and tool calls.

**Endpoint**: `GET /api/sessions/:id`

**Path Parameters**:
- `id`: Session ID

**Response** (`SessionResponse`):
```json
{
  "id": 42,
  "agent_name": "sqlite_analysis",
  "provider": "anthropic",
  "model": "claude-3-5-sonnet-20241022",
  "system_prompt": "You are a SQLite analysis agent...",
  "user_prompt": "What's my spending this month?",
  "config": {"max_tokens": 4000},
  "status": "completed",
  "result": "Your spending this month is $2,345.67",
  "messages": [
    {
      "role": "user",
      "content": "What's my spending this month?",
      "created_at": 1705536000
    },
    {
      "role": "assistant",
      "content": "Let me query your transaction data...",
      "created_at": 1705536001
    }
  ],
  "tool_calls": [
    {
      "tool_name": "sqlite3_reader",
      "request": {
        "query": "SELECT SUM(amount) FROM finance_transactions WHERE ...",
        "limit": 1000
      },
      "response": {
        "rows": [{"total": 2345.67}]
      },
      "status": "completed",
      "execution_time_ms": 15
    }
  ],
  "started_at": 1705536000,
  "ended_at": 1705536010
}
```

**Status Codes**:
- `200 OK`: Session found
- `404 Not Found`: Session does not exist
- `500 Internal Server Error`: Server error

### List Sessions

Get all agent sessions (optionally filtered).

**Endpoint**: `GET /api/sessions`

**Query Parameters**:
- `status` (optional): Filter by status (`running`, `completed`, `failed`)
- `agent_name` (optional): Filter by agent name
- `limit` (optional): Maximum number of results (default: 100)
- `offset` (optional): Pagination offset (default: 0)

**Examples**:
```
GET /api/sessions
GET /api/sessions?status=running
GET /api/sessions?agent_name=sqlite_analysis&limit=20
```

**Response** (`SessionListResponse`):
```json
{
  "sessions": [
    {
      "id": 42,
      "agent_name": "sqlite_analysis",
      "user_prompt": "What's my spending this month?",
      "status": "completed",
      "started_at": 1705536000
    },
    {
      "id": 41,
      "agent_name": "requirements_gathering",
      "user_prompt": "Track my finances",
      "status": "completed",
      "started_at": 1705530000
    }
  ]
}
```

**Status Codes**:
- `200 OK`: Sessions returned (may be empty array)
- `400 Bad Request`: Invalid query parameters
- `500 Internal Server Error`: Server error

### Update Session

Continue or modify an existing session (e.g., add user response).

**Endpoint**: `PATCH /api/sessions/:id`

**Path Parameters**:
- `id`: Session ID

**Request Body**:
```json
{
  "user_message": "My banks are Chase and Bank of America"
}
```

**Response**:
```json
{
  "session_id": 42,
  "status": "running",
  "message": "Processing user response..."
}
```

**Status Codes**:
- `200 OK`: Session updated
- `404 Not Found`: Session does not exist
- `400 Bad Request`: Invalid request
- `500 Internal Server Error`: Server error

### Delete Session

Delete a session and all associated data.

**Endpoint**: `DELETE /api/sessions/:id`

**Path Parameters**:
- `id`: Session ID

**Response**:
```json
{
  "message": "Session deleted successfully"
}
```

**Status Codes**:
- `200 OK`: Session deleted
- `404 Not Found`: Session does not exist
- `500 Internal Server Error`: Server error

## Project Management

### Create Project

Create a new project with dynamic schema.

**Endpoint**: `POST /api/projects`

**Request Body**:
```json
{
  "name": "Personal Finance",
  "domain": "finance",
  "description": "Track my income, expenses, and savings",
  "schema": {
    "tables": [
      {
        "name": "accounts",
        "columns": [
          {"name": "id", "type": "INTEGER PRIMARY KEY"},
          {"name": "name", "type": "TEXT"},
          {"name": "balance", "type": "REAL"}
        ]
      }
    ]
  }
}
```

**Response**:
```json
{
  "project_id": "proj_123",
  "name": "Personal Finance",
  "created_at": 1705536000
}
```

**Status Codes**:
- `201 Created`: Project created
- `400 Bad Request`: Invalid schema
- `500 Internal Server Error`: Server error

### Get Project

Retrieve project details.

**Endpoint**: `GET /api/projects/:id`

**Response**:
```json
{
  "project_id": "proj_123",
  "name": "Personal Finance",
  "domain": "finance",
  "description": "Track my income, expenses, and savings",
  "tables": ["finance_accounts", "finance_transactions"],
  "created_at": 1705536000,
  "updated_at": 1705540000
}
```

**Status Codes**:
- `200 OK`: Project found
- `404 Not Found`: Project does not exist

### List Projects

Get all projects.

**Endpoint**: `GET /api/projects`

**Response**:
```json
{
  "projects": [
    {
      "project_id": "proj_123",
      "name": "Personal Finance",
      "domain": "finance",
      "created_at": 1705536000
    }
  ]
}
```

## Direct Database Queries

### Execute Query

Run a read-only SQL query.

**Endpoint**: `POST /api/query`

**Request Body**:
```json
{
  "query": "SELECT * FROM finance_accounts",
  "limit": 100
}
```

**Response**:
```json
{
  "columns": ["id", "name", "balance"],
  "rows": [
    {"id": 1, "name": "Chase Checking", "balance": 5000.00},
    {"id": 2, "name": "BoA Savings", "balance": 15000.00}
  ],
  "row_count": 2
}
```

**Security Notes**:
- Only SELECT queries allowed
- Query is validated before execution
- No destructive operations (INSERT, UPDATE, DELETE, DROP)

**Status Codes**:
- `200 OK`: Query executed
- `400 Bad Request`: Invalid or forbidden query
- `500 Internal Server Error`: Query execution error

## Error Responses

All error responses follow this format:

```json
{
  "error": "Detailed error message"
}
```

**Common HTTP Status Codes**:
- `400 Bad Request`: Invalid input
- `401 Unauthorized`: Authentication required
- `403 Forbidden`: Insufficient permissions
- `404 Not Found`: Resource does not exist
- `500 Internal Server Error`: Server error
- `503 Service Unavailable`: Service temporarily unavailable

## Rate Limiting

Currently no rate limiting is implemented. For production:
- Recommended: 100 requests per minute per client
- Monitor via server logs

## Authentication

Current version: No authentication required (local-only deployment)

Future versions will support:
- API key authentication
- Session-based auth
- OAuth2 integration

## WebSocket Support (Future)

Planned WebSocket endpoint for real-time agent updates:

```
ws://localhost:8080/api/sessions/:id/stream
```

This will allow:
- Real-time message streaming
- Tool call progress updates
- Live agent status changes

## TypeScript Client Example

```typescript
import type { CreateSessionRequest, SessionResponse } from './types';

class DwataClient {
  private baseUrl = 'http://localhost:8080';

  async createSession(request: CreateSessionRequest): Promise<SessionResponse> {
    const response = await fetch(`${this.baseUrl}/api/sessions`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${await response.text()}`);
    }

    return await response.json();
  }

  async getSession(id: number): Promise<SessionResponse> {
    const response = await fetch(`${this.baseUrl}/api/sessions/${id}`);

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${await response.text()}`);
    }

    return await response.json();
  }
}

// Usage
const client = new DwataClient();

const session = await client.createSession({
  agent_name: 'requirements_gathering',
  user_prompt: 'I want to track my finances',
  provider: 'anthropic',
  model: 'claude-3-5-sonnet-20241022',
});

console.log(`Created session ${session.session_id}`);
```

## Rust Client Example

```rust
use shared_types::{CreateSessionRequest, SessionResponse};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    let request = CreateSessionRequest {
        agent_name: "requirements_gathering".to_string(),
        user_prompt: "I want to track my finances".to_string(),
        provider: Some("anthropic".to_string()),
        model: Some("claude-3-5-sonnet-20241022".to_string()),
        config: None,
    };

    let response = client
        .post("http://localhost:8080/api/sessions")
        .json(&request)
        .send()
        .await?;

    let session: SessionResponse = response.json().await?;

    println!("Created session {}", session.id);

    Ok(())
}
```
