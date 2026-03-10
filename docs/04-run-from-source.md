# Run Dwata From Source (This Branch)

This guide is for running Dwata from source with both backend and GUI in development mode.

## Repositories to Clone

Clone these side-by-side under the same parent folder:

```bash
cd /path/to/workspace
git clone https://github.com/brainless/dwata.git
git clone https://github.com/brainless/llm-sdk.git
```

Expected layout:

```text
/path/to/workspace/
  dwata/
  llm-sdk/
```

`dwata` uses a local path dependency to `../../llm-sdk` (aliased as `nocodo-llm-sdk` in Cargo), so `llm-sdk` must be present as a sibling repo.

## Prerequisites

- Rust toolchain (stable)
- Node.js + npm

## 1. Configure API for Local Dev

Start `dwata-api` once to generate config (if missing):

```bash
cd dwata/dwata-api
cargo run --bin dwata-api -- --no-open
```

Config file path:
- macOS: `~/Library/Application Support/dwata/api.toml`
- Linux: `~/.config/dwata/api.toml`
- Windows: `%APPDATA%\dwata\api.toml`

Set values to match current GUI defaults in this branch:

```toml
[server]
host = "localhost"
port = 9200

[cors]
allowed_origins = ["http://localhost:9210"]
```

Notes:
- `host = "localhost"` avoids Google Desktop OAuth callback issues.
- GUI dev server runs on `9210` (`gui/vite.config.ts`).
- GUI API default points to port `9200` (`gui/src/config/api.ts`).

## 2. Run `dwata-api`

```bash
cd dwata/dwata-api
cargo run --bin dwata-api -- --no-open
```

Health check:

```bash
curl http://localhost:9200/api/health
```

## 3. Run GUI

In a second terminal:

```bash
cd dwata/gui
npm install
npm run dev
```

Open `http://localhost:9210`.

## 4. Keep API Types in Sync (when shared-types changes)

```bash
cd dwata/shared-types
cargo run --bin generate_api_types
```

## Source Dependency Check

- Required extra source repo for this branch: `llm-sdk` (for `nocodo-llm-sdk` alias to `llm-sdk` path dependency).
- No other source repo is needed.
