# Epic: Native restart and legacy UI removal

## Status

Proposed

## Overview

Restart Dwata around a native, cross-platform Rust application while retaining `dwata-api` as the backend and application boundary. The abandoned Tauri/SolidJS GUI is not a migration target or reference implementation and will be removed.

The long-term product supports two deployment modes through one typed JSON-over-HTTP API:

- **Server mode:** `dwata-api` serves the API over HTTPS.
- **Desktop mode:** the native Rust UI and API are linked into one desktop binary. The UI uses the API crate's application capabilities directly; it must not depend on a Tauri sidecar or the old web UI.

The detailed desktop composition, UI framework integration, and application-layer extraction are explicitly deferred to later epics.

## Motivation

Dwata's Tauri application and SolidJS GUI were abandoned before reaching users. Maintaining them would preserve a second, web-oriented implementation model that does not help the new native UI and would keep Node, TypeScript, Tauri packaging, generated TypeScript types, and embedded-GUI behavior in the repository.

The restart should instead preserve the valuable Rust backend and agent work while establishing a clear direction:

- A native Rust UI built with the new cross-platform wgpu-based framework.
- A stable, typed API that works for hosted/server deployments over HTTPS.
- A tightly integrated desktop application that can reuse the same API/application code without a loopback HTTP server or sidecar.
- Freedom to reshape the product core and API contract while there are no compatibility obligations.

## Architecture decision

### API protocol

Use a versioned JSON REST-style API over HTTPS as the external/server protocol. A typed REST API is the preferred choice for this restart; do not introduce gRPC/Protobuf at this stage.

JSON/HTTPS fits the existing Actix server, browser-based OAuth callbacks, ordinary resource and command endpoints, debugging, proxying, and future non-Rust clients. Rust can still have a fully typed client through shared contract types and an OpenAPI-described API. gRPC can be reconsidered only if concrete requirements emerge, such as high-volume streaming or polyglot generated clients that justify its extra schema and operational complexity.

Use REST for queries and commands. When long-running work needs live updates, prefer Server-Sent Events for server-to-client job progress; use WebSockets only for genuine bidirectional interaction. Polling remains acceptable for the first restart slices.

### Desktop transport

Desktop mode is a future in-process composition, not a local HTTP service by default:

```text
Native Rust UI ───────────────┐
                              ▼
                       application services
                              ▲
dwata-api HTTP/HTTPS adapter ─┘
```

`dwata-api` will remain the server package, but HTTP handlers should ultimately become thin adapters over transport-neutral application services. The desktop shell will call those services directly. This preserves one business implementation while avoiding Tauri sidecars, CORS, fixed local ports, and duplicated desktop versus server behavior.

### Types and contracts

Keep the Rust types currently in `shared-types` where they represent shared domain concepts or are used by `dwata-agents` and `dwata-api`. Do not remove that crate merely because the web GUI is removed.

Remove types, exports, dependencies, and generation code whose only consumer is the legacy Tauri/SolidJS GUI. In particular, retire TypeScript generation (`ts-rs` and `generate_api_types`) once no remaining consumer needs it.

In a later architecture epic, separate the remaining types deliberately:

- `dwata-domain` (or a similarly named crate): domain entities and shared application concepts used by API and agents.
- `dwata-contract` (or a similarly named crate): public HTTP request/response DTOs, pagination, API errors, and versioned schemas.

This split is a direction, not a requirement for the initial cleanup. Avoid coupling the future native UI directly to Actix request extractors or database models.

## Scope

### In scope

- Remove the Tauri desktop project and its sidecar/package configuration.
- Remove the SolidJS/TypeScript GUI and its Node dependencies; it is not retained as a reference.
- Remove Tauri/GUI launch scripts, packaging inputs, generated TypeScript API types, and references in active documentation.
- Remove GUI-serving/embedding behavior from `dwata-api`, including fallback routes that exist solely to serve the old application.
- Keep `dwata-api`, `dwata-agents`, database migrations/data compatibility, and Rust shared types required by the API or agents.
- Update the Cargo workspace, lockfile, developer documentation, and README so they accurately describe the backend-only interim state.

### Out of scope

- Building the new native UI, its window lifecycle, rendering, navigation, or packaging.
- Designing the final desktop binary/crate layout.
- Extracting all application services out of Actix handlers.
- Redesigning or versioning every existing endpoint.
- Introducing OpenAPI generation, a generated Rust client, SSE, WebSockets, gRPC, authentication redesign, or a hosted deployment stack.
- Changing product behavior, SQLite schema, credentials, OAuth semantics, or agent/extraction logic except where deletion of legacy UI wiring requires it.

## Tasks

- [x] Inventory every Tauri, SolidJS/GUI, TypeScript API-generation, GUI-embedding, and legacy launcher/packaging reference. Identify whether it is active code, documentation, a release asset, or historical task material. See [the inventory](001-native-restart-and-legacy-ui-removal-inventory.md).
- [x] Remove the `tauri/` project, including the Tauri Rust app, Node package files, generated capabilities/schema files, sidecar binary, icons, and Tauri-specific README.
- [x] Remove the `gui/` project, including SolidJS source, Vite configuration, Node package files, generated `api-types`, screenshots or assets that are solely legacy GUI documentation, and GUI-only guides.
- [x] Remove the legacy launch script (`run-dwata-app.sh`) and any Tauri-sidecar build or release wiring.
- [x] Delete `dwata-api` GUI embedding/static-serving code and its default fallback route. Ensure API requests still resolve normally and unknown routes return an API-appropriate 404.
- [x] Remove `shared-types` TypeScript export infrastructure and only types/exports that no longer have Rust consumers. Retain shared API/agent domain types; do not perform a speculative domain/contract split in this cleanup.
- [x] Update `Cargo.toml` workspace membership/exclusions and dependencies to reflect the remaining Rust crates. Regenerate `Cargo.lock` through Cargo after removal.
- [x] Rewrite `README.md` and `DEVELOP.md` for the interim backend-first restart: no desktop download instructions, no GUI/Tauri prerequisites, no TypeScript generation, and no claims that the legacy desktop application is available.
- [x] Search the active repository configuration and docs for stale Tauri/GUI references. Historical material in `tasks/` may either be clearly marked historical or removed in a separate documentation cleanup, but must not be presented as current development guidance.
- [x] Verify the resulting Rust workspace with formatting and workspace tests/builds appropriate to the supported crates. Confirm a clean checkout has no Node/Tauri dependency required for normal backend development.

## Acceptance criteria

- The repository contains no runnable Tauri or SolidJS GUI application and no legacy desktop sidecar artifacts.
- `dwata-api` builds and runs as an API service without serving GUI assets or attempting to open the legacy application.
- `dwata-agents` and `dwata-api` continue to compile against the retained Rust shared types.
- TypeScript-specific type generation and dependencies are gone.
- README and developer guidance accurately state that the native UI is planned, not shipped.
- The workspace's supported Rust crates pass the agreed verification commands.

## Risks and decisions to preserve

- Do not delete types solely because they once generated TypeScript. Delete them only after confirming that no Rust API or agent code uses them.
- Do not remove database migrations or alter persisted data during the UI cleanup.
- OAuth remains HTTP-oriented in server mode. Its future desktop UX will be designed with the native shell, not by retaining Tauri callbacks.
- The old task documents contain implementation history and may mention removed paths. They should not drive restart architecture decisions.

## Next steps (future epics)

1. **Application boundary extraction:** move use cases out of Actix handlers into transport-neutral application services with explicit dependencies and error types.
2. **API contract and versioning:** define `v1` resource/command contracts, a consistent error model, pagination, OpenAPI publication, compatibility rules, and a typed Rust HTTP client.
3. **Native desktop shell:** create the native wgpu UI crate, compose it with the application services into one binary, and establish desktop lifecycle, configuration, secure storage, and OAuth UX.
4. **Job/event transport:** decide where polling is sufficient and add SSE or WebSockets for concrete progress/interactive requirements.
5. **Server deployment and security:** define TLS termination, authentication/authorization, secret management, remote storage/backup policy, and operational packaging.
