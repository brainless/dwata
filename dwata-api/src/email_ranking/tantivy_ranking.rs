use crate::database::AsyncDbConnection;
use crate::email_ranking::multi_factor::{
    meets_extraction_criteria, rank_emails_multi_factor, MultiFactorRankedEmail, RankingContext,
    RankingWeights, ThreadInfo,
};
use crate::email_ranking::{find_amounts, FINANCIAL_KEYWORDS};
use crate::search::tantivy::TantivySearchIndex;
use anyhow::Result;
use shared_types::{HitId, SearchField, SearchRequest, SearchTarget, SearchTerm};
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Result using the old format for backwards compatibility
#[derive(Debug, Clone)]
pub struct RankedEmail {
    pub email_id: i64,
    pub credential_id: i64,
    pub from_address: String,
    pub subject: Option<String>,
    pub date_received: i64,
    pub score: u32,
    pub keywords_found: Vec<String>,
    pub amounts_found: Vec<String>,
    pub has_date: bool,
}

/// Build ranking context by querying database for user engagement and thread data
pub async fn build_ranking_context(
    conn: AsyncDbConnection,
    credential_id: Option<i64>,
) -> Result<RankingContext> {
    let current_time = chrono::Utc::now().timestamp_millis();
    let mut context = RankingContext::new(current_time);

    // Query for user reply counts per sender
    let reply_counts = query_user_reply_counts(conn.clone(), credential_id).await?;
    context.user_reply_counts = reply_counts;

    // Query for thread information
    let thread_info = query_thread_info(conn.clone(), credential_id).await?;
    context.thread_info = thread_info;

    Ok(context)
}

/// Query database to count user's replies to each sender
async fn query_user_reply_counts(
    conn: AsyncDbConnection,
    credential_id: Option<i64>,
) -> Result<HashMap<String, i64>> {
    let reply_counts = Arc::new(Mutex::new(HashMap::new()));
    let reply_counts_clone = reply_counts.clone();

    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let conn = conn.get_blocking();

        // Define the row mapper function
        fn map_row(row: &rusqlite::Row) -> rusqlite::Result<(String, i64)> {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        }

        // Query to find emails where user replied (is_answered = true)
        // and count by the original sender (the person user replied to)
        if let Some(cred) = credential_id {
            let mut stmt = conn.prepare(
                "SELECT e.from_address, COUNT(*) as reply_count 
                 FROM emails e 
                 WHERE e.is_answered = 1 
                   AND e.credential_id = ? 
                 GROUP BY e.from_address",
            )?;

            let rows = stmt.query_map([cred], map_row)?;
            let mut counts = reply_counts_clone.blocking_lock();
            for row in rows {
                let (sender, count) = row?;
                counts.insert(sender, count);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT e.from_address, COUNT(*) as reply_count 
                 FROM emails e 
                 WHERE e.is_answered = 1 
                 GROUP BY e.from_address",
            )?;

            let rows = stmt.query_map([], map_row)?;
            let mut counts = reply_counts_clone.blocking_lock();
            for row in rows {
                let (sender, count) = row?;
                counts.insert(sender, count);
            }
        }

        anyhow::Result::Ok(())
    })
    .await?;

    // Unwrap the Arc<Mutex<>> to get the HashMap
    let result = Arc::try_unwrap(reply_counts)
        .map_err(|_| anyhow::anyhow!("Failed to unwrap Arc"))?
        .into_inner();

    Ok(result)
}

/// Query database for thread information
async fn query_thread_info(
    conn: AsyncDbConnection,
    credential_id: Option<i64>,
) -> Result<HashMap<Option<String>, ThreadInfo>> {
    let thread_info = Arc::new(Mutex::new(HashMap::new()));
    let thread_info_clone = thread_info.clone();

    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let conn = conn.get_blocking();

        // Define the row mapper function
        fn map_thread_row(row: &rusqlite::Row) -> rusqlite::Result<(Option<String>, ThreadInfo)> {
            let thread_id: Option<String> = row.get(0)?;
            let email_count: i64 = row.get(1)?;
            let user_replies: i64 = row.get(2)?;
            Ok((
                thread_id,
                ThreadInfo {
                    email_count,
                    has_user_reply: user_replies > 0,
                },
            ))
        }

        // Query to get thread statistics
        // Count emails per thread and check if user replied (is_answered)
        if let Some(cred) = credential_id {
            let mut stmt = conn.prepare(
                "SELECT 
                    e.thread_id,
                    COUNT(*) as email_count,
                    SUM(CASE WHEN e.is_answered = 1 THEN 1 ELSE 0 END) as user_replies
                 FROM emails e 
                 WHERE e.credential_id = ? 
                   AND e.thread_id IS NOT NULL
                 GROUP BY e.thread_id",
            )?;

            let rows = stmt.query_map([cred], map_thread_row)?;
            let mut info = thread_info_clone.blocking_lock();
            for row in rows {
                let (thread_id, thread_data) = row?;
                info.insert(thread_id, thread_data);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT 
                    e.thread_id,
                    COUNT(*) as email_count,
                    SUM(CASE WHEN e.is_answered = 1 THEN 1 ELSE 0 END) as user_replies
                 FROM emails e 
                 WHERE e.thread_id IS NOT NULL
                 GROUP BY e.thread_id",
            )?;

            let rows = stmt.query_map([], map_thread_row)?;
            let mut info = thread_info_clone.blocking_lock();
            for row in rows {
                let (thread_id, thread_data) = row?;
                info.insert(thread_id, thread_data);
            }
        }

        anyhow::Result::Ok(())
    })
    .await?;

    // Unwrap the Arc<Mutex<>> to get the HashMap
    let result = Arc::try_unwrap(thread_info)
        .map_err(|_| anyhow::anyhow!("Failed to unwrap Arc"))?
        .into_inner();

    Ok(result)
}

/// Search emails using Tantivy index with financial keywords
/// Returns a list of ranked email IDs ordered by most recent first
pub fn search_financial_emails_with_tantivy(
    index: &TantivySearchIndex,
    credential_id: Option<i64>,
    limit: usize,
) -> Result<Vec<i64>> {
    let mut all_email_ids: HashSet<i64> = HashSet::new();

    // Search for each financial keyword
    for keyword in FINANCIAL_KEYWORDS.iter().take(20) {
        // Limit to most common keywords
        let request = SearchRequest {
            terms: vec![SearchTerm {
                field: SearchField::Any,
                value: keyword.to_string(),
                is_phrase: false,
            }],
            target: SearchTarget::Email,
            credential_id,
            limit: Some(limit),
            offset: Some(0),
        };

        match index.search(&request) {
            Ok(result) => {
                for hit in result.hits {
                    if let HitId::Email(email_id) = hit.hit_id {
                        all_email_ids.insert(email_id);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Search failed for keyword '{}': {}", keyword, e);
            }
        }
    }

    // Convert to sorted vector
    let mut email_ids: Vec<i64> = all_email_ids.into_iter().collect();

    // Sort by ID descending (assuming higher IDs are more recent)
    email_ids.sort_by(|a, b| b.cmp(a));

    Ok(email_ids)
}

/// Convert multi-factor ranked email to legacy RankedEmail format
fn convert_to_legacy_format(multi: &MultiFactorRankedEmail) -> RankedEmail {
    RankedEmail {
        email_id: multi.email_id,
        credential_id: multi.credential_id,
        from_address: multi.from_address.clone(),
        subject: multi.subject.clone(),
        date_received: multi.date_received,
        score: multi.final_score as u32,
        keywords_found: vec![], // Would need email content
        amounts_found: vec![],  // Would need email content
        has_date: true,         // Already filtered for this
    }
}

/// Quick check version that only uses database queries without Tantivy
/// Uses multi-factor ranking
pub async fn rank_emails_from_db(
    db: &crate::database::Database,
    credential_id: Option<i64>,
    limit: usize,
) -> Result<Vec<RankedEmail>> {
    // Build ranking context
    let context = build_ranking_context(db.async_connection.clone(), credential_id).await?;

    // Use default weights
    let weights = RankingWeights::default();

    // Fetch emails from database
    let emails = crate::database::emails::list_emails(
        db.async_connection.clone(),
        credential_id,
        None,      // no folder filter
        limit * 3, // fetch more to filter down
        0,         // no offset
    )
    .await?;

    // Filter and rank using multi-factor ranking
    let filtered_emails: Vec<_> = emails
        .into_iter()
        .filter(|email| meets_extraction_criteria(email))
        .collect();

    let multi_ranked = rank_emails_multi_factor(filtered_emails, &context, &weights);

    // Convert to legacy format and apply limit
    let ranked: Vec<RankedEmail> = multi_ranked
        .into_iter()
        .take(limit)
        .map(|multi| {
            // Re-extract keywords and amounts from the email for display
            // We need to get the full email to extract keywords/amounts
            // For now, return basic info
            RankedEmail {
                email_id: multi.email_id,
                credential_id: multi.credential_id,
                from_address: multi.from_address,
                subject: multi.subject,
                date_received: multi.date_received,
                score: multi.final_score as u32,
                keywords_found: vec![],
                amounts_found: vec![],
                has_date: true,
            }
        })
        .collect();

    Ok(ranked)
}

/// Rank emails with full detail (for CLI verbose mode)
pub async fn rank_emails_from_db_detailed(
    db: &crate::database::Database,
    credential_id: Option<i64>,
    limit: usize,
) -> Result<Vec<(MultiFactorRankedEmail, Vec<String>, Vec<String>)>> {
    // Build ranking context
    let context = build_ranking_context(db.async_connection.clone(), credential_id).await?;

    // Use default weights
    let weights = RankingWeights::default();

    // Fetch emails from database
    let emails = crate::database::emails::list_emails(
        db.async_connection.clone(),
        credential_id,
        None,
        limit * 3,
        0,
    )
    .await?;

    // Get full email data with content for keyword/amount extraction
    let email_ids: Vec<i64> = emails.iter().map(|e| e.id).collect();
    let full_emails =
        crate::database::emails::get_emails_by_ids(db.async_connection.clone(), &email_ids).await?;

    // Filter and rank using multi-factor ranking
    let filtered_emails: Vec<_> = full_emails
        .into_iter()
        .filter(|email| meets_extraction_criteria(email))
        .collect();

    let multi_ranked = rank_emails_multi_factor(filtered_emails.clone(), &context, &weights);

    // Build lookup map
    let email_map: std::collections::HashMap<i64, _> =
        filtered_emails.into_iter().map(|e| (e.id, e)).collect();

    // Extract keywords and amounts for each
    let detailed: Vec<_> = multi_ranked
        .into_iter()
        .take(limit)
        .map(|multi| {
            if let Some(email) = email_map.get(&multi.email_id) {
                let subject = email.subject.as_deref().unwrap_or("");
                let body_text = email.body_text.as_deref().unwrap_or("");
                let body_html = email.body_html.as_deref().unwrap_or("");
                let combined = format!("{} {} {}", subject, body_text, body_html);

                let keywords: Vec<String> = FINANCIAL_KEYWORDS
                    .iter()
                    .filter(|k| combined.to_lowercase().contains(*k))
                    .map(|k| k.to_string())
                    .collect();

                let amounts = find_amounts(&combined);

                (multi, keywords, amounts)
            } else {
                (multi, vec![], vec![])
            }
        })
        .collect();

    Ok(detailed)
}
