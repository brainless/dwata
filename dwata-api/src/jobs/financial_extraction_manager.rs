use crate::database::{financial_transactions as db, emails as emails_db};
use crate::database::AsyncDbConnection;
use anyhow::Result;
use extractors::FinancialPatternExtractor;

pub struct FinancialExtractionManager {
    db_conn: AsyncDbConnection,
    extractor: FinancialPatternExtractor,
}

impl FinancialExtractionManager {
    pub fn new(db_conn: AsyncDbConnection) -> Self {
        Self {
            db_conn,
            extractor: FinancialPatternExtractor::new(),
        }
    }

    pub async fn extract_from_emails(
        &self,
        email_ids: Option<Vec<i64>>,
    ) -> Result<usize> {
        let emails = if let Some(ids) = email_ids {
            let mut emails = Vec::new();
            for id in ids {
                if let Ok(email) = emails_db::get_email(self.db_conn.clone(), id).await {
                    emails.push(email);
                }
            }
            emails
        } else {
            emails_db::list_emails(self.db_conn.clone(), 1000).await?
        };

        let mut total_extracted = 0;

        for email in emails {
            let transactions = self.extractor.extract_from_email(
                &email.subject.unwrap_or_default(),
                &email.body_text.unwrap_or_default(),
            );

            for mut transaction in transactions {
                transaction.source_type = "email".to_string();
                transaction.source_id = email.id.to_string();

                db::insert_financial_transaction(
                    self.db_conn.clone(),
                    &transaction,
                    None,
                ).await?;

                total_extracted += 1;
            }
        }

        Ok(total_extracted)
    }
}
