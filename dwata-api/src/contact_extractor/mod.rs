mod classifier;
pub use classifier::{classify_sender, SenderKind};

use crate::database::{organisations, persons, AsyncDbConnection};
use anyhow::Result;

#[derive(Debug, Default, serde::Serialize)]
pub struct ExtractionStats {
    pub total_processed: usize,
    pub persons_created: usize,
    pub persons_skipped: usize,
    pub organisations_created: usize,
    pub organisations_skipped: usize,
}

/// Fetches all distinct (from_name, from_address) pairs from the emails table
/// where a display name is present.
async fn get_distinct_senders(conn: AsyncDbConnection) -> Result<Vec<(String, String)>> {
    let locked = conn.lock().await;

    let mut stmt = locked.prepare(
        "SELECT DISTINCT from_name, from_address
         FROM emails
         WHERE from_name IS NOT NULL AND from_name != ''
         ORDER BY from_address",
    )?;

    let pairs: Vec<(String, String)> = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(pairs)
}

/// Scans all emails, classifies each unique sender, and upserts them into
/// either the `persons` or `organisations` table.
///
/// Only senders where a display name is available (i.e. the email arrived in
/// the form `Name <addr>`) are processed.
pub async fn extract_contacts_from_emails(conn: AsyncDbConnection) -> Result<ExtractionStats> {
    let senders = get_distinct_senders(conn.clone()).await?;

    let mut stats = ExtractionStats::default();

    for (name, email) in senders {
        stats.total_processed += 1;

        match classify_sender(&name, &email) {
            SenderKind::Person => {
                match persons::get_or_create_person_by_email(
                    conn.clone(),
                    name.clone(),
                    email.clone(),
                )
                .await
                {
                    Ok((_, true)) => stats.persons_created += 1,
                    Ok((_, false)) => stats.persons_skipped += 1,
                    Err(err) => {
                        tracing::warn!(
                            name = %name,
                            email = %email,
                            error = %err,
                            "Failed to upsert person"
                        );
                    }
                }
            }
            SenderKind::Organisation => {
                match organisations::get_or_create_organisation_by_email(
                    conn.clone(),
                    name.clone(),
                    email.clone(),
                )
                .await
                {
                    Ok((_, true)) => stats.organisations_created += 1,
                    Ok((_, false)) => stats.organisations_skipped += 1,
                    Err(err) => {
                        tracing::warn!(
                            name = %name,
                            email = %email,
                            error = %err,
                            "Failed to upsert organisation"
                        );
                    }
                }
            }
        }
    }

    tracing::info!(
        total_processed = stats.total_processed,
        persons_created = stats.persons_created,
        persons_skipped = stats.persons_skipped,
        organisations_created = stats.organisations_created,
        organisations_skipped = stats.organisations_skipped,
        "Contact extraction complete"
    );

    Ok(stats)
}
