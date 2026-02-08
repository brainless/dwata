use crate::database::{
    emails as emails_db,
    financial_extraction_attempts as attempts_db,
    financial_extraction_sources as sources_db,
    financial_patterns as patterns_db,
    financial_transactions as db,
};
use crate::database::AsyncDbConnection;
use anyhow::Result;
use extractors::FinancialPatternExtractor;
use std::collections::HashMap;

struct AttemptStats {
    total_items_scanned: i64,
    transactions_extracted: i64,
}

pub struct FinancialExtractionManager {
    db_conn: AsyncDbConnection,
}

impl FinancialExtractionManager {
    pub fn new(db_conn: AsyncDbConnection) -> Self {
        Self {
            db_conn,
        }
    }

    pub async fn extract_from_emails(
        &self,
        email_ids: Option<Vec<i64>>,
        credential_id: Option<i64>,
    ) -> Result<usize> {
        let source_type = "imap";
        let attempted_at = chrono::Utc::now().timestamp();

        let emails = if let Some(ids) = email_ids {
            let mut emails = Vec::new();
            for id in ids {
                if let Ok(email) = emails_db::get_email(self.db_conn.clone(), id).await {
                    emails.push(email);
                }
            }
            emails
        } else {
            emails_db::list_emails(self.db_conn.clone(), credential_id, None, 1000, 0).await?
        };

        let mut attempt_stats: HashMap<i64, AttemptStats> = HashMap::new();
        if let Some(credential_id) = credential_id {
            attempt_stats.insert(
                credential_id,
                AttemptStats {
                    total_items_scanned: 0,
                    transactions_extracted: 0,
                },
            );
        }

        for email in &emails {
            let stats = attempt_stats.entry(email.credential_id).or_insert(
                AttemptStats {
                    total_items_scanned: 0,
                    transactions_extracted: 0,
                },
            );
            stats.total_items_scanned += 1;
        }

        let db_patterns = patterns_db::list_active_patterns(self.db_conn.clone()).await?;

        if db_patterns.is_empty() {
            for (account_id, stats) in attempt_stats {
                let _ = attempts_db::insert_attempt(
                    self.db_conn.clone(),
                    source_type,
                    account_id,
                    attempted_at,
                    stats.total_items_scanned,
                    0,
                    "failed",
                    Some("No active patterns found"),
                )
                .await;
            }

            return Err(anyhow::anyhow!("No active patterns found"));
        }

        let extractor = FinancialPatternExtractor::from_patterns(db_patterns)?;

        let mut total_extracted = 0;

        for email in emails {
            let source_type = "email";
            let source_id = email.id.to_string();

            let is_processed = sources_db::is_source_processed(
                self.db_conn.clone(),
                source_type,
                &source_id,
            ).await.unwrap_or(false);

            if is_processed {
                tracing::debug!("Skipping already processed email: {}", email.id);
                continue;
            }

            let text = format!("{}\n\n{}",
                email.subject.unwrap_or_default(),
                email.body_text.unwrap_or_default()
            );

            let results = extractor.extract_from_text(&text);

            let transaction_count = results.len();
            if let Some(stats) = attempt_stats.get_mut(&email.credential_id) {
                stats.transactions_extracted += transaction_count as i64;
            }

            for (mut transaction, pattern_id) in results {
                transaction.source_type = source_type.to_string();
                transaction.source_id = source_id.clone();

                db::insert_financial_transaction(
                    self.db_conn.clone(),
                    &transaction,
                    None,
                ).await?;

                patterns_db::increment_match_count(
                    self.db_conn.clone(),
                    pattern_id,
                ).await?;

                patterns_db::update_last_matched(
                    self.db_conn.clone(),
                    pattern_id,
                    chrono::Utc::now().timestamp(),
                ).await?;

                total_extracted += 1;
            }

            if transaction_count > 0 {
                sources_db::mark_source_processed(
                    self.db_conn.clone(),
                    source_type,
                    &source_id,
                    None,
                    transaction_count as i32,
                ).await?;
            }
        }

        for (account_id, stats) in attempt_stats {
            let _ = attempts_db::insert_attempt(
                self.db_conn.clone(),
                source_type,
                account_id,
                attempted_at,
                stats.total_items_scanned,
                stats.transactions_extracted,
                "completed",
                None,
            )
            .await;
        }

        Ok(total_extracted)
    }
}
