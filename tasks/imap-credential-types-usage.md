# IMAP Credential Types - Usage Examples

## Overview

The credential storage system now has typed support for IMAP (and other service-specific) settings. This provides type safety from the database through to the GUI.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         Frontend (TypeScript)               │
│                                                             │
│  CreateImapCredentialRequest → API                          │
│  ImapAccountSettings (type-safe)                            │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                         API (Rust)                          │
│                                                             │
│  CreateImapCredentialRequest → CreateCredentialRequest      │
│  ImapAccountSettings → JSON → extra_metadata field          │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                      Database (DuckDB)                      │
│                                                             │
│  credentials_metadata table:                                │
│    - service_name: "imap.gmail.com" (host)                  │
│    - port: 993                                              │
│    - use_tls: true                                          │
│    - extra_metadata: '{"auth_method":"plain",...}' (JSON)   │
└─────────────────────────────────────────────────────────────┘
```

## TypeScript Usage (Frontend)

### Creating an IMAP Credential

```typescript
import {
  CreateImapCredentialRequest,
  ImapAccountSettings,
  ImapAuthMethod
} from '@/api-types/types';

// Type-safe IMAP settings
const imapSettings: ImapAccountSettings = {
  host: "imap.gmail.com",
  port: 993,
  use_tls: true,
  auth_method: "plain", // ImapAuthMethod enum
  default_mailbox: "INBOX",
  connection_timeout_secs: 30,
  validate_certs: true,
};

// Type-safe request
const request: CreateImapCredentialRequest = {
  identifier: "work_email",
  username: "john@company.com",
  password: "secure_password_123",
  settings: imapSettings,
  notes: "Work email account",
};

// Send to API
const response = await fetch('/api/credentials/imap', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify(request),
});
```

### Parsing IMAP Credential Metadata

```typescript
import {
  CredentialMetadata,
  ImapCredentialMetadata
} from '@/api-types/types';

// Get generic credential
const credential: CredentialMetadata = await fetch('/api/credentials/cred_123')
  .then(r => r.json());

// Parse IMAP-specific settings from extra_metadata
const imapSettings: ImapAccountSettings = JSON.parse(
  credential.extra_metadata || '{}'
);

// Or use the dedicated ImapCredentialMetadata type
const imapCredential: ImapCredentialMetadata = {
  id: credential.id,
  identifier: credential.identifier,
  username: credential.username,
  settings: imapSettings,
  notes: credential.notes,
  created_at: credential.created_at,
  updated_at: credential.updated_at,
  last_accessed_at: credential.last_accessed_at,
  is_active: credential.is_active,
};
```

## Rust Usage (Backend)

### Creating IMAP Credential via API

```rust
use shared_types::{
    CreateImapCredentialRequest,
    ImapAccountSettings,
    ImapAuthMethod
};

// Handler for IMAP-specific endpoint (optional convenience)
#[axum::debug_handler]
pub async fn create_imap_credential(
    State(state): State<AppState>,
    Json(request): Json<CreateImapCredentialRequest>,
) -> Result<(StatusCode, Json<CredentialMetadata>), CredentialError> {
    // Convert to generic request
    let generic_request = request.into_generic()
        .map_err(|e| CredentialError::Validation(format!("Invalid IMAP settings: {}", e)))?;

    // Use existing credential creation logic
    create_credential(State(state), Json(generic_request)).await
}
```

### Parsing IMAP Settings from Database

```rust
use shared_types::{CredentialMetadata, ImapAccountSettings, ImapCredentialMetadata};

// Get credential from database
let credential: CredentialMetadata = db::get_credential(conn, "cred_123").await?;

// Parse IMAP settings (type-safe)
let imap_settings: ImapAccountSettings = credential.parse_imap_settings()
    .map_err(|e| format!("Failed to parse IMAP settings: {}", e))?;

println!("Host: {}", imap_settings.host);
println!("Port: {}", imap_settings.port);
println!("Auth method: {:?}", imap_settings.auth_method);

// Or convert to ImapCredentialMetadata
let imap_credential = ImapCredentialMetadata::from_generic(credential)?;
```

### Using IMAP Settings to Connect

```rust
use shared_types::ImapAccountSettings;

async fn connect_to_imap(
    settings: &ImapAccountSettings,
    username: &str,
    password: &str,
) -> Result<ImapSession, ImapError> {
    let client = if settings.use_tls {
        // Connect with TLS
        imap::ClientBuilder::new(&settings.host, settings.port)
            .connect()
            .map_err(|e| ImapError::ConnectionFailed(e.to_string()))?
    } else {
        // Connect without TLS (not recommended)
        imap::ClientBuilder::new(&settings.host, settings.port)
            .starttls()
            .connect()
            .map_err(|e| ImapError::ConnectionFailed(e.to_string()))?
    };

    // Authenticate based on auth method
    let session = match settings.auth_method {
        ImapAuthMethod::Plain => {
            client.login(username, password)
                .map_err(|e| ImapError::AuthFailed(e.to_string()))?
        }
        ImapAuthMethod::OAuth2 | ImapAuthMethod::Xoauth2 => {
            // OAuth2 authentication
            client.authenticate("XOAUTH2", |_challenge| {
                Ok(format!("user={}\x01auth=Bearer {}\x01\x01", username, password))
            })
            .map_err(|e| ImapError::AuthFailed(e.to_string()))?
        }
    };

    Ok(session)
}
```

## Database Schema

The IMAP settings are stored using a hybrid approach:

### Common Fields (Dedicated Columns)
```sql
CREATE TABLE credentials_metadata (
    id VARCHAR PRIMARY KEY,
    credential_type VARCHAR NOT NULL,           -- "imap"
    identifier VARCHAR NOT NULL UNIQUE,         -- "work_email"
    username VARCHAR NOT NULL,                  -- "john@company.com"
    service_name VARCHAR,                       -- "imap.gmail.com" (host)
    port INTEGER,                               -- 993
    use_tls BOOLEAN DEFAULT TRUE,               -- true
    notes VARCHAR,                              -- "Work email account"
    extra_metadata VARCHAR,                     -- JSON with additional fields
    -- ... timestamps, etc.
);
```

### IMAP-Specific Fields (JSON in extra_metadata)
```json
{
  "auth_method": "plain",
  "default_mailbox": "INBOX",
  "connection_timeout_secs": 30,
  "validate_certs": true
}
```

## Benefits

1. **Type Safety**: Compile-time checks in both Rust and TypeScript
2. **Consistency**: Same types used across frontend, backend, and database
3. **Auto-completion**: IDEs provide suggestions for all fields
4. **Documentation**: Doc comments from Rust appear in TypeScript types
5. **Validation**: Invalid settings caught at compile time, not runtime
6. **Extensibility**: Easy to add new service-specific settings types

## Other Service Types

Similar patterns for SMTP and API keys:

### SMTP
```typescript
const smtpSettings: SmtpAccountSettings = {
  host: "smtp.gmail.com",
  port: 587,
  use_tls: true,
  connection_timeout_secs: 30,
};
```

### API Keys
```typescript
const apiKeySettings: ApiKeySettings = {
  base_url: "https://api.stripe.com",
  api_version: "2023-10-16",
  timeout_secs: 30,
};
```

## Next Steps for IMAP Integration

When building the IMAP inbox ingestion feature:

1. **Fetch IMAP credentials**:
   ```rust
   let credentials = db::list_credentials(conn, false).await?;
   let imap_creds: Vec<_> = credentials.into_iter()
       .filter(|c| matches!(c.credential_type, CredentialType::Imap))
       .collect();
   ```

2. **Parse settings for each credential**:
   ```rust
   for credential in imap_creds {
       let settings = credential.parse_imap_settings()?;
       let password = KeyringService::get_password(
           &credential.credential_type,
           &credential.identifier,
           &credential.username
       )?;

       // Connect and fetch emails
       let session = connect_to_imap(&settings, &credential.username, &password).await?;
       // ... process mailbox
   }
   ```

3. **Use the settings**:
   - Host/port for connection
   - auth_method for authentication type
   - default_mailbox for which folder to sync
   - connection_timeout_secs for timeout handling
   - validate_certs for SSL certificate validation

## Type Generation

To regenerate TypeScript types after modifying Rust types:

```bash
cd shared-types
cargo run --bin generate_api_types
```

This exports all types marked with `#[ts(export)]` to `../gui/src/api-types/types.ts`.
