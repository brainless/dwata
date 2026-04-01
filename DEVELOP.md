# Developer Guide

## Financial Extraction North Star

- See `docs/03-type-driven-financial-extraction.md` for the canonical type-driven extraction approach and future user-in-the-loop direction.

## Prerequisites

- **Rust**: Required for building the API server (`dwata-api`) and shared types
- **Node.js and npm**: Required for running the GUI and Tauri desktop app
- **SQLite CLI**: Optional, if you want to run SQL queries directly against the database

## Project Structure

The dwata project is organized as a Cargo workspace plus frontend and desktop app packages:

### Workspace Configuration

The root `Cargo.toml` defines the workspace members:
```toml
members = [
    "dwata-agents",
    "dwata-api",
    "shared-types",
]
exclude = [
    "gui"
]
```

### 1. `dwata-agents` - KG Extraction Agents

Location: `/dwata-agents/`

Main email KG extractor: `dwata-agents/src/kg_email_extractor/`

See `docs/06-knowledge-graph-extraction.md` for the pass architecture, gating, and persistence/search flow.

### 2. `dwata-api` - Backend API Server

Location: `/dwata-api/`

The API server is built with Actix-web and uses SQLite for data storage.

#### Key Dependencies

From `dwata-api/Cargo.toml`:
```toml
actix-web.workspace = true
rusqlite = { version = "0.31", features = ["bundled"] }
shared-types = { path = "../shared-types" }
config = { version = "0.14", default-features = false, features = ["toml"] }
dirs = "5.0"
```

#### Source Structure

- `src/main.rs` - Entry point, HTTP server setup
- `src/config.rs` - Configuration management
- `src/database/` - Database models, queries, and migrations
  - `mod.rs` - Database connection and session management
  - `credentials.rs` - Credential storage
  - `downloads.rs` - Download job management
  - `emails.rs` - Email storage
  - `migrations.rs` - Database schema migrations
- `src/handlers/` - HTTP request handlers
  - `credentials.rs` - Credential CRUD endpoints
  - `downloads.rs` - Download job endpoints
  - `oauth.rs` - OAuth flow handlers
  - `settings.rs` - Settings endpoints
- `src/helpers/` - Utility functions
  - `database.rs` - Database path and initialization
  - `google_oauth.rs` - Google OAuth client
  - `oauth_state.rs` - OAuth state management
  - `token_cache.rs` - Token caching
- `src/integrations/` - External service integrations
- `src/jobs/` - Background job management
  - `download_manager.rs` - Manages download jobs

### 3. `shared-types` - Type Definitions

Location: `/shared-types/`

This crate contains all the shared type definitions used by both the API server and the GUI.

#### Key Dependencies

From `shared-types/Cargo.toml`:
```toml
serde.workspace = true
ts-rs = "8.0"
```

#### Structure

- `src/lib.rs` - Main module that re-exports all types
- `src/credential.rs` - Credential-related types
- `src/download.rs` - Download job types
- `src/email.rs` - Email types
- `src/event.rs` - Event types
- `src/project.rs` - Project types
- `src/session.rs` - Agent session types
- `src/settings.rs` - Settings types
- `src/task.rs` - Task types
- `src/extraction.rs` - Data extraction types

#### TypeScript Type Generation

The crate includes a binary at `src/bin/generate_api_types.rs` that uses `ts-rs` to generate TypeScript type definitions:

```rust
let output_dir = Path::new("../gui/src/api-types");
fs::create_dir_all(output_dir)?;
let output_path = output_dir.join("types.ts");
```

To generate types:
```bash
cargo run --bin generate_api_types
```

### 4. `gui` - Frontend Application

Location: `/gui/`

The GUI is built with SolidJS and Vite.

#### Key Dependencies

From `gui/package.json`:
```json
{
  "dependencies": {
    "@solidjs/router": "^0.15.1",
    "solid-js": "^1.9.5",
    "daisyui": "^5.5.14"
  }
}
```

#### Source Structure

- `src/index.tsx` - Application entry point
- `src/App.tsx` - Root component
- `src/api-types/` - Generated TypeScript types from shared-types
- `src/components/` - Reusable UI components
- `src/config/` - Frontend configuration
- `src/pages/` - Page components
  - `settings/` - Settings page

## Configuration Management

### API Server Configuration

The API server reads its configuration from the OS user's config directory + `dwata`.

From `dwata-api/src/config.rs`:

```rust
pub fn get_config_path() -> PathBuf {
    if let Some(config_dir) = dirs::config_dir() {
        config_dir.join("dwata").join("config.toml")
    } else {
        PathBuf::from("config.toml")
    }
}
```

Platform-specific config paths:
- **macOS**: `~/Library/Application Support/dwata/config.toml`
- **Linux**: `~/.config/dwata/config.toml`
- **Windows**: `%APPDATA%\dwata\config.toml`

The configuration is loaded in `src/main.rs`:

```rust
// Load config
let (config, _) = config::ApiConfig::load().expect("Failed to load config");
```

The `ApiConfig::load()` method:
1. Gets the config path using `get_config_path()`
2. Creates the config directory if it doesn't exist
3. Creates a default config file if one doesn't exist
4. Loads and deserializes the TOML configuration

Default configuration structure (from `config.rs`):
```toml
[api_keys]
# gemini_api_key = "your-gemini-key"

[cors]
allowed_origins = ["http://localhost:3030"]

[server]
host = "127.0.0.1"
port = 8080

[google_oauth]
# client_id = "YOUR_CLIENT_ID.apps.googleusercontent.com"
# client_secret = "YOUR_CLIENT_SECRET"
# redirect_uri = "http://localhost:8080/api/oauth/google/callback"

[downloads]
# When false, the API will not auto-start download jobs on startup.
auto_start = false
```

**Desktop OAuth note:** Google Desktop OAuth is sensitive to the exact host in the redirect URI. Use `server.host = "localhost"` (not `127.0.0.1`) to avoid token exchange failures. We support bring-your-own Google OAuth apps; if you set `client_id`/`client_secret` in the config, those are always used.

**Release defaults:** Release builds can embed a default Google OAuth `client_id`/`client_secret` at compile time. `scripts/build-production.sh` will read them from your local `config.toml` (or from `DWATA_DEFAULT_GOOGLE_CLIENT_ID` / `DWATA_DEFAULT_GOOGLE_CLIENT_SECRET`) and compile them in. The runtime config still overrides these defaults when set.

## Database Storage

### Database Location

The API server uses SQLite for storage. The database path is determined by the OS.

From `dwata-api/src/helpers/database.rs`:

```rust
/// Platform-specific paths
///
/// - **macOS**: `~/Library/Application Support/dwata/db.sqlite`
/// - **Linux**: `~/.local/share/dwata/db.sqlite`
/// - **Windows**: `%LOCALAPPDATA%\dwata\db.sqlite`
pub fn get_db_path() -> anyhow::Result<PathBuf> {
    let data_dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine local data directory"))?;

    let db_path = data_dir.join("dwata").join("db.sqlite");

    Ok(db_path)
}
```

### Database Initialization

From `dwata-api/src/database/mod.rs`:

```rust
pub fn new(db_path: &PathBuf) -> anyhow::Result<Self> {
    // Ensure directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Create sync connection first and run migrations
    let sync_conn = Connection::open(db_path)?;
    let sync_mutex = Arc::new(Mutex::new(sync_conn));

    // Run migrations on sync connection before opening async connection
    {
        let conn = sync_mutex.lock().unwrap();
        migrations::run_migrations(&conn)?;
    }

    // Now open async connection
    let async_conn = Connection::open(db_path)?;

    let database = Database {
        connection: sync_mutex,
        async_connection: Arc::new(TokioMutex::new(async_conn)),
    };

    Ok(database)
}
```

The database is initialized in `src/main.rs`:

```rust
// Initialize database
let db = helpers::database::initialize_database().expect("Failed to initialize database");

println!(
    "Database initialized at: {:?}",
    helpers::database::get_db_path().unwrap()
);
```

## Credential Security and Caching

### OS Keychain Integration

dwata uses the OS native keychain for secure credential storage:
- **macOS**: Keychain Access
- **Linux**: Secret Service (libsecret/gnome-keyring)
- **Windows**: Credential Manager

Credentials are stored in the SQLite database as metadata only (without passwords). Passwords and sensitive tokens are stored separately in the OS keychain using the `keyring` crate.

### Master Credentials Mode (Single Keychain Prompt)

**dwata uses "master credentials mode"** to minimize OS keychain prompts. Instead of storing each credential as a separate keychain entry, all credentials are stored together in a single master entry as encrypted JSON.

**Benefits:**
- **1 keychain prompt total** (instead of N prompts for N credentials)
- Works identically on macOS, Windows, and Linux
- Supports 200+ credentials in a single entry (tested up to 300+)
- New credentials automatically added to master entry (no new prompts)

**How it works:**
1. All credentials stored in single keychain entry: `"dwata-master"`
2. Entry contains encrypted JSON with all credential data
3. Still uses OS keychain encryption for security
4. In-memory cache reduces keychain access after first load

**Storage format (internal):**
```json
{
  "version": 1,
  "credentials": [
    {
      "type": "imap",
      "identifier": "gmail",
      "username": "user@example.com",
      "password": "encrypted_by_os_keychain"
    }
  ]
}
```

### In-Memory Caching

In addition to master mode, dwata implements an in-memory password cache:

- **Cache TTL**: 1 hour (configurable via `KeyringService::with_ttl()`)
- **Automatic loading**: Master credentials loaded into cache at startup
- **Thread-safe**: Uses `Arc<RwLock<HashMap>>` for concurrent access
- **Automatic expiration**: Cached passwords expire after the TTL

From `dwata-api/src/helpers/keyring_service.rs`:
```rust
// Initialize with default 1 hour TTL
let keyring_service = KeyringService::new();

// Or customize the TTL
let keyring_service = KeyringService::with_ttl(Duration::from_secs(7200)); // 2 hours
```

### First-Time Setup: macOS Keychain Prompt

On macOS, the first time dwata starts, you'll see **one** system prompt:

```
"dwata-api" wants to access the keychain item "dwata-master"
[ Deny ] [ Allow ] [ Always Allow ]
```

**Important**: Select **"Always Allow"** to grant permanent access. You'll never see this prompt again.

If you accidentally selected "Allow" (temporary access), you can fix this:
1. Open **Keychain Access** app
2. Search for "dwata-master"
3. Double-click the entry
4. Go to "Access Control" tab
5. Add `dwata-api` to the "Always allow access" list

### Cache Management

The `KeyringService` provides methods for cache management:

```rust
// Invalidate a specific credential
keyring_service.invalidate(&credential_type, &identifier, &username).await;

// Clear entire cache (useful after password changes)
keyring_service.clear_cache().await;

// Get cache statistics
let (total, expired) = keyring_service.cache_stats().await;
```

### Security Considerations

- Cache is memory-only (never written to disk)
- Cache is cleared when the server stops
- Individual credentials are invalidated when updated or deleted
- TTL ensures passwords don't stay in memory indefinitely

### 5. `tauri` - Desktop App Shell

Location: `/tauri/`

The Tauri app wraps the SolidJS GUI and starts `dwata-api` as a sidecar. It is the primary desktop build target.

## Running the Project

### Running the API Server (Standalone)

```bash
cd dwata-api
cargo run
```

With logging to a file:
```bash
cargo run -- --log-file-path /path/to/log/file.log
```

The server will:
1. Initialize the database at the OS-specific path
2. Load configuration from `~/Library/Application Support/dwata/config.toml` (on macOS)
3. Start the HTTP server on `127.0.0.1:8080` (or as configured)

## GitHub Actions Secrets

The `build-release` workflow builds the Tauri desktop app (and bundles the `dwata-api` sidecar). It can embed default Google OAuth credentials at build time. Set these repository secrets:
- `DWATA_GOOGLE_CLIENT_ID`
- `DWATA_GOOGLE_CLIENT_SECRET`

Release automation (`scripts/release.sh` and `scripts/build-production.sh`) targets the Tauri desktop app bundle, not a standalone `dwata-api` + GUI release.

### Running the GUI (Web)

```bash
cd gui
npm install
npm run dev
```

This starts the development server, typically on `http://localhost:3030`.

### Running the Desktop App (Tauri)

```bash
cd tauri
npm install
npm run dev
```

### Generating TypeScript Types

After modifying types in `shared-types`:

```bash
cd shared-types
cargo run --bin generate_api_types
```

This generates `gui/src/api-types/types.ts` with TypeScript definitions.

## Development Workflow

1. **Modifying API Types**:
   - Edit types in `shared-types/src/`
   - Regenerate TypeScript types: `cargo run --bin generate_api_types`
   - The GUI will automatically use the updated types

2. **Adding API Endpoints**:
   - Add request/response types to `shared-types`
   - Implement handler in `dwata-api/src/handlers/`
   - Register route in `dwata-api/src/main.rs`
   - Regenerate TypeScript types

3. **Database Migrations**:
   - Add migration logic to `dwata-api/src/database/migrations.rs`
   - Migrations run automatically on server startup

## Accessing the Database Directly

If you have the SQLite CLI installed, you can query the database directly:

```bash
# On macOS
sqlite3 ~/Library/Application\ Support/dwata/db.sqlite

# Example queries
SELECT * FROM credentials_metadata;
SELECT * FROM download_jobs;
SELECT * FROM emails;
.tables  # List all tables
.schema credentials_metadata  # Show table schema
```
