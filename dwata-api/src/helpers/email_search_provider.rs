use anyhow::Result;
use async_trait::async_trait;
use dwata_agents::email_entity_extractor::search::{
    EmailSearchProvider, EmailSearchResult, SearchEmailsParams,
};
use dwata_agents::simple_email_content;
use shared_types::{SearchField, SearchRequest, SearchTarget, SearchTerm};
use std::sync::Arc;

use crate::database::emails as emails_db;
use crate::database::AsyncDbConnection;
use crate::search::tantivy::TantivySearchIndex;

pub struct TantivyEmailSearchProvider {
    search_index: Arc<TantivySearchIndex>,
    db_conn: AsyncDbConnection,
    from_address: String,
}

impl TantivyEmailSearchProvider {
    pub fn new(
        search_index: Arc<TantivySearchIndex>,
        db_conn: AsyncDbConnection,
        from_address: String,
    ) -> Self {
        Self {
            search_index,
            db_conn,
            from_address,
        }
    }
}

#[async_trait]
impl EmailSearchProvider for TantivyEmailSearchProvider {
    async fn search_emails(&self, params: &SearchEmailsParams) -> Result<Vec<EmailSearchResult>> {
        let limit = params.limit.unwrap_or(5).min(10) as usize;

        let request = SearchRequest {
            target: SearchTarget::Email,
            terms: vec![
                SearchTerm {
                    field: SearchField::FromAddress,
                    value: self.from_address.clone(),
                    is_phrase: false,
                },
                SearchTerm {
                    field: SearchField::Any,
                    value: params.keywords.clone(),
                    is_phrase: false,
                },
            ],
            credential_id: None,
            limit: Some(limit),
            offset: None,
        };

        let search_index = self.search_index.clone();
        let tantivy_results =
            tokio::task::spawn_blocking(move || search_index.search(&request)).await??;

        if tantivy_results.hits.is_empty() {
            return Ok(Vec::new());
        }

        // Extract email IDs from hits
        let email_ids: Vec<i64> = tantivy_results
            .hits
            .iter()
            .filter_map(|h| match &h.hit_id {
                shared_types::HitId::Email(id) => Some(*id),
                _ => None,
            })
            .collect();

        let scan_rows = emails_db::list_email_scan_rows_by_ids(
            self.db_conn.clone(),
            &email_ids,
            None,
            Some(limit),
        )
        .await?;

        let results = scan_rows
            .into_iter()
            .map(|row| {
                let cleaned = simple_email_content(
                    row.subject.as_deref(),
                    row.body_text.as_deref(),
                    row.body_html.as_deref(),
                );
                let excerpt: String = cleaned.body.chars().take(500).collect();
                let date = chrono::DateTime::from_timestamp_secs(row.date_received)
                    .map(|dt: chrono::DateTime<chrono::Utc>| dt.format("%Y-%m-%d").to_string());
                EmailSearchResult {
                    subject: cleaned.subject,
                    from: row.from_address,
                    date,
                    body_excerpt: excerpt,
                }
            })
            .collect();

        Ok(results)
    }
}
