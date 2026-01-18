# Agents Reference

## Overview

Dwata uses AI agents from the `nocodo-agents` library. Each agent is specialized for specific tasks and has access to different tools.

> **Note for Users**: For the most up-to-date information about available agents and their capabilities, make API calls to dwata-api. The API can provide dynamic agent information based on the current system state.

> **Note for Agents**: Query dwata-api endpoints to get current agent capabilities, session status, and tool availability.

## Agent Architecture

All agents implement the `Agent` trait:

```rust
pub trait Agent: Send + Sync {
    fn objective(&self) -> &str;              // Agent's purpose
    fn system_prompt(&self) -> String;        // LLM instructions
    fn pre_conditions(&self) -> Option<Vec<String>>;  // Requirements
    fn tools(&self) -> Vec<AgentTool>;        // Available tools
    async fn execute(&self, user_prompt: &str, session_id: i64) -> Result<String>;
}
```

## Available Agents

### 1. Requirements Gathering Agent

**Objective**: Understand user's situation and requirements through conversational interaction

**Use Cases**:
- Initial project setup
- Gathering information about finance, work, travel, etc.
- Clarifying ambiguous requests
- Building user context

**Tools**:
- `ask_user`: Ask clarifying questions
- `sqlite3_reader`: Read existing project data
- `write_file`: Store gathered information

**Workflow**:
1. Receive initial user request
2. Ask targeted questions to understand context
3. Create project structure in database
4. Generate dynamic tables for project data
5. Store all gathered information

**Example Interaction**:
```
User: "I want to track my finances"

Agent: "I'll help you set up finance tracking. Let me ask a few questions:
       1. What banks do you use?
       2. Do you have credit cards?
       3. What are your income sources?
       4. Do you freelance or have a regular salary?"

[After gathering responses]

Agent: "I've created a Finance Project with tables for:
       - Accounts (checking, savings, credit cards)
       - Income sources
       - Expense categories
       - Transactions"
```

### 2. SQLite Analysis Agent

**Objective**: Query and analyze data stored in SQLite database

**Use Cases**:
- Answer questions about project data
- Generate reports and insights
- Perform forecasting and trend analysis
- Aggregate and summarize information

**Tools**:
- `sqlite3_reader`: Execute SELECT queries
- `read_file`: Access schema information
- `grep`: Search for patterns in results

**Workflow**:
1. Receive user question
2. Examine database schema
3. Generate appropriate SQL queries
4. Execute queries and analyze results
5. Format insights for user

**Example Queries**:
```sql
-- Financial health check
SELECT
    SUM(CASE WHEN account_type = 'checking' THEN balance ELSE 0 END) as checking_total,
    SUM(CASE WHEN account_type = 'savings' THEN balance ELSE 0 END) as savings_total,
    SUM(CASE WHEN account_type = 'credit_card' THEN -balance ELSE 0 END) as debt_total
FROM finance_accounts;

-- Monthly spending by category
SELECT
    category,
    SUM(amount) as total,
    COUNT(*) as transaction_count
FROM finance_transactions
WHERE date >= strftime('%s', 'now', '-1 month')
GROUP BY category
ORDER BY total DESC;
```

### 3. Codebase Analysis Agent

**Objective**: Analyze code structure and identify architectural patterns

**Use Cases**:
- Understanding project structure
- Identifying patterns and anti-patterns
- Documenting code architecture
- Suggesting improvements

**Tools**:
- `list_files`: Browse directory structure
- `read_file`: Read source files
- `grep`: Search for patterns
- `bash`: Run analysis commands

**Example Analysis**:
```
Project Structure:
- Frontend: React with TypeScript
- Backend: Rust with Actix-web
- Database: SQLite with migration system
- Key patterns: Repository pattern, Factory pattern
- API: REST with JSON
```

### 4. Structured JSON Agent

**Objective**: Generate structured JSON conforming to TypeScript types

**Use Cases**:
- Creating type-safe configuration
- Generating test data
- Converting unstructured data to typed format
- API payload generation

**Tools**:
- `read_file`: Read type definitions
- `write_file`: Write generated JSON
- `sqlite3_reader`: Query data for conversion

**Configuration**:
```rust
StructuredJsonAgentConfig {
    type_names: vec!["PMProject", "Workflow", "WorkflowStep"],
    domain_description: "Project management entities",
}
```

**Example Output**:
```json
{
  "project": {
    "id": "proj_123",
    "name": "Q1 Finance Review",
    "status": "in_progress",
    "created_at": 1705536000
  },
  "workflow": {
    "steps": [
      {"name": "Gather Data", "status": "completed"},
      {"name": "Analyze", "status": "in_progress"}
    ]
  }
}
```

### 5. Tesseract OCR Agent

**Objective**: Extract text from images using OCR

**Use Cases**:
- Reading receipts and invoices
- Extracting data from screenshots
- Processing scanned documents
- Converting images to searchable text

**Tools**:
- `read_file`: Read image files
- `bash`: Run tesseract command
- `write_file`: Save extracted text

**Supported Formats**:
- PNG, JPG, JPEG
- TIFF
- PDF (via image conversion)

**Example**:
```
Input: receipt.jpg
Output:
  Store: Whole Foods
  Date: 2024-01-15
  Items:
    - Organic Bananas: $3.99
    - Almond Milk: $5.49
  Total: $9.48
```

### 6. User Clarification Agent

**Objective**: Determine if a user request needs clarification before proceeding

**Use Cases**:
- Validating user intent
- Identifying missing information
- Suggesting next steps
- Routing to appropriate specialist agent

**Tools**:
- `ask_user`: Request clarification
- `sqlite3_reader`: Check existing context

**Decision Process**:
1. Analyze user request
2. Check if information is sufficient
3. Identify ambiguities or missing details
4. Either:
   - Request clarification, or
   - Proceed with clear intent

## Tool Reference

Agents have access to various tools:

| Tool | Description | Available To |
|------|-------------|--------------|
| `list_files` | Browse directory structure | Codebase Analysis |
| `read_file` | Read file contents | Most agents |
| `write_file` | Write/create files | Structured JSON, Requirements |
| `grep` | Search for patterns | Analysis agents |
| `apply_patch` | Apply code patches | Codebase agents |
| `bash` | Execute shell commands | Advanced agents |
| `ask_user` | Interactive questions | Requirements, Clarification |
| `sqlite3_reader` | Query database | All data-related agents |

## Agent Selection Guide

Choose the right agent for your task:

| Task | Recommended Agent |
|------|------------------|
| Starting a new project | Requirements Gathering |
| Understanding finances | Requirements Gathering → SQLite Analysis |
| Answering data questions | SQLite Analysis |
| Planning travel | Requirements Gathering → SQLite Analysis |
| Processing receipts | Tesseract OCR → Requirements Gathering |
| Unclear request | User Clarification |
| Code analysis | Codebase Analysis |
| Type-safe data generation | Structured JSON |

## Creating Custom Agents

To create a new agent for dwata:

1. **Define the Agent**:
```rust
pub struct FinanceManagerAgent {
    llm_client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,
}
```

2. **Implement the Agent Trait**:
```rust
#[async_trait]
impl Agent for FinanceManagerAgent {
    fn objective(&self) -> &str {
        "Manage personal finances and provide financial insights"
    }

    fn system_prompt(&self) -> String {
        "You are a finance management assistant...".to_string()
    }

    fn tools(&self) -> Vec<AgentTool> {
        vec![
            AgentTool::Sqlite3Reader,
            AgentTool::AskUser,
            AgentTool::Bash,  // For API calls to banks
        ]
    }

    async fn execute(&self, user_prompt: &str, session_id: i64) -> Result<String> {
        // Implementation
    }
}
```

3. **Register in Factory**:
```rust
// Add to AgentType enum
pub enum AgentType {
    FinanceManager,
    // ... existing types
}

// Add factory method
impl AgentFactory {
    pub fn create_finance_manager_agent(&self) -> FinanceManagerAgent {
        FinanceManagerAgent::new(
            self.llm_client.clone(),
            self.database.clone(),
            self.tool_executor.clone(),
        )
    }
}
```

4. **Add API Endpoint** (if needed):
```rust
#[post("/api/finance/analyze")]
async fn analyze_finances(
    req: web::Json<AnalyzeRequest>,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse> {
    // Use FinanceManagerAgent
}
```

## Agent Configuration

Agents can be configured via the session creation request:

```json
{
  "agent_name": "sqlite_analysis",
  "user_prompt": "What's my spending this month?",
  "provider": "anthropic",
  "model": "claude-3-5-sonnet-20241022",
  "config": {
    "max_tokens": 4000,
    "temperature": 0.0,
    "db_path": "/path/to/project.db"
  }
}
```

## Agent Best Practices

### For Users

1. **Be Specific**: Provide clear context in your initial request
2. **Answer Fully**: Give complete answers to agent questions
3. **Review Results**: Check agent outputs before acting on them
4. **Iterate**: Refine requirements over time as needs change

### For Developers

1. **Clear Objectives**: Define precise agent purposes
2. **Appropriate Tools**: Only grant necessary tool access
3. **Error Handling**: Gracefully handle tool failures
4. **User Feedback**: Provide clear progress updates
5. **Session Persistence**: Store all interactions for context

## Future Agents

Planned agents for dwata:

- **Travel Planner**: Itinerary creation and trip management
- **Work Manager**: Task and project organization
- **Goal Tracker**: Personal objectives monitoring
- **Health Tracker**: Wellness and fitness data
- **Email Analyzer**: Email parsing and organization
- **Document Summarizer**: Long-form content analysis
- **API Integrator**: Connect to external services

Each will follow the same agent framework and integrate with dwata's database.
