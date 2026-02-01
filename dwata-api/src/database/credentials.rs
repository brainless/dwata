use duckdb::Connection;
use shared_types::credential::{CredentialMetadata, CredentialType, CreateCredentialRequest};
use std::sync::Arc;
use tokio::sync::Mutex;

pub type DbConnection = Arc<std::sync::Mutex<Connection>>;
pub type AsyncDbConnection = Arc<Mutex<Connection>>;

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

pub async fn insert_credential(
    conn: AsyncDbConnection,
    request: &CreateCredentialRequest,
) -> Result<CredentialMetadata, CredentialDbError> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    let mut stmt = conn
        .prepare("SELECT COUNT(*) FROM credentials_metadata WHERE identifier = ?")
        .map_err(|e| CredentialDbError::DatabaseError(e.to_string()))?;

    let count: i64 = stmt
        .query_row([&request.identifier], |row| row.get(0))
        .map_err(|e| CredentialDbError::DatabaseError(e.to_string()))?;

    if count > 0 {
        return Err(CredentialDbError::DuplicateIdentifier);
    }

    let id: i64 = conn
        .query_row(
            "INSERT INTO credentials_metadata
             (credential_type, identifier, username, service_name, port, use_tls, notes,
              created_at, updated_at, is_active, extra_metadata)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING id",
            duckdb::params![
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
            |row| row.get(0),
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

pub async fn list_credentials(
    conn: AsyncDbConnection,
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
                "localfile" => CredentialType::LocalFile,
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

pub async fn get_credential(
    conn: AsyncDbConnection,
    id: i64,
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

pub async fn update_last_accessed(
    conn: AsyncDbConnection,
    id: i64,
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

pub async fn update_credential(
    conn: AsyncDbConnection,
    id: i64,
    username: Option<String>,
    service_name: Option<String>,
    port: Option<i32>,
    use_tls: Option<bool>,
    notes: Option<String>,
    extra_metadata: Option<String>,
) -> Result<CredentialMetadata, CredentialDbError> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    let mut updates = vec!["updated_at = ?".to_string()];
    let mut params: Vec<Box<dyn duckdb::ToSql>> = vec![Box::new(now)];

    if username.is_some() {
        updates.push("username = ?".to_string());
        params.push(Box::new(username.clone()));
    }
    if service_name.is_some() {
        updates.push("service_name = ?".to_string());
        params.push(Box::new(service_name.clone()));
    }
    if port.is_some() {
        updates.push("port = ?".to_string());
        params.push(Box::new(port));
    }
    if use_tls.is_some() {
        updates.push("use_tls = ?".to_string());
        params.push(Box::new(use_tls));
    }
    if notes.is_some() {
        updates.push("notes = ?".to_string());
        params.push(Box::new(notes.clone()));
    }
    if extra_metadata.is_some() {
        updates.push("extra_metadata = ?".to_string());
        params.push(Box::new(extra_metadata.clone()));
    }

    params.push(Box::new(id));

    let query = format!(
        "UPDATE credentials_metadata SET {} WHERE id = ?",
        updates.join(", ")
    );

    let params_refs: Vec<&dyn duckdb::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    conn.execute(&query, params_refs.as_slice())
        .map_err(|e| CredentialDbError::DatabaseError(e.to_string()))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, credential_type, identifier, username, service_name, port, use_tls, notes,
                    created_at, updated_at, last_accessed_at, is_active, extra_metadata
             FROM credentials_metadata
             WHERE id = ?",
        )
        .map_err(|e| CredentialDbError::DatabaseError(e.to_string()))?;

    let credential = stmt.query_row([id], |row| {
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
    })?;

    drop(conn);
    Ok(credential)
}

pub async fn soft_delete_credential(
    conn: AsyncDbConnection,
    id: i64,
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

pub async fn hard_delete_credential(
    conn: AsyncDbConnection,
    id: i64,
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
