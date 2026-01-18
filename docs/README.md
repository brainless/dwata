# Dwata Documentation

Welcome to the dwata documentation! This guide will help you understand, use, and extend dwata - an AI-powered personal assistant for finance, work management, life goals, and more.

## Documentation Structure

### For Users

Start here if you want to use dwata:

1. **[Overview](./01-overview.md)** - What is dwata and what can it do?
2. **[User Guide](./06-user-guide.md)** - Complete guide to using dwata effectively

### For Developers

Technical documentation for understanding and extending dwata:

3. **[Architecture](./02-architecture.md)** - System design and component overview
4. **[Database Schema](./03-database-schema.md)** - Database structure and tables
5. **[Agents Reference](./04-agents-reference.md)** - Available AI agents and their capabilities
6. **[API Reference](./05-api-reference.md)** - REST API endpoints and usage

### For AI Agents

If you are an AI agent working with dwata:

- **All agents should query dwata-api** for the most up-to-date information
- API calls provide dynamic agent capabilities and session status
- Generated TypeScript types ensure type safety across the stack
- See [API Reference](./05-api-reference.md) for endpoints

## Quick Start

### Users

1. Read the [Overview](./01-overview.md) to understand dwata
2. Follow the [User Guide](./06-user-guide.md) to get started
3. Create your first project and start tracking!

### Developers

1. Review the [Architecture](./02-architecture.md) to understand the system
2. Check [Database Schema](./03-database-schema.md) for data structure
3. Explore [Agents Reference](./04-agents-reference.md) for agent capabilities
4. Use [API Reference](./05-api-reference.md) to integrate

## Key Features

- **Multi-Agent System**: Specialized AI agents for different domains
- **Dynamic Database**: Tables created on-demand based on your needs
- **Local-First**: All data stored locally on your machine
- **Type-Safe**: Generated types for Rust and TypeScript
- **Extensible**: Easy to add new agents and features

## Technology Stack

- **Backend**: Rust with Actix-web
- **Database**: SQLite
- **LLM Integration**: nocodo-llm-sdk (Anthropic, OpenAI, etc.)
- **Agents**: nocodo-agents framework
- **Frontend**: Web-based GUI with TypeScript

## Use Cases

- **Personal Finance**: Track income, expenses, savings, and financial health
- **Travel Planning**: Research destinations, create itineraries, manage trips
- **Work Management**: Organize tasks, projects, and professional goals
- **Life Goals**: Plan and monitor personal objectives
- **And more**: Extensible to any domain with custom agents

## Document Details

| Document | Audience | Description |
|----------|----------|-------------|
| [01-overview.md](./01-overview.md) | Everyone | High-level introduction to dwata |
| [02-architecture.md](./02-architecture.md) | Developers | System architecture and components |
| [03-database-schema.md](./03-database-schema.md) | Developers | Database tables and schema |
| [04-agents-reference.md](./04-agents-reference.md) | Developers, Agents | AI agent capabilities and tools |
| [05-api-reference.md](./05-api-reference.md) | Developers, Agents | REST API endpoints and types |
| [06-user-guide.md](./06-user-guide.md) | Users | Complete usage guide |

## Getting Help

- **GitHub Issues**: Report bugs or request features
- **Discussions**: Share use cases and ask questions
- **Documentation**: You're reading it!

## Contributing

Want to contribute to dwata or its documentation?

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

### Documentation Contributions

- Fix typos or unclear explanations
- Add examples and use cases
- Improve code samples
- Update outdated information

All contributions are welcome!

## Version

These docs are for dwata version 0.1.0.

Last updated: 2026-01-18

## License

MIT License - See LICENSE file for details
