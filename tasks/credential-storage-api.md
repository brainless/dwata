# Task: Cross-Platform Credential Storage API Endpoint

## Objective

Implement secure credential storage API endpoints that leverage OS-native keychains (macOS Keychain, Windows Credential Manager, Linux Secret Service) for storing sensitive credentials like passwords, while maintaining metadata in DuckDB for efficient listing and searching capabilities.

## Background

Modern operating systems provide secure credential storage mechanisms:
- **macOS**: Keychain Services (integrated with iCloud Keychain)
- **Windows**: Credential Manager (DPAPI-based encryption)
- **Linux**: Secret Service (libsecret/GNOME Keyring/KWallet)

These OS-level services provide:
- Hardware-backed encryption (where available)
- OS-level access control
- Secure credential lifecycle management
- Native backup/sync capabilities

This implementation uses a **hybrid approach**:
- **Secrets** (passwords, tokens) → OS keychain via `keyring` crate
- **Metadata** (usernames, types, timestamps) → DuckDB for fast queries
- **Separation of concerns**: Sensitive data never touches the database

## Security Considerations

### Current State
⚠️ **IMPORTANT**: The dwata API currently has **NO authentication mechanism**. The API binds to localhost only (127.0.0.1), which provides basic protection against remote access, but anyone with local access can interact with the API.

### Mitigations
1. **Configuration Flag**: Add `credentials_api_enabled = false` by default (must be explicitly enabled)
2. **Security Warning**: Add `X-Security-Warning` header to all credential responses
3. **Documentation**: Clearly document security implications in README

### Future Enhancements
- Implement authentication/authorization system
- Add audit logging for all credential access
- Implement rate limiting
- Add role-based access control (RBAC)
- Consider encryption-at-rest for metadata in DuckDB

## Architecture

### Storage Strategy

```
┌─────────────────────────────────────────────────────────┐
│                    API Request                          │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│              HTTP Handler Layer                         │
│  (credentials.rs - POST/GET/PUT/DELETE)                │
└─────────────────────────────────────────────────────────┘
                          ↓
         ┌────────────────┴────────────────┐
         ↓                                  ↓
┌──────────────────────┐      ┌──────────────────────┐
│  Keyring Service     │      │  Database Operations │
│  (keyring_service.rs)│      │  (credentials.rs)    │
│                      │      │                      │
│  - set_password()    │      │  - insert_metadata() │
│  - get_password()    │      │  - list_credentials()│
│  - delete_password() │      │  - update_metadata() │
└──────────────────────┘      └──────────────────────┘
         ↓                                  ↓
┌──────────────────────┐      ┌──────────────────────┐
│   OS Keychain        │      │      DuckDB          │
│                      │      │                      │
│  Passwords/Tokens    │      │  Metadata/Timestamps │
└──────────────────────┘      └──────────────────────┘
```

### Naming Conventions

**Service Name Format**: `dwata:<credential_type>`
- Examples: `dwata:imap`, `dwata:smtp`, `dwata:oauth`

**Username Format**: `<identifier>:<username>`
- Examples: `work_email:john@company.com`, `personal_gmail:jane@gmail.com`

This allows:
- Easy identification of dwata credentials in OS keychain viewers
- Multiple credentials of the same type with different identifiers
- Human-readable keychain entries

## API Endpoints

### 1. Create Credential
```http
POST /api/credentials
Content-Type: application/json

{
  "credential_type": "imap",
  "identifier": "work_email",
  "username": "john@company.com",
  "password": "secure_password_123",
  "service_name": "imap.gmail.com",
  "port": 993,
  "use_tls": true,
  "notes": "Work email account"
}
```

**Response (201 Created)**:
```json
{
  "id": "cred_abc123xyz",
  "credential_type": "imap",
  "identifier": "work_email",
  "username": "john@company.com",
  "service_name": "imap.gmail.com",
  "port": 993,
  "use_tls": true,
  "notes": "Work email account",
  "created_at": 1706745600000,
  "updated_at": 1706745600000,
  "is_active": true
}
```

**Errors**:
- `400`: Invalid credential_type, missing required fields, duplicate identifier
- `503`: OS keychain service unavailable
- `500`: Internal error (rollback performed if possible)

### 2. List Credentials (Metadata Only)
```http
GET /api/credentials
```

**Response (200 OK)**:
```json
{
  "credentials": [
    {
      "id": "cred_abc123xyz",
      "credential_type": "imap",
      "identifier": "work_email",
      "username": "john@company.com",
      "service_name": "imap.gmail.com",
      "port": 993,
      "use_tls": true,
      "created_at": 1706745600000,
      "updated_at": 1706745600000,
      "last_accessed_at": 1706831200000,
      "is_active": true
    }
  ]
}
```

**Note**: Passwords are NEVER included in list responses.

### 3. Get Single Credential Metadata
```http
GET /api/credentials/:id
```

**Response (200 OK)**: Same as individual item from list endpoint

**Errors**:
- `404`: Credential not found

### 4. Retrieve Password
```http
GET /api/credentials/:id/password
```

**Response (200 OK)**:
```json
{
  "password": "secure_password_123"
}
```

**Headers**:
```
X-Security-Warning: This API has no authentication. Enable only in trusted environments.
```

**Errors**:
- `404`: Credential not found
- `500`: Keychain entry exists in DB but not in OS keychain (inconsistent state)
- `503`: OS keychain service unavailable

**Side Effect**: Updates `last_accessed_at` timestamp in metadata.

### 5. Update Credential
```http
PUT /api/credentials/:id
Content-Type: application/json

{
  "username": "jane@company.com",
  "password": "new_password_456",
  "service_name": "imap.newserver.com",
  "port": 993,
  "notes": "Updated to new server"
}
```

**Response (200 OK)**: Updated credential metadata (same format as GET)

**Errors**:
- `404`: Credential not found
- `400`: Invalid data
- `503`: OS keychain service unavailable

**Behavior**:
- Omitted fields remain unchanged
- Updating password replaces the OS keychain entry
- Metadata changes are atomic (transaction)

### 6. Delete Credential
```http
DELETE /api/credentials/:id?hard=false
```

**Query Parameters**:
- `hard=false` (default): Soft delete (sets `is_active = false`, keeps keychain entry)
- `hard=true`: Hard delete (removes from DB and keychain entirely)

**Response (204 No Content)**

**Errors**:
- `404`: Credential not found
- `503`: OS keychain service unavailable (hard delete only)

## Implementation Plan

### Phase 1: Dependencies and Types

#### 1.1 Add Dependencies

**File**: `/Users/brainless/Projects/dwata/dwata-api/Cargo.toml`

Add to `[dependencies]`:
```toml
keyring = { version = "2.3", features = ["linux-native"] }
```

**Note**: The `linux-native` feature enables libsecret support on Linux. On macOS/Windows, it uses native APIs automatically.

#### 1.2 Create Shared Types

**File**: `/Users/brainless/Projects/dwata/shared-types/src/credential.rs`

```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Supported credential types
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum CredentialType {
    Imap,
    Smtp,
    OAuth,
    ApiKey,
    Database,
    Custom,
}

impl CredentialType {
    /// Returns the service name format for OS keychain
    pub fn service_name(&self) -> String {
        format!("dwata:{}", self.as_str())
    }

    pub fn as_str(&self) -> &str {
        match self {
            CredentialType::Imap => "imap",
            CredentialType::Smtp => "smtp",
            CredentialType::OAuth => "oauth",
            CredentialType::ApiKey => "apikey",
            CredentialType::Database => "database",
            CredentialType::Custom => "custom",
        }
    }
}

impl std::fmt::Display for CredentialType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Request to create a new credential
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateCredentialRequest {
    pub credential_type: CredentialType,
    pub identifier: String,
    pub username: String,
    pub password: String,
    pub service_name: Option<String>,
    pub port: Option<i32>,
    pub use_tls: Option<bool>,
    pub notes: Option<String>,
    pub extra_metadata: Option<String>, // JSON string for extensibility
}

/// Request to update an existing credential
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateCredentialRequest {
    pub username: Option<String>,
    pub password: Option<String>,
    pub service_name: Option<String>,
    pub port: Option<i32>,
    pub use_tls: Option<bool>,
    pub notes: Option<String>,
    pub extra_metadata: Option<String>,
}

/// Credential metadata (no password included)
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CredentialMetadata {
    pub id: String,
    pub credential_type: CredentialType,
    pub identifier: String,
    pub username: String,
    pub service_name: Option<String>,
    pub port: Option<i32>,
    pub use_tls: Option<bool>,
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_accessed_at: Option<i64>,
    pub is_active: bool,
    pub extra_metadata: Option<String>,
}

/// Response containing a password from the keychain
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct PasswordResponse {
    pub password: String,
}

/// List of credentials (metadata only)
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct CredentialListResponse {
    pub credentials: Vec<CredentialMetadata>,
}
```

**File**: `/Users/brainless/Projects/dwata/shared-types/src/lib.rs`

Add export:
```rust
pub mod credential;
```

### Phase 2: Database Schema and Migrations

#### 2.1 Add Migration

**File**: `/Users/brainless/Projects/dwata/dwata-api/src/database/migrations.rs`

Add to the `run_migrations()` function:

```rust
pub async fn run_migrations(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    // ... existing migrations ...

    // Credential metadata table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS credentials_metadata (
            id VARCHAR PRIMARY KEY,
            credential_type VARCHAR NOT NULL,
            identifier VARCHAR NOT NULL UNIQUE,
            username VARCHAR NOT NULL,
            service_name VARCHAR,
            port INTEGER,
            use_tls BOOLEAN DEFAULT TRUE,
            notes VARCHAR,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,
            last_accessed_at BIGINT,
            is_active BOOLEAN DEFAULT TRUE,
            extra_metadata VARCHAR
        )",
        [],
    ).await?;

    // Index for efficient listing and filtering
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_credentials_type_active
         ON credentials_metadata(credential_type, is_active)",
        [],
    ).await?;

    Ok(())
}
```

### Phase 3: Keyring Service Module

**File**: `/Users/brainless/Projects/dwata/dwata-api/src/helpers/keyring_service.rs`

```rust
use keyring::Entry;
use shared_types::credential::CredentialType;

#[derive(Debug)]
pub enum KeyringError {
    NotFound,
    ServiceUnavailable(String),
    OperationFailed(String),
}

impl std::fmt::Display for KeyringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyringError::NotFound => write!(f, "Credential not found in keychain"),
            KeyringError::ServiceUnavailable(msg) => {
                write!(f, "Keychain service unavailable: {}", msg)
            }
            KeyringError::OperationFailed(msg) => write!(f, "Keychain operation failed: {}", msg),
        }
    }
}

impl std::error::Error for KeyringError {}

/// Keyring service for cross-platform credential storage
pub struct KeyringService;

impl KeyringService {
    /// Generate keychain username from identifier and username
    /// Format: "<identifier>:<username>"
    fn keychain_username(identifier: &str, username: &str) -> String {
        format!("{}:{}", identifier, username)
    }

    /// Store password in OS keychain
    pub fn set_password(
        credential_type: &CredentialType,
        identifier: &str,
        username: &str,
        password: &str,
    ) -> Result<(), KeyringError> {
        let service = credential_type.service_name();
        let keychain_user = Self::keychain_username(identifier, username);

        let entry = Entry::new(&service, &keychain_user).map_err(|e| {
            KeyringError::ServiceUnavailable(format!("Failed to create keychain entry: {}", e))
        })?;

        entry.set_password(password).map_err(|e| {
            KeyringError::OperationFailed(format!("Failed to store password: {}", e))
        })?;

        Ok(())
    }

    /// Retrieve password from OS keychain
    pub fn get_password(
        credential_type: &CredentialType,
        identifier: &str,
        username: &str,
    ) -> Result<String, KeyringError> {
        let service = credential_type.service_name();
        let keychain_user = Self::keychain_username(identifier, username);

        let entry = Entry::new(&service, &keychain_user).map_err(|e| {
            KeyringError::ServiceUnavailable(format!("Failed to create keychain entry: {}", e))
        })?;

        entry.get_password().map_err(|e| {
            // Distinguish between "not found" and other errors
            if e.to_string().contains("not found") || e.to_string().contains("NotFound") {
                KeyringError::NotFound
            } else {
                KeyringError::OperationFailed(format!("Failed to retrieve password: {}", e))
            }
        })
    }

    /// Delete password from OS keychain
    pub fn delete_password(
        credential_type: &CredentialType,
        identifier: &str,
        username: &str,
    ) -> Result<(), KeyringError> {
        let service = credential_type.service_name();
        let keychain_user = Self::keychain_username(identifier, username);

        let entry = Entry::new(&service, &keychain_user).map_err(|e| {
            KeyringError::ServiceUnavailable(format!("Failed to create keychain entry: {}", e))
        })?;

        entry.delete_credential().map_err(|e| {
            if e.to_string().contains("not found") || e.to_string().contains("NotFound") {
                KeyringError::NotFound
            } else {
                KeyringError::OperationFailed(format!("Failed to delete password: {}", e))
            }
        })?;

        Ok(())
    }

    /// Update password in OS keychain (convenience method)
    pub fn update_password(
        credential_type: &CredentialType,
        identifier: &str,
        username: &str,
        new_password: &str,
    ) -> Result<(), KeyringError> {
        // keyring crate's set_password automatically updates existing entries
        Self::set_password(credential_type, identifier, username, new_password)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keychain_username_format() {
        let username = KeyringService::keychain_username("work_email", "john@company.com");
        assert_eq!(username, "work_email:john@company.com");
    }

    #[test]
    fn test_service_name_format() {
        let service = CredentialType::Imap.service_name();
        assert_eq!(service, "dwata:imap");
    }
}
```

**File**: `/Users/brainless/Projects/dwata/dwata-api/src/helpers/mod.rs`

Add:
```rust
pub mod keyring_service;
```

### Phase 4: Database Operations

**File**: `/Users/brainless/Projects/dwata/dwata-api/src/database/credentials.rs`

```rust
use duckdb::Connection;
use shared_types::credential::{CredentialMetadata, CredentialType, CreateCredentialRequest};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub enum CredentialDbError {
    NotFound,
    DuplicateIdentifier,
    DatabaseError(String),
}

impl std::fmt::Display for CredentialDbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CredentialDbError::NotFound => write!(f, "Credential not found"),
            CredentialDbError::DuplicateIdentifier => {
                write!(f, "A credential with this identifier already exists")
            }
            CredentialDbError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for CredentialDbError {}

/// Generate a unique ID for credentials
fn generate_credential_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let random_part: String = (0..12)
        .map(|_| {
            let idx = rng.gen_range(0..36);
            "abcdefghijklmnopqrstuvwxyz0123456789"
                .chars()
                .nth(idx)
                .unwrap()
        })
        .collect();
    format!("cred_{}", random_part)
}

/// Insert credential metadata into database
pub async fn insert_credential(
    conn: Arc<Mutex<Connection>>,
    request: &CreateCredentialRequest,
) -> Result<CredentialMetadata, CredentialDbError> {
    let conn = conn.lock().await;
    let id = generate_credential_id();
    let now = chrono::Utc::now().timestamp_millis();

    // Check for duplicate identifier
    let mut stmt = conn
        .prepare("SELECT COUNT(*) FROM credentials_metadata WHERE identifier = ?")
        .map_err(|e| CredentialDbError::DatabaseError(e.to_string()))?;

    let count: i64 = stmt
        .query_row([&request.identifier], |row| row.get(0))
        .map_err(|e| CredentialDbError::DatabaseError(e.to_string()))?;

    if count > 0 {
        return Err(CredentialDbError::DuplicateIdentifier);
    }

    // Insert credential
    conn.execute(
        "INSERT INTO credentials_metadata
         (id, credential_type, identifier, username, service_name, port, use_tls, notes,
          created_at, updated_at, is_active, extra_metadata)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        duckdb::params![
            &id,
            request.credential_type.as_str(),
            &request.identifier,
            &request.username,
            &request.service_name,
            &request.port,
            &request.use_tls.unwrap_or(true),
            &request.notes,
            now,
            now,
            true,
            &request.extra_metadata,
        ],
    )
    .map_err(|e| CredentialDbError::DatabaseError(e.to_string()))?;

    Ok(CredentialMetadata {
        id,
        credential_type: request.credential_type.clone(),
        identifier: request.identifier.clone(),
        username: request.username.clone(),
        service_name: request.service_name.clone(),
        port: request.port,
        use_tls: request.use_tls,
        notes: request.notes.clone(),
        created_at: now,
        updated_at: now,
        last_accessed_at: None,
        is_active: true,
        extra_metadata: request.extra_metadata.clone(),
    })
}

/// List all credentials (active only by default)
pub async fn list_credentials(
    conn: Arc<Mutex<Connection>>,
    include_inactive: bool,
) -> Result<Vec<CredentialMetadata>, CredentialDbError> {
    let conn = conn.lock().await;

    let query = if include_inactive {
        "SELECT id, credential_type, identifier, username, service_name, port, use_tls, notes,
                created_at, updated_at, last_accessed_at, is_active, extra_metadata
         FROM credentials_metadata
         ORDER BY created_at DESC"
    } else {
        "SELECT id, credential_type, identifier, username, service_name, port, use_tls, notes,
                created_at, updated_at, last_accessed_at, is_active, extra_metadata
         FROM credentials_metadata
         WHERE is_active = true
         ORDER BY created_at DESC"
    };

    let mut stmt = conn
        .prepare(query)
        .map_err(|e| CredentialDbError::DatabaseError(e.to_string()))?;

    let rows = stmt
        .query_map([], |row| {
            let credential_type_str: String = row.get(1)?;
            let credential_type = match credential_type_str.as_str() {
                "imap" => CredentialType::Imap,
                "smtp" => CredentialType::Smtp,
                "oauth" => CredentialType::OAuth,
                "apikey" => CredentialType::ApiKey,
                "database" => CredentialType::Database,
                _ => CredentialType::Custom,
            };

            Ok(CredentialMetadata {
                id: row.get(0)?,
                credential_type,
                identifier: row.get(2)?,
                username: row.get(3)?,
                service_name: row.get(4)?,
                port: row.get(5)?,
                use_tls: row.get(6)?,
                notes: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
                last_accessed_at: row.get(10)?,
                is_active: row.get(11)?,
                extra_metadata: row.get(12)?,
            })
        })
        .map_err(|e| CredentialDbError::DatabaseError(e.to_string()))?;

    let mut credentials = Vec::new();
    for row in rows {
        credentials.push(row.map_err(|e| CredentialDbError::DatabaseError(e.to_string()))?);
    }

    Ok(credentials)
}

/// Get a single credential by ID
pub async fn get_credential(
    conn: Arc<Mutex<Connection>>,
    id: &str,
) -> Result<CredentialMetadata, CredentialDbError> {
    let conn = conn.lock().await;

    let mut stmt = conn
        .prepare(
            "SELECT id, credential_type, identifier, username, service_name, port, use_tls, notes,
                    created_at, updated_at, last_accessed_at, is_active, extra_metadata
             FROM credentials_metadata
             WHERE id = ?",
        )
        .map_err(|e| CredentialDbError::DatabaseError(e.to_string()))?;

    stmt.query_row([id], |row| {
        let credential_type_str: String = row.get(1)?;
        let credential_type = match credential_type_str.as_str() {
            "imap" => CredentialType::Imap,
            "smtp" => CredentialType::Smtp,
            "oauth" => CredentialType::OAuth,
            "apikey" => CredentialType::ApiKey,
            "database" => CredentialType::Database,
            _ => CredentialType::Custom,
        };

        Ok(CredentialMetadata {
            id: row.get(0)?,
            credential_type,
            identifier: row.get(2)?,
            username: row.get(3)?,
            service_name: row.get(4)?,
            port: row.get(5)?,
            use_tls: row.get(6)?,
            notes: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
            last_accessed_at: row.get(10)?,
            is_active: row.get(11)?,
            extra_metadata: row.get(12)?,
        })
    })
    .map_err(|e| match e {
        duckdb::Error::QueryReturnedNoRows => CredentialDbError::NotFound,
        _ => CredentialDbError::DatabaseError(e.to_string()),
    })
}

/// Update last_accessed_at timestamp
pub async fn update_last_accessed(
    conn: Arc<Mutex<Connection>>,
    id: &str,
) -> Result<(), CredentialDbError> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "UPDATE credentials_metadata SET last_accessed_at = ? WHERE id = ?",
        duckdb::params![now, id],
    )
    .map_err(|e| CredentialDbError::DatabaseError(e.to_string()))?;

    Ok(())
}

/// Update credential metadata
pub async fn update_credential(
    conn: Arc<Mutex<Connection>>,
    id: &str,
    username: Option<String>,
    service_name: Option<String>,
    port: Option<i32>,
    use_tls: Option<bool>,
    notes: Option<String>,
    extra_metadata: Option<String>,
) -> Result<CredentialMetadata, CredentialDbError> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    // Build dynamic UPDATE query
    let mut updates = vec!["updated_at = ?"];
    let mut params: Vec<Box<dyn duckdb::ToSql>> = vec![Box::new(now)];

    if username.is_some() {
        updates.push("username = ?");
        params.push(Box::new(username.clone()));
    }
    if service_name.is_some() {
        updates.push("service_name = ?");
        params.push(Box::new(service_name.clone()));
    }
    if port.is_some() {
        updates.push("port = ?");
        params.push(Box::new(port));
    }
    if use_tls.is_some() {
        updates.push("use_tls = ?");
        params.push(Box::new(use_tls));
    }
    if notes.is_some() {
        updates.push("notes = ?");
        params.push(Box::new(notes.clone()));
    }
    if extra_metadata.is_some() {
        updates.push("extra_metadata = ?");
        params.push(Box::new(extra_metadata.clone()));
    }

    params.push(Box::new(id.to_string()));

    let query = format!(
        "UPDATE credentials_metadata SET {} WHERE id = ?",
        updates.join(", ")
    );

    let params_refs: Vec<&dyn duckdb::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    conn.execute(&query, params_refs.as_slice())
        .map_err(|e| CredentialDbError::DatabaseError(e.to_string()))?;

    drop(conn);
    get_credential(Arc::clone(&Arc::new(Mutex::new(
        Connection::open_in_memory().unwrap(),
    ))), id).await
}

/// Soft delete credential (set is_active = false)
pub async fn soft_delete_credential(
    conn: Arc<Mutex<Connection>>,
    id: &str,
) -> Result<(), CredentialDbError> {
    let conn = conn.lock().await;

    let rows_affected = conn
        .execute(
            "UPDATE credentials_metadata SET is_active = false WHERE id = ?",
            [id],
        )
        .map_err(|e| CredentialDbError::DatabaseError(e.to_string()))?;

    if rows_affected == 0 {
        return Err(CredentialDbError::NotFound);
    }

    Ok(())
}

/// Hard delete credential (remove from database)
pub async fn hard_delete_credential(
    conn: Arc<Mutex<Connection>>,
    id: &str,
) -> Result<(), CredentialDbError> {
    let conn = conn.lock().await;

    let rows_affected = conn
        .execute("DELETE FROM credentials_metadata WHERE id = ?", [id])
        .map_err(|e| CredentialDbError::DatabaseError(e.to_string()))?;

    if rows_affected == 0 {
        return Err(CredentialDbError::NotFound);
    }

    Ok(())
}
```

**File**: `/Users/brainless/Projects/dwata/dwata-api/src/database/mod.rs`

Add:
```rust
pub mod credentials;
```

### Phase 5: HTTP Handlers

**File**: `/Users/brainless/Projects/dwata/dwata-api/src/handlers/credentials.rs`

```rust
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use shared_types::credential::{
    CreateCredentialRequest, CredentialListResponse, CredentialMetadata, PasswordResponse,
    UpdateCredentialRequest,
};

use crate::{
    database::credentials as db,
    helpers::keyring_service::{KeyringError, KeyringService},
    AppState,
};

/// Error responses for credential endpoints
#[derive(Debug)]
enum CredentialError {
    Validation(String),
    NotFound,
    Duplicate,
    KeychainUnavailable(String),
    InconsistentState(String),
    Internal(String),
}

impl IntoResponse for CredentialError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            CredentialError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            CredentialError::NotFound => {
                (StatusCode::NOT_FOUND, "Credential not found".to_string())
            }
            CredentialError::Duplicate => (
                StatusCode::BAD_REQUEST,
                "A credential with this identifier already exists".to_string(),
            ),
            CredentialError::KeychainUnavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
            CredentialError::InconsistentState(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            CredentialError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

/// Add security warning header to responses
fn add_security_header(mut headers: HeaderMap) -> HeaderMap {
    headers.insert(
        "X-Security-Warning",
        "This API has no authentication. Enable only in trusted environments."
            .parse()
            .unwrap(),
    );
    headers
}

/// POST /api/credentials - Create new credential
pub async fn create_credential(
    State(state): State<AppState>,
    Json(request): Json<CreateCredentialRequest>,
) -> Result<(StatusCode, HeaderMap, Json<CredentialMetadata>), CredentialError> {
    // Validate required fields
    if request.identifier.trim().is_empty() {
        return Err(CredentialError::Validation(
            "Identifier cannot be empty".to_string(),
        ));
    }
    if request.username.trim().is_empty() {
        return Err(CredentialError::Validation(
            "Username cannot be empty".to_string(),
        ));
    }
    if request.password.trim().is_empty() {
        return Err(CredentialError::Validation(
            "Password cannot be empty".to_string(),
        ));
    }

    // Step 1: Store password in OS keychain first
    KeyringService::set_password(
        &request.credential_type,
        &request.identifier,
        &request.username,
        &request.password,
    )
    .map_err(|e| match e {
        KeyringError::ServiceUnavailable(msg) => CredentialError::KeychainUnavailable(msg),
        _ => CredentialError::Internal(format!("Failed to store password: {}", e)),
    })?;

    // Step 2: Store metadata in database
    let metadata = db::insert_credential(state.db_conn.clone(), &request)
        .await
        .map_err(|e| {
            // Rollback: delete from keychain if database insert fails
            let _ = KeyringService::delete_password(
                &request.credential_type,
                &request.identifier,
                &request.username,
            );

            match e {
                db::CredentialDbError::DuplicateIdentifier => CredentialError::Duplicate,
                db::CredentialDbError::DatabaseError(msg) => CredentialError::Internal(msg),
                _ => CredentialError::Internal(e.to_string()),
            }
        })?;

    Ok((
        StatusCode::CREATED,
        add_security_header(HeaderMap::new()),
        Json(metadata),
    ))
}

/// GET /api/credentials - List all credentials (metadata only)
#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    include_inactive: bool,
}

pub async fn list_credentials(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<(HeaderMap, Json<CredentialListResponse>), CredentialError> {
    let credentials = db::list_credentials(state.db_conn.clone(), query.include_inactive)
        .await
        .map_err(|e| CredentialError::Internal(e.to_string()))?;

    Ok((
        add_security_header(HeaderMap::new()),
        Json(CredentialListResponse { credentials }),
    ))
}

/// GET /api/credentials/:id - Get single credential metadata
pub async fn get_credential(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<(HeaderMap, Json<CredentialMetadata>), CredentialError> {
    let credential = db::get_credential(state.db_conn.clone(), &id)
        .await
        .map_err(|e| match e {
            db::CredentialDbError::NotFound => CredentialError::NotFound,
            _ => CredentialError::Internal(e.to_string()),
        })?;

    Ok((add_security_header(HeaderMap::new()), Json(credential)))
}

/// GET /api/credentials/:id/password - Retrieve password from keychain
pub async fn get_password(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<(HeaderMap, Json<PasswordResponse>), CredentialError> {
    // Get metadata first
    let credential = db::get_credential(state.db_conn.clone(), &id)
        .await
        .map_err(|e| match e {
            db::CredentialDbError::NotFound => CredentialError::NotFound,
            _ => CredentialError::Internal(e.to_string()),
        })?;

    // Retrieve password from keychain
    let password = KeyringService::get_password(
        &credential.credential_type,
        &credential.identifier,
        &credential.username,
    )
    .map_err(|e| match e {
        KeyringError::NotFound => CredentialError::InconsistentState(
            "Credential exists in database but not in keychain".to_string(),
        ),
        KeyringError::ServiceUnavailable(msg) => CredentialError::KeychainUnavailable(msg),
        KeyringError::OperationFailed(msg) => CredentialError::Internal(msg),
    })?;

    // Update last_accessed_at timestamp
    let _ = db::update_last_accessed(state.db_conn.clone(), &id).await;

    Ok((
        add_security_header(HeaderMap::new()),
        Json(PasswordResponse { password }),
    ))
}

/// PUT /api/credentials/:id - Update credential
pub async fn update_credential(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<UpdateCredentialRequest>,
) -> Result<(HeaderMap, Json<CredentialMetadata>), CredentialError> {
    // Get existing credential
    let existing = db::get_credential(state.db_conn.clone(), &id)
        .await
        .map_err(|e| match e {
            db::CredentialDbError::NotFound => CredentialError::NotFound,
            _ => CredentialError::Internal(e.to_string()),
        })?;

    // If password is being updated, update keychain
    if let Some(ref new_password) = request.password {
        KeyringService::update_password(
            &existing.credential_type,
            &existing.identifier,
            &existing.username,
            new_password,
        )
        .map_err(|e| match e {
            KeyringError::ServiceUnavailable(msg) => CredentialError::KeychainUnavailable(msg),
            _ => CredentialError::Internal(format!("Failed to update password: {}", e)),
        })?;
    }

    // Update metadata in database
    let updated = db::update_credential(
        state.db_conn.clone(),
        &id,
        request.username,
        request.service_name,
        request.port,
        request.use_tls,
        request.notes,
        request.extra_metadata,
    )
    .await
    .map_err(|e| CredentialError::Internal(e.to_string()))?;

    Ok((add_security_header(HeaderMap::new()), Json(updated)))
}

/// DELETE /api/credentials/:id - Delete credential
#[derive(Deserialize)]
pub struct DeleteQuery {
    #[serde(default)]
    hard: bool,
}

pub async fn delete_credential(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DeleteQuery>,
) -> Result<(StatusCode, HeaderMap), CredentialError> {
    // Get credential metadata first
    let credential = db::get_credential(state.db_conn.clone(), &id)
        .await
        .map_err(|e| match e {
            db::CredentialDbError::NotFound => CredentialError::NotFound,
            _ => CredentialError::Internal(e.to_string()),
        })?;

    if query.hard {
        // Hard delete: remove from keychain and database
        KeyringService::delete_password(
            &credential.credential_type,
            &credential.identifier,
            &credential.username,
        )
        .map_err(|e| match e {
            KeyringError::NotFound => {
                // Continue with database deletion even if keychain entry doesn't exist
                CredentialError::InconsistentState(
                    "Keychain entry not found, deleting database record".to_string(),
                )
            }
            KeyringError::ServiceUnavailable(msg) => CredentialError::KeychainUnavailable(msg),
            KeyringError::OperationFailed(msg) => CredentialError::Internal(msg),
        })
        .ok(); // Continue even if keychain delete fails

        db::hard_delete_credential(state.db_conn.clone(), &id)
            .await
            .map_err(|e| CredentialError::Internal(e.to_string()))?;
    } else {
        // Soft delete: just mark as inactive
        db::soft_delete_credential(state.db_conn.clone(), &id)
            .await
            .map_err(|e| match e {
                db::CredentialDbError::NotFound => CredentialError::NotFound,
                _ => CredentialError::Internal(e.to_string()),
            })?;
    }

    Ok((StatusCode::NO_CONTENT, add_security_header(HeaderMap::new())))
}
```

**File**: `/Users/brainless/Projects/dwata/dwata-api/src/handlers/mod.rs`

Add:
```rust
pub mod credentials;
```

### Phase 6: Route Registration

**File**: `/Users/brainless/Projects/dwata/dwata-api/src/main.rs`

Add routes to the router:

```rust
use crate::handlers::credentials;

// In the router setup function:
let app = Router::new()
    // ... existing routes ...
    .route("/api/credentials", post(credentials::create_credential))
    .route("/api/credentials", get(credentials::list_credentials))
    .route("/api/credentials/:id", get(credentials::get_credential))
    .route("/api/credentials/:id/password", get(credentials::get_password))
    .route("/api/credentials/:id", put(credentials::update_credential))
    .route("/api/credentials/:id", delete(credentials::delete_credential))
    .with_state(state);
```

### Phase 7: TypeScript Type Generation

**File**: `/Users/brainless/Projects/dwata/shared-types/src/bin/generate_api_types.rs`

The types will be automatically exported when running the type generation script, as all types in `credential.rs` are annotated with `#[ts(export)]`.

Run type generation:
```bash
cargo run --bin generate_api_types
```

This will generate TypeScript types in the GUI project that match the Rust types.

## Error Handling Strategy

### Error Categories and HTTP Status Codes

| Error Type | HTTP Status | Description |
|------------|-------------|-------------|
| Validation | 400 | Invalid input, missing required fields, duplicate identifier |
| Not Found | 404 | Credential doesn't exist |
| Keychain Unavailable | 503 | OS keychain service not accessible |
| Inconsistent State | 500 | Metadata exists but keychain entry missing (or vice versa) |
| Internal | 500 | Database errors, unexpected failures |

### Rollback Strategy

When creating credentials:
1. Store password in OS keychain first
2. If successful, store metadata in database
3. If database fails, **rollback**: delete from keychain
4. Return error to client

This ensures no orphaned keychain entries exist when database operations fail.

### Inconsistent State Recovery

If metadata exists but keychain entry is missing:
- `GET /api/credentials/:id/password` returns 500 with clear error message
- User must delete and recreate the credential
- Future enhancement: Add reconciliation endpoint to detect and fix inconsistencies

## Cross-Platform Considerations

### macOS
- **Keychain**: Keychain Services API (accessed via `keyring` crate)
- **Viewing Entries**: Keychain Access.app (`/Applications/Utilities/Keychain Access.app`)
- **Search for dwata credentials**: Filter by "dwata:" in keychain viewer
- **Permissions**: No special permissions required for user's default keychain

### Windows
- **Storage**: Windows Credential Manager (DPAPI-based)
- **Viewing Entries**: Control Panel → Credential Manager → Windows Credentials
- **Search for dwata credentials**: Look for items prefixed with "dwata:"
- **Permissions**: Standard user permissions sufficient

### Linux
- **Services**: Multiple backends supported via `keyring` crate:
  - GNOME Keyring (libsecret)
  - KWallet (KDE)
  - Secret Service API (org.freedesktop.secrets)
- **Viewing Entries**:
  - Seahorse (GNOME): `seahorse` command or from applications menu
  - CLI: `secret-tool search service dwata:imap`
- **Dependencies**: Requires `libsecret-1-dev` (Debian/Ubuntu) or `libsecret-devel` (Fedora/RHEL)
- **Permissions**: Desktop session must have keyring daemon running

### Testing on Each Platform

**macOS**:
```bash
# Create credential via API
curl -X POST http://localhost:3000/api/credentials -H "Content-Type: application/json" -d '{
  "credential_type": "imap",
  "identifier": "test_email",
  "username": "test@example.com",
  "password": "test_password_123"
}'

# Verify in Keychain Access
open "/Applications/Utilities/Keychain Access.app"
# Search for "dwata:imap"
```

**Linux**:
```bash
# Install secret-tool (if needed)
sudo apt-get install libsecret-tools  # Debian/Ubuntu
sudo dnf install libsecret             # Fedora

# Create credential via API (same as above)

# Verify with secret-tool
secret-tool search service dwata:imap

# Retrieve password
secret-tool lookup service dwata:imap username test_email:test@example.com
```

**Windows**:
```powershell
# Create credential via API (same as above)

# Verify in Credential Manager
control /name Microsoft.CredentialManager
# Look for "dwata:imap" entries
```

## Testing Strategy

### Manual Testing Checklist

1. **Create Credential**
   ```bash
   curl -X POST http://localhost:3000/api/credentials \
     -H "Content-Type: application/json" \
     -d '{
       "credential_type": "imap",
       "identifier": "work_email",
       "username": "john@company.com",
       "password": "secure_password_123",
       "service_name": "imap.gmail.com",
       "port": 993,
       "use_tls": true,
       "notes": "Work email account"
     }'
   ```
   **Expected**: 201 Created, returns metadata (no password)

2. **List Credentials**
   ```bash
   curl http://localhost:3000/api/credentials
   ```
   **Expected**: 200 OK, array of credentials (no passwords)

3. **Get Single Credential**
   ```bash
   curl http://localhost:3000/api/credentials/cred_abc123xyz
   ```
   **Expected**: 200 OK, credential metadata

4. **Retrieve Password**
   ```bash
   curl http://localhost:3000/api/credentials/cred_abc123xyz/password
   ```
   **Expected**: 200 OK, `{"password": "secure_password_123"}`
   **Header**: `X-Security-Warning` present

5. **Update Credential**
   ```bash
   curl -X PUT http://localhost:3000/api/credentials/cred_abc123xyz \
     -H "Content-Type: application/json" \
     -d '{
       "username": "jane@company.com",
       "password": "new_password_456",
       "notes": "Updated username and password"
     }'
   ```
   **Expected**: 200 OK, updated metadata

6. **Soft Delete**
   ```bash
   curl -X DELETE http://localhost:3000/api/credentials/cred_abc123xyz
   ```
   **Expected**: 204 No Content
   **Verify**: Credential still in keychain, but `is_active = false` in DB

7. **Hard Delete**
   ```bash
   curl -X DELETE http://localhost:3000/api/credentials/cred_abc123xyz?hard=true
   ```
   **Expected**: 204 No Content
   **Verify**: Credential removed from both keychain and DB

### Error Scenario Testing

1. **Duplicate Identifier**
   - Create credential with identifier "work_email"
   - Try to create another with same identifier
   - **Expected**: 400 Bad Request, "A credential with this identifier already exists"

2. **Not Found**
   - Request non-existent credential ID
   - **Expected**: 404 Not Found

3. **Invalid Credential Type**
   - Create credential with invalid type
   - **Expected**: 400 Bad Request (serde deserialization error)

4. **Empty Required Fields**
   - Create credential with empty identifier, username, or password
   - **Expected**: 400 Bad Request, validation error

5. **Inconsistent State** (manual test)
   - Create credential via API
   - Manually delete from OS keychain
   - Try to retrieve password via API
   - **Expected**: 500 Internal Server Error, "Credential exists in database but not in keychain"

### OS Keychain Verification

After creating credentials, verify they exist in OS keychain:

**macOS**:
```bash
# Open Keychain Access
open "/Applications/Utilities/Keychain Access.app"
# Search for "dwata:imap"
# Verify entry: "dwata:imap" with username "work_email:john@company.com"
```

**Linux**:
```bash
secret-tool search service dwata:imap
# Should show: attribute.username = work_email:john@company.com

secret-tool lookup service dwata:imap username work_email:john@company.com
# Should output: secure_password_123
```

**Windows**:
```powershell
# Open Credential Manager
control /name Microsoft.CredentialManager
# Look for "dwata:imap" in Windows Credentials
# Username should be "work_email:john@company.com"
```

### Integration Testing

Create a test script that:
1. Creates multiple credentials of different types
2. Lists and verifies all exist
3. Retrieves passwords and verifies correctness
4. Updates credentials and verifies changes
5. Soft deletes and verifies `is_active = false`
6. Hard deletes and verifies removal from keychain

Example test script (`test_credentials.sh`):
```bash
#!/bin/bash

API_URL="http://localhost:3000/api/credentials"

# Create IMAP credential
echo "Creating IMAP credential..."
IMAP_RESPONSE=$(curl -s -X POST $API_URL -H "Content-Type: application/json" -d '{
  "credential_type": "imap",
  "identifier": "test_imap",
  "username": "test@example.com",
  "password": "imap_password"
}')
IMAP_ID=$(echo $IMAP_RESPONSE | jq -r '.id')
echo "Created IMAP credential: $IMAP_ID"

# Create SMTP credential
echo "Creating SMTP credential..."
SMTP_RESPONSE=$(curl -s -X POST $API_URL -H "Content-Type: application/json" -d '{
  "credential_type": "smtp",
  "identifier": "test_smtp",
  "username": "test@example.com",
  "password": "smtp_password"
}')
SMTP_ID=$(echo $SMTP_RESPONSE | jq -r '.id')
echo "Created SMTP credential: $SMTP_ID"

# List credentials
echo "Listing credentials..."
curl -s $API_URL | jq

# Retrieve passwords
echo "Retrieving IMAP password..."
curl -s $API_URL/$IMAP_ID/password | jq

echo "Retrieving SMTP password..."
curl -s $API_URL/$SMTP_ID/password | jq

# Update IMAP credential
echo "Updating IMAP credential..."
curl -s -X PUT $API_URL/$IMAP_ID -H "Content-Type: application/json" -d '{
  "password": "new_imap_password",
  "notes": "Updated password"
}' | jq

# Verify updated password
echo "Verifying updated password..."
curl -s $API_URL/$IMAP_ID/password | jq

# Soft delete SMTP credential
echo "Soft deleting SMTP credential..."
curl -s -X DELETE $API_URL/$SMTP_ID
echo "Soft delete complete"

# List credentials (should not show soft-deleted)
echo "Listing active credentials..."
curl -s $API_URL | jq

# Hard delete IMAP credential
echo "Hard deleting IMAP credential..."
curl -s -X DELETE "$API_URL/$IMAP_ID?hard=true"
echo "Hard delete complete"

# List credentials (should be empty)
echo "Listing all credentials..."
curl -s $API_URL | jq

echo "Test complete!"
```

## Success Criteria

### Implementation Complete When:

1. ✅ All dependencies added to `Cargo.toml`
2. ✅ Shared types created and exported
3. ✅ Database schema created with migration
4. ✅ Keyring service module implemented
5. ✅ Database operations module implemented
6. ✅ HTTP handlers implemented
7. ✅ Routes registered in main.rs
8. ✅ TypeScript types generated
9. ✅ All manual tests pass
10. ✅ Credentials verified in OS keychain tools on target platform(s)

### Quality Checklist:

- [ ] Error handling covers all failure scenarios
- [ ] Rollback logic prevents orphaned keychain entries
- [ ] Security warning header present on all responses
- [ ] `last_accessed_at` updated when passwords retrieved
- [ ] Soft delete doesn't remove keychain entries
- [ ] Hard delete removes from both DB and keychain
- [ ] TypeScript types match Rust types exactly
- [ ] Code follows repository conventions
- [ ] No passwords logged or exposed in error messages

## Future Enhancements

### Phase 2 Features (After Initial Implementation)

1. **Authentication & Authorization**
   - Add API key authentication
   - Implement role-based access control
   - Add user/session management

2. **Audit Logging**
   - Log all credential access (who, when, what)
   - Store audit logs in separate table
   - Add audit log query endpoints

3. **Advanced Features**
   - Credential sharing between users
   - Credential rotation reminders
   - Import/export (encrypted)
   - Credential health checks (test IMAP/SMTP connections)

4. **Reconciliation Tools**
   - Endpoint to detect inconsistencies
   - Automatic recovery from inconsistent state
   - Bulk operations (delete all inactive, etc.)

5. **GUI Integration**
   - React components for credential management
   - Visual keychain status indicators
   - Inline credential testing

## Additional Resources

### Dependencies Documentation
- **keyring crate**: https://docs.rs/keyring/latest/keyring/
- **DuckDB Rust**: https://docs.rs/duckdb/latest/duckdb/

### Platform-Specific Keychain Docs
- **macOS Keychain**: https://developer.apple.com/documentation/security/keychain_services
- **Windows Credential Manager**: https://docs.microsoft.com/en-us/windows/win32/secauthn/credential-manager
- **Linux Secret Service**: https://specifications.freedesktop.org/secret-service/

### Security Best Practices
- OWASP Credential Storage Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
- NIST Password Guidelines: https://pages.nist.gov/800-63-3/

---

**Document Version**: 1.0
**Created**: 2026-01-28
**Status**: Ready for Implementation
