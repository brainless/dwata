# Run Dwata From Source

This guide is for running the supported Rust backend from source.

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

## 1. Configure API for Local Dev

Start `dwata-api` once to generate configuration if it is missing:

```bash
cd dwata
cargo run -p dwata-api
```

Config file path:
- macOS: `~/Library/Application Support/dwata/config.toml`
- Linux: `~/.config/dwata/config.toml`
- Windows: `%APPDATA%\dwata\config.toml`

Set values appropriate for your local API instance, for example:

```toml
[server]
host = "127.0.0.1"
port = 8080
```

Notes:
- The Google OAuth callback URI is derived from the configured host and port.
- Configure `[cors]` only when you intentionally run a browser client in
  front of the API.

## 2. Run `dwata-api`

```bash
cd dwata
cargo run -p dwata-api
```

Health check:

```bash
curl http://127.0.0.1:8080/api/health
```

## Source Dependency Check

- Required extra source repo for this branch: `llm-sdk` (for `nocodo-llm-sdk` alias to `llm-sdk` path dependency).
- No other source repo is needed.
