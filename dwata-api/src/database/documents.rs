use anyhow::{anyhow, Result};
use rusqlite::{params, params_from_iter, OptionalExtension};
use shared_types::{
    Document, DocumentCursor, DocumentKind, DocumentSortBy, DocumentSource, DocumentSourceType,
    ListDocumentsRequest, ListDocumentsResponse, SourceAccessState, SourcePermissionState,
};
use tokio::task;

use crate::database::AsyncDbConnection;
use crate::search::tantivy::IndexedTextFields;

fn parse_document_kind(value: &str) -> Result<DocumentKind> {
    match value {
        "email" => Ok(DocumentKind::Email),
        "attachment" => Ok(DocumentKind::Attachment),
        "file" => Ok(DocumentKind::File),
        _ => Err(anyhow!("Invalid document kind: {}", value)),
    }
}

fn document_kind_to_db(kind: &DocumentKind) -> &'static str {
    match kind {
        DocumentKind::Email => "email",
        DocumentKind::Attachment => "attachment",
        DocumentKind::File => "file",
    }
}

fn parse_source_type(value: &str) -> Result<DocumentSourceType> {
    match value {
        "imap-account" => Ok(DocumentSourceType::ImapAccount),
        "local-folder" => Ok(DocumentSourceType::LocalFolder),
        "cloud-drive" => Ok(DocumentSourceType::CloudDrive),
        "cloud-mailbox" => Ok(DocumentSourceType::CloudMailbox),
        "manual-import" => Ok(DocumentSourceType::ManualImport),
        _ => Err(anyhow!("Invalid source type: {}", value)),
    }
}

fn source_type_to_db(source_type: &DocumentSourceType) -> &'static str {
    match source_type {
        DocumentSourceType::ImapAccount => "imap-account",
        DocumentSourceType::LocalFolder => "local-folder",
        DocumentSourceType::CloudDrive => "cloud-drive",
        DocumentSourceType::CloudMailbox => "cloud-mailbox",
        DocumentSourceType::ManualImport => "manual-import",
    }
}

fn parse_access_state(value: &str) -> Result<SourceAccessState> {
    match value {
        "accessible" => Ok(SourceAccessState::Accessible),
        "offline" => Ok(SourceAccessState::Offline),
        "unreachable" => Ok(SourceAccessState::Unreachable),
        "disabled" => Ok(SourceAccessState::Disabled),
        "unknown" => Ok(SourceAccessState::Unknown),
        _ => Err(anyhow!("Invalid source access state: {}", value)),
    }
}

fn access_state_to_db(access_state: &SourceAccessState) -> &'static str {
    match access_state {
        SourceAccessState::Accessible => "accessible",
        SourceAccessState::Offline => "offline",
        SourceAccessState::Unreachable => "unreachable",
        SourceAccessState::Disabled => "disabled",
        SourceAccessState::Unknown => "unknown",
    }
}

fn parse_permission_state(value: &str) -> Result<SourcePermissionState> {
    match value {
        "granted" => Ok(SourcePermissionState::Granted),
        "expired" => Ok(SourcePermissionState::Expired),
        "revoked" => Ok(SourcePermissionState::Revoked),
        "insufficient-scope" => Ok(SourcePermissionState::InsufficientScope),
        "forbidden" => Ok(SourcePermissionState::Forbidden),
        "unknown" => Ok(SourcePermissionState::Unknown),
        _ => Err(anyhow!("Invalid source permission state: {}", value)),
    }
}

fn permission_state_to_db(permission_state: &SourcePermissionState) -> &'static str {
    match permission_state {
        SourcePermissionState::Granted => "granted",
        SourcePermissionState::Expired => "expired",
        SourcePermissionState::Revoked => "revoked",
        SourcePermissionState::InsufficientScope => "insufficient-scope",
        SourcePermissionState::Forbidden => "forbidden",
        SourcePermissionState::Unknown => "unknown",
    }
}

fn map_document_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Document> {
    let kind_raw: String = row.get(2)?;
    let kind = parse_document_kind(&kind_raw).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            )),
        )
    })?;

    Ok(Document {
        id: row.get(0)?,
        source_id: row.get(1)?,
        kind,
        parent_document_id: row.get(3)?,
        email_id: row.get(4)?,
        attachment_id: row.get(5)?,
        title: row.get(6)?,
        canonical_name: row.get(7)?,
        mime_type: row.get(8)?,
        size_bytes: row.get(9)?,
        checksum_sha256: row.get(10)?,
        storage_path: row.get(11)?,
        external_uri: row.get(12)?,
        date_created: row.get(13)?,
        date_modified: row.get(14)?,
        date_received: row.get(15)?,
        indexed_at: row.get(16)?,
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
    })
}

pub async fn list_documents(
    conn: AsyncDbConnection,
    request: ListDocumentsRequest,
) -> Result<ListDocumentsResponse> {
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let limit = request.limit.unwrap_or(50);
        let sort_by = request.sort_by.unwrap_or(DocumentSortBy::ReceivedAtDesc);

        let sort_expr = match sort_by {
            DocumentSortBy::ReceivedAtDesc => {
                "COALESCE(d.date_received, d.date_modified, d.date_created, d.created_at, 0)"
            }
            DocumentSortBy::ModifiedAtDesc => {
                "COALESCE(d.date_modified, d.date_created, d.created_at, 0)"
            }
            DocumentSortBy::CreatedAtDesc => "COALESCE(d.date_created, d.created_at, 0)",
        };

        let mut query = format!(
            "SELECT d.id, d.source_id, d.kind, d.parent_document_id, d.email_id, d.attachment_id,
                    d.title, d.canonical_name, d.mime_type, d.size_bytes, d.checksum_sha256,
                    d.storage_path, d.external_uri, d.date_created, d.date_modified,
                    d.date_received, d.indexed_at, d.created_at, d.updated_at
             FROM documents d
             JOIN document_sources s ON s.id = d.source_id
             WHERE (?1 IS NULL OR d.source_id = ?1)
               AND (?2 IS NULL OR s.credential_id = ?2)
               AND (?3 IS NULL OR d.kind = ?3)
               AND (?4 IS NULL OR d.parent_document_id = ?4)"
        );

        let mut values = vec![
            rusqlite::types::Value::from(request.source_id),
            rusqlite::types::Value::from(request.credential_id),
            rusqlite::types::Value::from(
                request
                    .kind
                    .as_ref()
                    .map(|k| document_kind_to_db(k).to_string()),
            ),
            rusqlite::types::Value::from(request.parent_document_id),
        ];

        if let Some(cursor) = request.cursor {
            query.push_str(&format!(
                " AND ({} < ?5 OR ({} = ?6 AND d.id < ?7))",
                sort_expr, sort_expr
            ));
            values.push(rusqlite::types::Value::from(cursor.sort_value));
            values.push(rusqlite::types::Value::from(cursor.sort_value));
            values.push(rusqlite::types::Value::from(cursor.id));
            query.push_str(&format!(" ORDER BY {} DESC, d.id DESC LIMIT ?8", sort_expr));
        } else {
            query.push_str(&format!(" ORDER BY {} DESC, d.id DESC LIMIT ?5", sort_expr));
        }

        values.push(rusqlite::types::Value::from((limit + 1) as i64));

        let mut stmt = conn.prepare(&query)?;
        let mut docs = stmt
            .query_map(params_from_iter(values), map_document_row)?
            .collect::<Result<Vec<_>, _>>()?;

        let has_more = docs.len() > limit;
        if has_more {
            docs.truncate(limit);
        }

        let next_cursor = if has_more {
            docs.last().map(|d| {
                let sort_value = match sort_by {
                    DocumentSortBy::ReceivedAtDesc => d
                        .date_received
                        .or(d.date_modified)
                        .or(d.date_created)
                        .unwrap_or(d.created_at),
                    DocumentSortBy::ModifiedAtDesc => {
                        d.date_modified.or(d.date_created).unwrap_or(d.created_at)
                    }
                    DocumentSortBy::CreatedAtDesc => d.date_created.unwrap_or(d.created_at),
                };
                DocumentCursor {
                    sort_value,
                    id: d.id,
                }
            })
        } else {
            None
        };

        Ok(ListDocumentsResponse {
            documents: docs,
            next_cursor,
            has_more,
        })
    })
    .await?
}

pub async fn get_document(conn: AsyncDbConnection, id: i64) -> Result<Option<Document>> {
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let mut stmt = conn.prepare(
            "SELECT id, source_id, kind, parent_document_id, email_id, attachment_id,
                    title, canonical_name, mime_type, size_bytes, checksum_sha256,
                    storage_path, external_uri, date_created, date_modified,
                    date_received, indexed_at, created_at, updated_at
             FROM documents
             WHERE id = ?1",
        )?;

        let document = stmt.query_row([id], map_document_row).optional()?;
        Ok(document)
    })
    .await?
}

pub async fn get_documents_by_ids(conn: AsyncDbConnection, ids: &[i64]) -> Result<Vec<Document>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let ids = ids.to_vec();
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let placeholders = std::iter::repeat("?")
            .take(ids.len())
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!(
            "SELECT id, source_id, kind, parent_document_id, email_id, attachment_id,
                    title, canonical_name, mime_type, size_bytes, checksum_sha256,
                    storage_path, external_uri, date_created, date_modified,
                    date_received, indexed_at, created_at, updated_at
             FROM documents
             WHERE id IN ({})",
            placeholders
        );

        let mut stmt = conn.prepare(&query)?;
        let docs = stmt
            .query_map(params_from_iter(ids), map_document_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(docs)
    })
    .await?
}

#[derive(Debug, Clone)]
pub struct DocumentForIndexing {
    pub document: Document,
    pub indexed_text: IndexedTextFields,
}

pub async fn list_documents_for_indexing_page(
    conn: AsyncDbConnection,
    after_id: i64,
    limit: usize,
) -> Result<Vec<DocumentForIndexing>> {
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let mut stmt = conn.prepare(
            "SELECT d.id, d.source_id, d.kind, d.parent_document_id, d.email_id, d.attachment_id,
                    d.title, d.canonical_name, d.mime_type, d.size_bytes, d.checksum_sha256,
                    d.storage_path, d.external_uri, d.date_created, d.date_modified,
                    d.date_received, d.indexed_at, d.created_at, d.updated_at,
                    e.subject, e.from_address, e.body_text
             FROM documents d
             LEFT JOIN emails e ON e.id = d.email_id
             WHERE d.id > ?1
             ORDER BY d.id ASC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![after_id, limit as i64], |row| {
            let document = map_document_row(row)?;
            let subject: Option<String> = row.get(19)?;
            let from_address: Option<String> = row.get(20)?;
            let body_text: Option<String> = row.get(21)?;
            Ok(DocumentForIndexing {
                document,
                indexed_text: IndexedTextFields {
                    title: subject,
                    from_address,
                    body_text,
                    attachment_text: None,
                },
            })
        })?;

        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    })
    .await?
}

pub async fn upsert_document_source(
    conn: AsyncDbConnection,
    source: &DocumentSource,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    let id: i64 = conn.query_row(
        "INSERT INTO document_sources (
            id, source_type, display_name, credential_id, root_reference,
            access_state, permission_state, access_checked_at, permission_checked_at, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(id) DO UPDATE SET
            source_type = excluded.source_type,
            display_name = excluded.display_name,
            credential_id = excluded.credential_id,
            root_reference = excluded.root_reference,
            access_state = excluded.access_state,
            permission_state = excluded.permission_state,
            access_checked_at = excluded.access_checked_at,
            permission_checked_at = excluded.permission_checked_at,
            updated_at = excluded.updated_at
         RETURNING id",
        params![
            source.id,
            source_type_to_db(&source.source_type),
            source.display_name,
            source.credential_id,
            source.root_reference,
            access_state_to_db(&source.access_state),
            permission_state_to_db(&source.permission_state),
            source.access_checked_at,
            source.permission_checked_at,
            source.created_at,
            now,
        ],
        |row| row.get(0),
    )?;

    Ok(id)
}

pub async fn upsert_document(conn: AsyncDbConnection, document: &Document) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp_millis();

    let id: i64 = conn.query_row(
        "INSERT INTO documents (
            id, source_id, kind, parent_document_id, email_id, attachment_id, title, canonical_name,
            mime_type, size_bytes, checksum_sha256, storage_path, external_uri, date_created,
            date_modified, date_received, indexed_at, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
         ON CONFLICT(id) DO UPDATE SET
            source_id = excluded.source_id,
            kind = excluded.kind,
            parent_document_id = excluded.parent_document_id,
            email_id = excluded.email_id,
            attachment_id = excluded.attachment_id,
            title = excluded.title,
            canonical_name = excluded.canonical_name,
            mime_type = excluded.mime_type,
            size_bytes = excluded.size_bytes,
            checksum_sha256 = excluded.checksum_sha256,
            storage_path = excluded.storage_path,
            external_uri = excluded.external_uri,
            date_created = excluded.date_created,
            date_modified = excluded.date_modified,
            date_received = excluded.date_received,
            indexed_at = excluded.indexed_at,
            updated_at = excluded.updated_at
         RETURNING id",
        params![
            document.id,
            document.source_id,
            document_kind_to_db(&document.kind),
            document.parent_document_id,
            document.email_id,
            document.attachment_id,
            document.title,
            document.canonical_name,
            document.mime_type,
            document.size_bytes,
            document.checksum_sha256,
            document.storage_path,
            document.external_uri,
            document.date_created,
            document.date_modified,
            document.date_received,
            document.indexed_at,
            document.created_at,
            now,
        ],
        |row| row.get(0),
    )?;

    Ok(id)
}

#[allow(dead_code)]
pub async fn get_document_source(
    conn: AsyncDbConnection,
    id: i64,
) -> Result<Option<DocumentSource>> {
    task::spawn_blocking(move || {
        let conn = conn.get_blocking();
        let mut stmt = conn.prepare(
            "SELECT id, source_type, display_name, credential_id, root_reference, access_state,
                    permission_state, access_checked_at, permission_checked_at, created_at, updated_at
             FROM document_sources
             WHERE id = ?1",
        )?;

        stmt.query_row([id], |row| {
            Ok(DocumentSource {
                id: row.get(0)?,
                source_type: parse_source_type(&row.get::<_, String>(1)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            e.to_string(),
                        )),
                    )
                })?,
                display_name: row.get(2)?,
                credential_id: row.get(3)?,
                root_reference: row.get(4)?,
                access_state: parse_access_state(&row.get::<_, String>(5)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        5,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            e.to_string(),
                        )),
                    )
                })?,
                permission_state: parse_permission_state(&row.get::<_, String>(6)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        6,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            e.to_string(),
                        )),
                    )
                })?,
                access_checked_at: row.get(7)?,
                permission_checked_at: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })
        .optional()
        .map_err(Into::into)
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::AsyncDbConnection;
    use r2d2::Pool;
    use r2d2_sqlite::SqliteConnectionManager;
    use rusqlite::Connection;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_db_path(name: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("dwata-{}-{}.sqlite", name, ts))
    }

    fn doc(
        id: i64,
        source_id: i64,
        date_received: Option<i64>,
        date_modified: Option<i64>,
        date_created: Option<i64>,
        created_at: i64,
        kind: DocumentKind,
    ) -> Document {
        Document {
            id,
            source_id,
            kind,
            parent_document_id: None,
            email_id: None,
            attachment_id: None,
            title: None,
            canonical_name: None,
            mime_type: None,
            size_bytes: None,
            checksum_sha256: None,
            storage_path: Some(format!("p-{}", id)),
            external_uri: None,
            date_created,
            date_modified,
            date_received,
            indexed_at: None,
            created_at,
            updated_at: created_at,
        }
    }

    fn test_async_conn(name: &str) -> AsyncDbConnection {
        let path = test_db_path(name);
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS document_sources (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_type VARCHAR NOT NULL,
                display_name VARCHAR NOT NULL,
                credential_id INTEGER,
                root_reference VARCHAR,
                access_state VARCHAR NOT NULL,
                permission_state VARCHAR NOT NULL,
                access_checked_at BIGINT,
                permission_checked_at BIGINT,
                created_at BIGINT NOT NULL,
                updated_at BIGINT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS documents (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_id INTEGER NOT NULL,
                kind VARCHAR NOT NULL,
                parent_document_id INTEGER,
                email_id INTEGER,
                attachment_id INTEGER,
                title VARCHAR,
                canonical_name VARCHAR,
                mime_type VARCHAR,
                size_bytes BIGINT,
                checksum_sha256 VARCHAR,
                storage_path VARCHAR,
                external_uri VARCHAR,
                date_created BIGINT,
                date_modified BIGINT,
                date_received BIGINT,
                indexed_at BIGINT,
                created_at BIGINT NOT NULL,
                updated_at BIGINT NOT NULL
             );",
        )
        .unwrap();
        drop(conn);

        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::builder().max_size(4).build(manager).unwrap();
        AsyncDbConnection::new(pool)
    }

    #[tokio::test]
    async fn keyset_pagination_is_stable_with_tie_break() {
        let async_conn = test_async_conn("documents-keyset");

        upsert_document_source(
            async_conn.clone(),
            &DocumentSource {
                id: 1,
                source_type: DocumentSourceType::ImapAccount,
                display_name: "imap".to_string(),
                credential_id: Some(10),
                root_reference: None,
                access_state: SourceAccessState::Accessible,
                permission_state: SourcePermissionState::Granted,
                access_checked_at: None,
                permission_checked_at: None,
                created_at: 1,
                updated_at: 1,
            },
        )
        .await
        .unwrap();

        upsert_document(
            async_conn.clone(),
            &doc(100, 1, Some(1000), None, None, 100, DocumentKind::Email),
        )
        .await
        .unwrap();
        upsert_document(
            async_conn.clone(),
            &doc(99, 1, Some(1000), None, None, 100, DocumentKind::Email),
        )
        .await
        .unwrap();
        upsert_document(
            async_conn.clone(),
            &doc(98, 1, Some(900), None, None, 100, DocumentKind::Email),
        )
        .await
        .unwrap();

        let page1 = list_documents(
            async_conn.clone(),
            ListDocumentsRequest {
                source_id: None,
                credential_id: None,
                kind: None,
                parent_document_id: None,
                limit: Some(2),
                cursor: None,
                sort_by: Some(DocumentSortBy::ReceivedAtDesc),
            },
        )
        .await
        .unwrap();

        assert_eq!(
            page1.documents.iter().map(|d| d.id).collect::<Vec<_>>(),
            vec![100, 99]
        );
        assert!(page1.has_more);
        assert!(page1.next_cursor.is_some());

        let page2 = list_documents(
            async_conn.clone(),
            ListDocumentsRequest {
                source_id: None,
                credential_id: None,
                kind: None,
                parent_document_id: None,
                limit: Some(2),
                cursor: page1.next_cursor,
                sort_by: Some(DocumentSortBy::ReceivedAtDesc),
            },
        )
        .await
        .unwrap();

        assert_eq!(
            page2.documents.iter().map(|d| d.id).collect::<Vec<_>>(),
            vec![98]
        );
        assert!(!page2.has_more);
    }

    #[tokio::test]
    async fn null_fallback_sort_works_for_received() {
        let async_conn = test_async_conn("documents-fallback");

        upsert_document_source(
            async_conn.clone(),
            &DocumentSource {
                id: 1,
                source_type: DocumentSourceType::ImapAccount,
                display_name: "imap".to_string(),
                credential_id: Some(10),
                root_reference: None,
                access_state: SourceAccessState::Accessible,
                permission_state: SourcePermissionState::Granted,
                access_checked_at: None,
                permission_checked_at: None,
                created_at: 1,
                updated_at: 1,
            },
        )
        .await
        .unwrap();

        // d1 uses date_modified fallback (2000), d2 uses date_created fallback (1500), d3 uses created_at fallback (1000)
        upsert_document(
            async_conn.clone(),
            &doc(1, 1, None, Some(2000), None, 500, DocumentKind::File),
        )
        .await
        .unwrap();
        upsert_document(
            async_conn.clone(),
            &doc(2, 1, None, None, Some(1500), 500, DocumentKind::Attachment),
        )
        .await
        .unwrap();
        upsert_document(
            async_conn.clone(),
            &doc(3, 1, None, None, None, 1000, DocumentKind::Email),
        )
        .await
        .unwrap();

        let response = list_documents(
            async_conn.clone(),
            ListDocumentsRequest {
                source_id: None,
                credential_id: None,
                kind: None,
                parent_document_id: None,
                limit: Some(10),
                cursor: None,
                sort_by: Some(DocumentSortBy::ReceivedAtDesc),
            },
        )
        .await
        .unwrap();

        assert_eq!(
            response.documents.iter().map(|d| d.id).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    #[tokio::test]
    async fn filters_compose_as_expected() {
        let async_conn = test_async_conn("documents-filters");

        upsert_document_source(
            async_conn.clone(),
            &DocumentSource {
                id: 1,
                source_type: DocumentSourceType::ImapAccount,
                display_name: "imap-a".to_string(),
                credential_id: Some(10),
                root_reference: None,
                access_state: SourceAccessState::Accessible,
                permission_state: SourcePermissionState::Granted,
                access_checked_at: None,
                permission_checked_at: None,
                created_at: 1,
                updated_at: 1,
            },
        )
        .await
        .unwrap();
        upsert_document_source(
            async_conn.clone(),
            &DocumentSource {
                id: 2,
                source_type: DocumentSourceType::ImapAccount,
                display_name: "imap-b".to_string(),
                credential_id: Some(11),
                root_reference: None,
                access_state: SourceAccessState::Accessible,
                permission_state: SourcePermissionState::Granted,
                access_checked_at: None,
                permission_checked_at: None,
                created_at: 1,
                updated_at: 1,
            },
        )
        .await
        .unwrap();

        upsert_document(
            async_conn.clone(),
            &doc(10, 1, Some(100), None, None, 10, DocumentKind::Email),
        )
        .await
        .unwrap();
        upsert_document(
            async_conn.clone(),
            &doc(11, 1, Some(90), None, None, 10, DocumentKind::Attachment),
        )
        .await
        .unwrap();
        let mut child = doc(12, 1, Some(80), None, None, 10, DocumentKind::Attachment);
        child.parent_document_id = Some(10);
        upsert_document(async_conn.clone(), &child).await.unwrap();
        upsert_document(
            async_conn.clone(),
            &doc(13, 2, Some(70), None, None, 10, DocumentKind::Email),
        )
        .await
        .unwrap();

        let response = list_documents(
            async_conn.clone(),
            ListDocumentsRequest {
                source_id: Some(1),
                credential_id: Some(10),
                kind: Some(DocumentKind::Attachment),
                parent_document_id: Some(10),
                limit: Some(10),
                cursor: None,
                sort_by: Some(DocumentSortBy::ReceivedAtDesc),
            },
        )
        .await
        .unwrap();

        assert_eq!(response.documents.len(), 1);
        assert_eq!(response.documents[0].id, 12);
    }
}
