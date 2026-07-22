# Dwata

Dwata is a local-first Rust backend for connecting email accounts, storing mail
in SQLite, and using LLM agents to extract financial data and knowledge-graph
entities. Email data stays on the machine running `dwata-api` unless you choose
an external AI provider.

> [!WARNING]
> Dwata is an active restart. The native cross-platform UI is planned but is
> not shipped yet. This repository currently provides the backend API and Rust
> agent components; it does not provide a downloadable desktop application.

## What is available now

- `dwata-api`: an Actix-web JSON-over-HTTP API with local SQLite storage,
  email/OAuth integration, search, download jobs, and extraction endpoints.
- `dwata-agents`: Rust agents for financial and knowledge-graph extraction.
- `shared-types`: Rust domain and API types shared by the backend and agents.

The API is intended to be the server boundary for the restart. A future native
Rust UI will compose with the same application capabilities; the former web
and Tauri desktop applications are no longer part of the project.

## Run the backend

Install the Rust toolchain, then start the API from the repository root:

```bash
cargo run -p dwata-api
```

By default it listens on `127.0.0.1:8080`. Verify that the service and SQLite
connection are available:

```bash
curl http://127.0.0.1:8080/api/health
```

The first start creates a configuration file in the OS configuration directory
when neither `config.toml` nor `../config.toml` exists:

- macOS: `~/Library/Application Support/dwata/config.toml`
- Linux: `~/.config/dwata/config.toml`
- Windows: `%APPDATA%\dwata\config.toml`

Copy and adapt [`config.example.toml`](config.example.toml) if you need to set
the listen address, OAuth client credentials, AI-provider keys, or search
index path. See the [developer guide](DEVELOP.md) for configuration, database,
and agent-development details.

## Gmail and AI providers

Gmail OAuth uses a Google OAuth client configured under `[google_oauth]` in
`config.toml`. The redirect URI is derived from `[server]` as
`http://<host>:<port>/api/oauth/google/callback`; register that exact URI with
your OAuth application.

For local inference, install [Ollama](https://ollama.com) and pull the default
model:

```bash
ollama pull qwen3.5:2b
```

OpenAI and Gemini keys can instead be configured in
`[ai_provider_api_keys]`. Using an external provider sends the applicable
extraction prompts and content to that provider.

## Privacy and storage

- SQLite data is local. Its default path is OS-specific; see
  [DEVELOP.md](DEVELOP.md#database-storage).
- Credentials are stored as SQLite metadata plus secrets in the OS keychain.
- Ollama inference is local; third-party AI providers are not.

## Development

Backend development needs Rust. SQLite's CLI is optional for inspecting the
database. Node.js, TypeScript, Tauri, and a web GUI are not required or
included in the current workspace.

Useful commands:

```bash
cargo fmt --check
cargo test --workspace
cargo build --workspace
```

## Support

- [Issues](https://github.com/brainless/dwata/issues)
- [Discussions](https://github.com/brainless/dwata/discussions)
- [Developer guide](DEVELOP.md)

## License

GPL v3 — see [LICENSE](LICENSE).
