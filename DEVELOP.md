# Developer Guide

Dwata is in an interim backend-first restart. The supported workspace is Rust
only: `dwata-api`, `dwata-agents`, and `shared-types`. A native Rust UI is
planned for a later epic; the former Tauri/SolidJS application has been
removed and is not a development target.

## Prerequisites

- **Rust**: required for every supported crate.
- **SQLite CLI**: optional, for querying local data directly.
- **Ollama**: optional, for local LLM inference.

Node.js, npm, TypeScript generation, Tauri, and GUI tooling are not required
for normal backend development.

## Workspace structure

The root `Cargo.toml` defines these workspace members:

```toml
members = ["dwata-agents", "dwata-api", "shared-types"]
```

### `dwata-api`

`dwata-api/` is the Actix-web JSON-over-HTTP backend. It owns HTTP routing,
SQLite initialization and migrations, integrations, background jobs, and
search.

Key areas:

- `src/main.rs`: server setup and route registration.
- `src/config.rs`: configuration discovery and defaults.
- `src/database/`: queries, data models, and migrations.
- `src/handlers/`: HTTP adapters for API endpoints.
- `src/helpers/`: database, OAuth, keyring, and utility support.
- `src/jobs/`: background work, including email downloads.
- `src/search/`: Tantivy indexing and search.

The long-term direction is to make handlers thin adapters over
transport-neutral application services so both the HTTP server and future
native UI can share one business implementation. That extraction is not part
of this cleanup.

### `dwata-agents`

`dwata-agents/` contains the Rust extraction agents. The email knowledge-graph
extractor is under `dwata-agents/src/kg_email_extractor/`.

See [knowledge-graph extraction](docs/06-knowledge-graph-extraction.md) for
the pass architecture, gating, persistence, and search flow. The canonical
financial extraction direction is in
[type-driven financial extraction](docs/03-type-driven-financial-extraction.md).

### `shared-types`

`shared-types/` contains the Rust types shared by the API and agents,
including API request/response and domain concepts. Keep a type here only
when it has a Rust consumer; it no longer generates TypeScript declarations.

Key modules include `credential.rs`, `download.rs`, `email.rs`, `event.rs`,
`extraction.rs`, `financial.rs`, `session.rs`, and `settings.rs`.

## Running and verifying the API

Start the API from the workspace root:

```bash
cargo run -p dwata-api
```

Pass `--log-file-path` to duplicate logs to a file:

```bash
cargo run -p dwata-api -- --log-file-path /path/to/dwata-api.log
```

The default address is `127.0.0.1:8080`. Once running, verify it with:

```bash
curl http://127.0.0.1:8080/api/health
```

For routine workspace checks:

```bash
cargo fmt --check
cargo test --workspace
cargo build --workspace
```

## Configuration management

`ApiConfig` searches for configuration in this order:

1. `config.toml` in the current directory.
2. `../config.toml`.
3. The OS configuration directory at `dwata/config.toml`.
4. Locations near the executable.

If no configuration exists, the API creates a default OS-level configuration:

- macOS: `~/Library/Application Support/dwata/config.toml`
- Linux: `~/.config/dwata/config.toml`
- Windows: `%APPDATA%\dwata\config.toml`

Use [`config.example.toml`](config.example.toml) as a documented starting
point. Important sections are:

```toml
[server]
host = "127.0.0.1"
port = 8080

[google_oauth]
client_id = "YOUR_CLIENT_ID.apps.googleusercontent.com"
client_secret = "YOUR_CLIENT_SECRET"

[email_downloads]
auto_start = false
```

The Google OAuth callback URI is constructed as
`http://<server.host>:<server.port>/api/oauth/google/callback`. Register the
exact value with your Google OAuth application. For a browser-based local
callback, use `localhost` as the server host when required by the OAuth client
configuration.

`[cors]` controls browser origins when you intentionally place a browser
client in front of the API. It is not needed for direct backend development.
`[ai_provider_api_keys]` configures OpenAI or Gemini, and `[selected_llm]`
selects the extraction provider/model. Ollama is the default local provider.

## Database storage

The API uses SQLite. By default the database is at the OS local-data directory
under `dwata/db.sqlite`:

- macOS: `~/Library/Application Support/dwata/db.sqlite`
- Linux: `~/.local/share/dwata/db.sqlite`
- Windows: `%LOCALAPPDATA%\dwata\db.sqlite`

Database initialization creates parent directories and runs migrations before
the server accepts requests. Add migrations in
`dwata-api/src/database/migrations.rs`; do not change persisted data merely as
part of API/UI cleanup.

With the SQLite CLI installed, inspect a local database directly:

```bash
sqlite3 ~/Library/Application\ Support/dwata/db.sqlite
```

```sql
SELECT * FROM credentials_metadata;
SELECT * FROM download_jobs;
SELECT * FROM emails;
```

## Credentials and local security

Credential metadata lives in SQLite; passwords and sensitive tokens live in
the operating system keychain. Dwata uses a single `dwata-master` keychain
entry containing encrypted credential JSON, then caches credentials in memory
to avoid repeated keychain prompts.

On macOS, select **Always Allow** for the initial keychain request if you want
the API process to access that entry without future prompts. Cached values are
memory-only and expire after the configured TTL (one hour by default).

## Development workflow

When changing an endpoint:

1. Add or update Rust request/response/domain types in `shared-types` when
   they are shared by the API or agents.
2. Implement the handler in `dwata-api/src/handlers/` and register its route
   in `dwata-api/src/main.rs`.
3. Add or update database migrations when persistent schema changes are
   required.
4. Run formatting and the relevant workspace tests/build.

There is currently no generated public API schema or client. Keep API changes
intentional: the planned external/server protocol is versioned JSON over HTTPS,
while the future native UI will use transport-neutral application services.
