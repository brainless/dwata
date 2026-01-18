# Dwata - AI-Powered Personal Assistant

## What is Dwata?

Dwata is a multi-agent LLM-enabled application designed to help you manage various aspects of your personal and professional life, including:

- **Finance Management**: Track income, expenses, savings, and financial health
- **Work Management**: Organize tasks, projects, and professional goals
- **Life Goals**: Plan and monitor personal objectives and milestones
- **Travel Planning**: Research destinations, create itineraries, and manage travel logistics
- And more domains as new agents are added

## Core Philosophy

Dwata uses **objective-focused AI agents** that:

1. **Gather Requirements**: Understand your specific situation through conversational interaction
2. **Create Projects**: Organize information into structured projects stored in a local SQLite database
3. **Manage Data**: Create and manage database tables dynamically based on your needs
4. **Analyze & Answer**: Query data using AI-powered analysis to answer your ad-hoc questions

## Key Features

### Dynamic Data Management
- SQLite database at the core for structured data storage
- Tables created on-demand based on task requirements
- All project details and requirements stored locally

### Intelligent Agents
- Specialized agents for different domains (finance, travel, etc.)
- Requirements gathering agents to understand your situation
- Analysis agents to query and forecast based on your data
- Agents can gather data via APIs, email, or file access

### Flexible Architecture
- API-first design (dwata-api)
- GUI interface for user interaction
- Type-safe communication between components
- Extensible agent system

## Example Workflow: Finance Overview

1. **User Request**: "I want to get an overview of my finances"
2. **Requirements Gathering**:
   - Agent asks about your banks, income sources, credit cards, lifestyle
   - Creates a "Finance Project" in the database
3. **Data Structuring**:
   - Creates tables for income, expenses, accounts, etc.
   - Stores all gathered information
4. **Ongoing Analysis**:
   - Answer questions like "What's my financial health?"
   - Generate forecasts
   - Identify spending patterns
   - Requirements can be refined over time

## Getting Started

For users, see [User Guide](./06-user-guide.md) to learn how to interact with dwata.

For developers looking to understand the system architecture, see [Architecture](./02-architecture.md).

To learn about available agents, see [Agents Reference](./04-agents-reference.md).

To access the API, see [API Reference](./05-api-reference.md).

## Technology Stack

- **Backend**: Rust (dwata-api)
- **Database**: SQLite
- **LLM Integration**: nocodo-llm-sdk (supports multiple providers)
- **Agents**: nocodo-agents framework
- **Frontend**: GUI (web-based interface)
- **Type Safety**: Shared types between Rust and TypeScript
