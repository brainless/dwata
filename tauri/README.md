# Dwata Tauri App (v2)

This package wires the existing SolidJS GUI (`../gui`) into a Tauri v2 shell and starts the `dwata-api` as a sidecar.

## Dev

1. Start the GUI dev server automatically through Tauri:
   - `cd tauri`
   - `npm install`
   - `npm run dev`

2. Provide a sidecar path in dev (recommended):
   - `DWATA_API_PATH=/path/to/dwata-api npm run dev`

If you do not set `DWATA_API_PATH`, the app will try to run a bundled sidecar from the app resources and will log an error when it is missing.

## Bundling the API sidecar

The bundle config expects a binary named `dwata-api` in `tauri/bin/` (or platform-specific names per Tauri rules). For example:

- macOS: `tauri/bin/dwata-api-x86_64-apple-darwin` or `tauri/bin/dwata-api-aarch64-apple-darwin`
- Windows: `tauri/bin/dwata-api-x86_64-pc-windows-msvc.exe`
- Linux: `tauri/bin/dwata-api-x86_64-unknown-linux-gnu`

Place the appropriate binary in `tauri/bin/` before running `npm run build`.

## Notes

- The GUI is served from `http://localhost:3030` in dev.
- Use `server.host = "localhost"` in the API config for Google Desktop OAuth (not `127.0.0.1`).
