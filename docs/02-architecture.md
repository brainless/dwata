# Dwata Architecture

## System Overview

Dwata is built as a multi-component system with clear separation of concerns:

```
┌─────────────┐
│     GUI     │  (User Interface)
└──────┬──────┘
       │ HTTP/REST
       │
┌──────▼──────┐
│  dwata-api  │  (Backend API Server)
└──────┬──────┘
       │
       ├─────────┐
       │         │
┌──────▼──┐  ┌──▼─────────┐
│ SQLite  │  │   Agents   │  (AI Agents from nocodo)
│   DB    │  │  Framework │
└─────────┘  └────────────┘
```

## Components

### 1. dwata-api (Backend Server)

The core API server built with Rust and Actix-web.

**Responsibilities:**
- HTTP API endpoints for client communication
- Database initialization and management
- Agent session management
- Request routing and coordination

**Technology:**
- Actix-web 4 for HTTP server
- SQLite via rusqlite for data persistence
- Shared types for API contracts

**Key Files:**
- `dwata-api/src/main.rs`: Server initialization
- `dwata-api/src/database/`: Database operations
- `dwata-api/src/helpers/`: Utility functions

### 2. SQLite Database

Local SQLite database serving as the central data store.

**Purpose:**
- Store agent session data
- Store agent messages and tool calls
- Store user projects and requirements
- Dynamic table creation for domain-specific data
- Store analysis results and insights

**Location:** Local file system (path configured on initialization)

See [Database Schema](./03-database-schema.md) for detailed schema information.

### 3. Agent Framework (nocodo-agents)

External agent library providing specialized AI agents.

**Integration:**
- Linked via Git dependency from nocodo repository
- Provides pre-built agents for common tasks
- Extensible framework for custom agents

**Available Agents:**
- Requirements Gathering Agent
- SQLite Analysis Agent
- Codebase Analysis Agent
- Structured JSON Agent
- Tesseract OCR Agent
- User Clarification Agent

See [Agents Reference](./04-agents-reference.md) for detailed agent documentation.

### 4. GUI (Frontend)

Web-based user interface for interacting with dwata.

**Responsibilities:**
- User authentication and session management
- Visual presentation of projects and data
- Interactive agent conversations
- Data visualization and reporting

**Technology:**
- TypeScript for type safety
- Generated TypeScript types from Rust definitions
- REST API client for dwata-api communication

### 5. Shared Types

Common type definitions used across components.

**Package:** `shared-types`

**Purpose:**
- Ensure type consistency between Rust backend and TypeScript frontend
- Define API request/response structures
- Agent session and message types
- Tool call definitions

**Key Types:**
- `AgentSession`: Session metadata and status
- `AgentMessage`: Messages in agent conversations
- `AgentToolCall`: Tool invocations and results
- `CreateSessionRequest`: Start new agent session
- `SessionResponse`: Detailed session with history

## Data Flow

### Creating a New Project

```
User (GUI)
  │
  ├─► POST /api/sessions (Create session with agent)
  │
dwata-api
  │
  ├─► Create agent session in DB
  │
  ├─► Initialize Requirements Gathering Agent
  │
Agent
  │
  ├─► Ask clarifying questions
  │
  ├─► Store responses in DB
  │
  ├─► Create project tables dynamically
  │
  └─► Return structured project data
```

### Querying Project Data

```
User (GUI)
  │
  ├─► Ask question about project
  │
dwata-api
  │
  ├─► Load project context from DB
  │
  ├─► Initialize Analysis Agent (e.g., SQLite Analysis)
  │
Agent
  │
  ├─► Read project schema
  │
  ├─► Generate SQL queries
  │
  ├─► Execute queries via tools
  │
  ├─► Analyze results
  │
  └─► Return insights/answers
```

## Communication Patterns

### API to Frontend
- REST API with JSON payloads
- Type-safe contracts via shared types
- Error handling with structured error responses

### API to Agents
- Agent factory pattern for creation
- Tool execution via ToolExecutor
- Session-based conversation management
- Message and tool call persistence

### Agents to Database
- Read/write via SQL queries
- Dynamic schema creation
- Transaction support for data integrity

## Extensibility

### Adding New Agents

1. Implement agent in nocodo-agents
2. Register in agent factory
3. Add API endpoints in dwata-api if needed
4. Update GUI to support new agent type

### Adding New Domains

1. Define domain-specific data models
2. Create requirements gathering prompts
3. Implement domain analysis logic
4. Add visualization components in GUI

## Type Generation

Dwata generates TypeScript types from Rust type definitions to ensure API consistency:

```rust
// Rust (shared-types)
#[derive(Serialize, Deserialize)]
pub struct AgentSession { ... }

// Generated TypeScript (GUI)
export interface AgentSession { ... }
```

This ensures compile-time type safety across the entire stack.

## Deployment

### Development
- Local SQLite database
- API server on localhost:8080
- GUI on development server

### Production
- Standalone desktop application
- Embedded database
- Single-user local deployment

## Security Considerations

- **Local-First**: All data stored locally on user's machine
- **No Cloud Dependencies**: Works without internet (except for LLM API calls)
- **API Access**: Agents can access external APIs with user permission
- **File System Access**: Controlled via ToolExecutor permissions
- **Email Access**: User-authorized access for data gathering

## Performance

- **SQLite**: Efficient local database with minimal overhead
- **Rust Backend**: High-performance, low-resource API server
- **Agent Sessions**: Persistent sessions avoid redundant LLM calls
- **Tool Caching**: Repeated tool calls can be cached when appropriate
