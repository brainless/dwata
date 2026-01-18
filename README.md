# Dwata

> AI-powered personal assistant for finance, work management, life goals, and more.

Dwata is a multi-agent LLM-enabled application that helps you manage various aspects of your personal and professional life using specialized AI agents backed by a local SQLite database.

## Features

- **Multi-Agent System**: Specialized AI agents for different domains (finance, travel, work, etc.)
- **Dynamic Data Management**: SQLite database with on-demand table creation
- **Requirements Gathering**: Conversational agents understand your specific needs
- **Intelligent Analysis**: Query and analyze your data using AI-powered SQL analysis
- **Local-First**: All data stored securely on your machine
- **Type-Safe**: Generated types ensure consistency across Rust and TypeScript
- **Extensible**: Easy to add new agents and domains

## Quick Start

```bash
# Clone the repository
git clone https://github.com/brainless/dwata.git
cd dwata

# Build the project
cargo build --release

# Run the API server
cargo run --release

# Access GUI at http://localhost:3000
```

## Example Use Cases

### Personal Finance
```
"I want to track my personal finances"
→ Agent gathers info about banks, income, expenses
→ Creates finance tables dynamically
→ Answers questions like "What's my spending this month?"
```

### Travel Planning
```
"Plan a 10-day trip to Japan in April"
→ Agent asks about budget, interests, preferences
→ Creates itinerary and booking tables
→ Tracks expenses and provides recommendations
```

### Work Management
```
"Track my work projects and goals"
→ Agent organizes tasks, deadlines, milestones
→ Monitors progress and completion rates
→ Provides productivity insights
```

## Architecture

```
┌─────────────┐
│     GUI     │  (Web Interface)
└──────┬──────┘
       │ REST API
┌──────▼──────┐
│  dwata-api  │  (Rust Backend)
└──────┬──────┘
       │
       ├─────────┐
       │         │
┌──────▼──┐  ┌──▼─────────┐
│ SQLite  │  │   Agents   │
│   DB    │  │ (nocodo)   │
└─────────┘  └────────────┘
```

## Available Agents

- **Requirements Gathering**: Understand user needs and structure data
- **SQLite Analysis**: Query and analyze database information
- **Codebase Analysis**: Analyze code structure and patterns
- **Structured JSON**: Generate type-safe JSON data
- **Tesseract OCR**: Extract text from images
- **User Clarification**: Determine if requests need clarification

More agents coming: Finance Manager, Travel Planner, Work Manager, and more!

## Documentation

Comprehensive documentation is available in the [docs/](./docs/) directory:

- **[Overview](./docs/01-overview.md)** - Introduction to dwata and core concepts
- **[Architecture](./docs/02-architecture.md)** - System design and components
- **[Database Schema](./docs/03-database-schema.md)** - Database structure and tables
- **[Agents Reference](./docs/04-agents-reference.md)** - Available agents and their capabilities
- **[API Reference](./docs/05-api-reference.md)** - REST API endpoints and types
- **[User Guide](./docs/06-user-guide.md)** - Complete usage guide

Start with the [Documentation Index](./docs/README.md).

## Technology Stack

- **Backend**: Rust, Actix-web
- **Database**: SQLite (via rusqlite)
- **LLM Integration**: nocodo-llm-sdk (supports Anthropic, OpenAI)
- **Agents**: nocodo-agents framework
- **Frontend**: TypeScript with generated types
- **Type Safety**: Shared types between Rust and TypeScript

## Project Structure

```
dwata/
├── dwata-api/           # Backend API server
│   ├── src/
│   │   ├── database/    # Database operations
│   │   └── helpers/     # Utility functions
│   └── Cargo.toml
├── shared-types/        # Common type definitions
│   └── src/
│       └── session.rs   # Session and message types
├── gui/                 # Web frontend (excluded from workspace)
├── docs/                # Documentation
│   ├── 01-overview.md
│   ├── 02-architecture.md
│   ├── 03-database-schema.md
│   ├── 04-agents-reference.md
│   ├── 05-api-reference.md
│   └── 06-user-guide.md
└── Cargo.toml           # Workspace configuration
```

## Dependencies

External libraries from nocodo repository:
- `nocodo-llm-sdk`: LLM client abstraction
- `nocodo-agents`: Pre-built AI agents
- `manager-tools`: Tool execution framework

## Configuration

Configure your LLM provider:

```bash
# Set API key
export ANTHROPIC_API_KEY="your-api-key"

# Or create a config file
cp config.example.toml config.toml
# Edit config.toml with your settings
```

## Development

```bash
# Run in development mode
cargo run

# Run tests
cargo test

# Check code
cargo clippy

# Format code
cargo fmt
```

## Contributing

Contributions are welcome! Please see:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

Areas for contribution:
- New agent implementations
- Additional domains (health, learning, etc.)
- GUI improvements
- Documentation enhancements
- Bug fixes and optimizations

## License

MIT License - See LICENSE file for details

## Credits

Built with:
- [Actix-web](https://actix.rs/) - Web framework
- [SQLite](https://www.sqlite.org/) - Database
- [nocodo](https://github.com/brainless/nocodo) - Agent framework
- [Anthropic Claude](https://www.anthropic.com/) - AI capabilities

## Support

- **Issues**: [GitHub Issues](https://github.com/brainless/dwata/issues)
- **Discussions**: [GitHub Discussions](https://github.com/brainless/dwata/discussions)
- **Documentation**: [docs/](./docs/)

---

Made with ❤️ for personal data sovereignty and AI-powered productivity
