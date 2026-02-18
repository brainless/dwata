# Task: Daemon-Based Updater + Keychain Proxy (Planning Only)

## Objective

Define a plan for a small `dwata-daemon` that:
- Owns OS keychain access on behalf of `dwata-api`
- Checks for updates and downloads `dwata-api` (and GUI) from GitHub releases
- Starts `dwata-api` and manages lifecycle
- Can register itself for OS auto-start

This document is a planning artifact only. Implementation must not start until this task is reviewed and explicitly approved.

## Background

The current `dwata-api` binary accesses the OS keychain directly. On macOS, keychain prompts reappear after binary updates unless signing and access controls are stable. A stable daemon that alone accesses the keychain can reduce prompts by keeping the access identity constant across `dwata-api` updates.

## Proposed Approach

### High-Level Behavior
- User downloads `dwata-daemon` from GitHub releases.
- On first run, daemon:
  - installs itself for auto-start (with user consent)
  - checks GitHub releases for latest `dwata-api` and GUI
  - downloads and verifies artifacts
  - starts `dwata-api` and optionally the GUI
- `dwata-api` never touches keychain directly.
- `dwata-api` requests secrets via IPC from the daemon.

### IPC Model
- Local-only IPC (Unix domain socket on macOS/Linux, named pipe or localhost TCP on Windows).
- Mutually authenticated channel using a short-lived token or a shared secret written to a permissions-restricted file.
- Requests are minimal and scoped:
  - `get_credential`
  - `set_credential`
  - `delete_credential`
  - `health`
- Daemon performs in-memory caching with TTL (matching or replacing current keyring cache).

### Autostart
- macOS: LaunchAgent under user context (not system daemon).
- Windows: Task Scheduler or Run key.
- Linux: `systemd --user` or `~/.config/autostart/*.desktop`.

### Update Flow
- Daemon queries GitHub releases API.
- Downloads artifacts for current platform.
- Verifies artifact integrity.
- Replaces previous `dwata-api` binary and restarts the process.

## Security Considerations (To Review)

- Release artifact verification:
  - Signed release manifests or Ed25519 signatures.
  - Hash verification before install.
- Code signing and notarization:
  - macOS signing for daemon and app binaries with the same Team ID.
  - Notarization to reduce warnings.
- IPC hardening:
  - Local-only, authenticated, permission-restricted socket.
  - Request allowlist and strict input validation.
- Credential exposure:
  - Daemon is sole keychain caller.
  - `dwata-api` receives only required secrets.
  - In-memory cache TTL and explicit invalidation.
- Supply chain:
  - GitHub release verification and pinned owner/repo.
  - Update rollback on failure.
- Least privilege:
  - No admin rights required for user-level install.

## Non-Goals

- No system-wide installs or admin-required services in v1.
- No remote keychain access.
- No auto-migration of existing keychain entries between identities without explicit user action.

## Deliverables (After Review)

- `dwata-daemon` binary for macOS, Windows, Linux.
- IPC protocol definition and Rust client for `dwata-api`.
- Update mechanism with verification and rollback.
- OS auto-start setup scripts or embedded installer logic.
- Documentation and troubleshooting guide.

## Review Gate (Blocking)

Work on this task must not begin until this document is reviewed and explicitly approved. This review should confirm:
- Update verification strategy
- IPC security model
- Signing and notarization plan
- OS auto-start behavior and UX

## Next Steps

- Review this task with the team.
- Decide on update signing strategy and IPC transport.
- Confirm which OSes are in scope for v1.

