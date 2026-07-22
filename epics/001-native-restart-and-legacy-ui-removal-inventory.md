# Epic 001 legacy UI inventory

Inventory completed for Epic 001 task 1. Paths below are classified by their
current role; `tasks/` items are implementation history, not current guidance.

## Active code and configuration

- `tauri/` — runnable desktop project: Node manifests, Tauri Rust shell,
  generated capability/schema files, icon, Tauri README, and the tracked
  `bin/dwata-api-aarch64-apple-darwin` sidecar release binary. Its
  `tauri.conf.json` builds and loads `../gui` and bundles that sidecar.
- `gui/` — runnable SolidJS/Vite application: Node manifests, TypeScript/Vite
  configuration, application source, `.env.example`, GUI guide, and generated
  `src/api-types/types.ts`.
- `shared-types/Cargo.toml` and `shared-types/src/bin/generate_api_types.rs`
  — `ts-rs` dependency and the TypeScript export binary targeting
  `gui/src/api-types/types.ts`.
- `dwata-api/src/main.rs` — inactive-but-live GUI embedding scaffolding:
  `GUI_EMBED_ENABLED`, `gui_embed::serve_gui`, fallback service, and browser
  launch condition. `dwata-api/src/config.rs` and `config.example.toml` retain
  the GUI port configuration/default (`3030`).
- Root `Cargo.toml` — excludes `gui` from the Cargo workspace.
- `run-dwata-app.sh` — installs GUI/Tauri packages, builds the API sidecar, and
  launches Tauri.

## Release and packaging inputs

- `.github/workflows/build-release.yml` — macOS Tauri build/release workflow;
  caches `gui/package-lock.json`, installs the Tauri CLI, and publishes DMG/app
  artifacts.
- `scripts/build-production.sh` — generates TypeScript types, installs GUI
  dependencies, copies the API sidecar into `tauri/bin`, and creates a Tauri
  bundle.
- `scripts/release.sh` — versions `gui/package.json` and Tauri manifests and
  stages them for release.
- `packaging/arch/PKGBUILD` — declares Node/npm build requirements and builds
  the GUI before the API. `packaging/arch/INSTALL.md` still lists Node/npm as
  build dependencies. `packaging/README.md` describes API-only release assets
  and has no Tauri/GUI dependency.

## Active documentation and release assets

- `README.md` — says the released Tauri desktop app starts an API sidecar and
  lists SolidJS as the frontend.
- `DEVELOP.md` — documents GUI/Tauri prerequisites, projects, running modes,
  Tauri release automation, and TypeScript generation.
- `docs/README.md`, `docs/02-current-architecture.md`, and
  `docs/04-run-from-source.md` — present the GUI as a current runtime component
  and development prerequisite.
- `docs/assets/dwata_email_home.png`,
  `dwata_detect_financial_templates_with_ai.png`,
  `dwata_run_financial_template_detection.png`, `dwata_financial_templates.png`,
  `dwata_financial_transactions.png`, `dwata_financial_bills.png`, and
  `dwata_settings_ollama_ministral3_3b.png` — legacy GUI screenshots embedded
  by `README.md`; remove or replace when the README is rewritten.

## Historical task material

The following files contain legacy GUI, TypeScript-generation, or GUI-launch
instructions and must be explicitly labelled historical or cleaned separately
so they are not read as current guidance:

- `tasks/credential-storage-api.md`
- `tasks/credentials-list-implementation.md`
- `tasks/credentials-list-visual.md`
- `tasks/dwata-daemon-updater-plan.md`
- `tasks/email-storage-and-imap-implementation.md`
- `tasks/extraction-framework-foundation.md`
- `tasks/extraction-job-manager.md`
- `tasks/gmail-oauth2-authentication.md`
- `tasks/imap-credential-types-usage.md`
- `tasks/imap-download-manager.md`
- `tasks/linkedin-archive-extractor.md`
- `tasks/normalize-email-folders-labels.md`
- `tasks/settings-imap-form-implementation.md`
- `tasks/setup-core-types-and-database.md`
- `tasks/tantivy-search-for-documents.md`
- `tasks/unified-documents-api-and-pagination.md`

`epics/001-native-restart-and-legacy-ui-removal.md` is current planning
material, not a stale reference; it intentionally names the legacy paths as
removal targets. No `AGENTS.md` exists in this checkout. No `Cargo.lock` is
tracked or present, so the later lockfile task must generate one after the
dependency cleanup.
