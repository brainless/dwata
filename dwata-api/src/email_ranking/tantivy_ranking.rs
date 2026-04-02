use crate::database::AsyncDbConnection;
use crate::email_ranking::multi_factor::{
    meets_extraction_criteria, rank_emails_multi_factor, MultiFactorRankedEmail, RankingContext,
    RankingWeights, ThreadInfo,
};
use crate::email_ranking::sender::{compute_sender_score, normalize_sender_key, SenderAggregate};
use crate::email_ranking::{find_amounts, FINANCIAL_KEYWORDS};
use crate::search::tantivy::TantivySearchIndex;
use anyhow::Result;
use shared_types::email::Email;
use shared_types::{HitId, SearchField, SearchRequest, SearchTarget, SearchTerm};
use std::collections::HashMap;
use std::collections::HashSet;

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

#[derive(Debug, Clone)]
pub struct SenderScoreRow {
    pub sender: String,
    pub score: f64,
    pub emails_received: i64,
    pub emails_replied: i64,
    pub most_recent_date_received_ms: i64,
}

/// Pipeline switches to make ranking experimentation easy.
#[derive(Debug, Clone)]
pub struct RankingPipelineOptions {
    pub enable_tantivy_candidate_pass: bool,
    pub enable_sender_reputation_pass: bool,
    pub candidate_fetch_multiplier: usize,
}

impl Default for RankingPipelineOptions {
    fn default() -> Self {
        Self {
            enable_tantivy_candidate_pass: false,
            enable_sender_reputation_pass: true,
            candidate_fetch_multiplier: 6,
        }
    }
}

/// Build ranking context by querying database for user engagement/thread data, and optional sender data.
pub async fn build_ranking_context(
    conn: AsyncDbConnection,
    credential_id: Option<i64>,
) -> Result<RankingContext> {
    build_ranking_context_with_options(conn, credential_id, true).await
}

pub async fn build_ranking_context_with_options(
    conn: AsyncDbConnection,
    credential_id: Option<i64>,
    enable_sender_reputation_pass: bool,
) -> Result<RankingContext> {
    let current_time = chrono::Utc::now().timestamp_millis();
    let mut context = RankingContext::new(current_time);

    context.user_reply_counts = query_user_reply_counts(conn.clone(), credential_id).await?;
    context.thread_info = query_thread_info(conn.clone(), credential_id).await?;

    if enable_sender_reputation_pass {
        context.sender_scores =
            query_sender_scores(conn.clone(), credential_id, current_time).await?;
    }

    Ok(context)
}

/// Query database to count user's replies to each sender.
/// Uses sent emails that have in_reply_to set to another known message.
async fn query_user_reply_counts(
    conn: AsyncDbConnection,
    credential_id: Option<i64>,
) -> Result<HashMap<String, i64>> {
    tokio::task::spawn_blocking(move || -> anyhow::Result<HashMap<String, i64>> {
        let conn = conn.get_blocking();
        let mut counts = HashMap::new();
        let canonical_from_expr = "lower(trim(
            CASE
                WHEN instr(e1.from_address, '<') > 0
                 AND instr(e1.from_address, '>') > instr(e1.from_address, '<')
                THEN substr(
                    e1.from_address,
                    instr(e1.from_address, '<') + 1,
                    instr(e1.from_address, '>') - instr(e1.from_address, '<') - 1
                )
                ELSE e1.from_address
            END
        ))";

        let sql = format!(
            "WITH owner_emails AS (
                SELECT lower(trim(username)) AS email
                FROM credentials_metadata
                WHERE is_active = true
                  AND (?1 IS NULL OR id = ?1)
                  AND username IS NOT NULL
                  AND trim(username) <> ''
                  AND instr(username, '@') > 0

                UNION

                SELECT lower(trim(identifier)) AS email
                FROM credentials_metadata
                WHERE is_active = true
                  AND (?1 IS NULL OR id = ?1)
                  AND identifier IS NOT NULL
                  AND trim(identifier) <> ''
                  AND instr(identifier, '@') > 0
            ),
            reply_events AS (
                -- Reliable threaded replies
                SELECT
                    lower(trim(
                        CASE
                            WHEN instr(e2.from_address, '<') > 0
                             AND instr(e2.from_address, '>') > instr(e2.from_address, '<')
                            THEN substr(
                                e2.from_address,
                                instr(e2.from_address, '<') + 1,
                                instr(e2.from_address, '>') - instr(e2.from_address, '<') - 1
                            )
                            ELSE e2.from_address
                        END
                    )) AS sender_key,
                    e1.id AS sent_id
                FROM emails e1
                JOIN emails e2 ON e1.in_reply_to = e2.message_id
                WHERE e1.in_reply_to IS NOT NULL
                  AND e1.is_draft = false
                  AND e2.message_id IS NOT NULL
                  AND (?1 IS NULL OR (e1.credential_id = ?1 AND e2.credential_id = ?1))
                  AND (
                    NOT EXISTS (SELECT 1 FROM owner_emails)
                    OR {canonical_from_expr} IN (SELECT email FROM owner_emails)
                  )

                UNION

                -- Direct recipient matching from Sent -> to_addresses JSON
                SELECT
                    lower(trim(json_extract(ta.value, '$.email'))) AS sender_key,
                    e1.id AS sent_id
                FROM emails e1
                JOIN json_each(COALESCE(e1.to_addresses, '[]')) ta
                WHERE (?1 IS NULL OR e1.credential_id = ?1)
                  AND e1.is_draft = false
                  AND json_extract(ta.value, '$.email') IS NOT NULL
                  AND trim(json_extract(ta.value, '$.email')) <> ''
                  AND (
                    NOT EXISTS (SELECT 1 FROM owner_emails)
                    OR {canonical_from_expr} IN (SELECT email FROM owner_emails)
                  )
            )
            SELECT sender_key, COUNT(DISTINCT sent_id) AS reply_count
            FROM reply_events
            WHERE sender_key IS NOT NULL AND sender_key <> ''
            GROUP BY sender_key"
        );

        let mut stmt = conn.prepare(&sql)?;

        let rows = stmt.query_map([credential_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        for row in rows {
            let (sender_key, count) = row?;
            let sender_key = normalize_sender_key(&sender_key);
            *counts.entry(sender_key).or_insert(0) += count;
        }

        Ok(counts)
    })
    .await?
}

/// Query database for thread information.
/// Uses sent emails with in_reply_to mapped to thread messages.
async fn query_thread_info(
    conn: AsyncDbConnection,
    credential_id: Option<i64>,
) -> Result<HashMap<Option<String>, ThreadInfo>> {
    tokio::task::spawn_blocking(
        move || -> anyhow::Result<HashMap<Option<String>, ThreadInfo>> {
            let conn = conn.get_blocking();
            let mut info = HashMap::new();

            fn map_thread_row(
                row: &rusqlite::Row,
            ) -> rusqlite::Result<(Option<String>, ThreadInfo)> {
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

            if let Some(cred) = credential_id {
                let mut stmt = conn.prepare(
                    "SELECT
                    e.thread_id,
                    COUNT(DISTINCT e.id) as email_count,
                    COUNT(DISTINCT sent.id) as user_replies
                 FROM emails e
                 LEFT JOIN emails sent ON (
                     sent.in_reply_to = e.message_id
                     AND sent.folder_id IN (
                         SELECT id FROM email_folders
                         WHERE credential_id = ? AND folder_type = 'Sent'
                     )
                 )
                 WHERE e.credential_id = ?
                   AND e.thread_id IS NOT NULL
                 GROUP BY e.thread_id",
                )?;

                let rows = stmt.query_map([cred, cred], map_thread_row)?;
                for row in rows {
                    let (thread_id, thread_data) = row?;
                    info.insert(thread_id, thread_data);
                }
            } else {
                let mut stmt = conn.prepare(
                    "SELECT
                    e.thread_id,
                    COUNT(DISTINCT e.id) as email_count,
                    COUNT(DISTINCT sent.id) as user_replies
                 FROM emails e
                 LEFT JOIN emails sent ON (
                     sent.in_reply_to = e.message_id
                     AND sent.folder_id IN (
                         SELECT id FROM email_folders WHERE folder_type = 'Sent'
                     )
                 )
                 WHERE e.thread_id IS NOT NULL
                 GROUP BY e.thread_id",
                )?;

                let rows = stmt.query_map([], map_thread_row)?;
                for row in rows {
                    let (thread_id, thread_data) = row?;
                    info.insert(thread_id, thread_data);
                }
            }

            Ok(info)
        },
    )
    .await?
}

/// Query sender-level aggregates and convert them to sender reputation scores.
async fn query_sender_scores(
    conn: AsyncDbConnection,
    credential_id: Option<i64>,
    current_time_ms: i64,
) -> Result<HashMap<String, f64>> {
    let aggregates = query_sender_aggregates(conn, credential_id).await?;
    let mut sender_scores = HashMap::new();
    for (sender_key, stats) in aggregates {
        sender_scores.insert(sender_key, compute_sender_score(&stats, current_time_ms));
    }
    Ok(sender_scores)
}

async fn query_sender_aggregates(
    conn: AsyncDbConnection,
    credential_id: Option<i64>,
) -> Result<Vec<(String, SenderAggregate)>> {
    tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<(String, SenderAggregate)>> {
        let conn = conn.get_blocking();
        let owner_sender_keys = query_owner_sender_keys(&conn, credential_id)?;
        let mut rows_out = Vec::new();

        let canonical_from_expr = "lower(trim(
            CASE
                WHEN instr(e1.from_address, '<') > 0
                 AND instr(e1.from_address, '>') > instr(e1.from_address, '<')
                THEN substr(
                    e1.from_address,
                    instr(e1.from_address, '<') + 1,
                    instr(e1.from_address, '>') - instr(e1.from_address, '<') - 1
                )
                ELSE e1.from_address
            END
        ))";
        let sql = format!(
            r#"
            WITH owner_emails AS (
                SELECT lower(trim(username)) AS email
                FROM credentials_metadata
                WHERE is_active = true
                  AND (?1 IS NULL OR id = ?1)
                  AND username IS NOT NULL
                  AND trim(username) <> ''
                  AND instr(username, '@') > 0

                UNION

                SELECT lower(trim(identifier)) AS email
                FROM credentials_metadata
                WHERE is_active = true
                  AND (?1 IS NULL OR id = ?1)
                  AND identifier IS NOT NULL
                  AND trim(identifier) <> ''
                  AND instr(identifier, '@') > 0
            ),
            sender_base AS (
                SELECT
                    lower(trim(
                        CASE
                            WHEN instr(e.from_address, '<') > 0
                             AND instr(e.from_address, '>') > instr(e.from_address, '<')
                            THEN substr(
                                e.from_address,
                                instr(e.from_address, '<') + 1,
                                instr(e.from_address, '>') - instr(e.from_address, '<') - 1
                            )
                            ELSE e.from_address
                        END
                    )) AS sender_key,
                    COUNT(*) AS email_count,
                    MAX(e.date_received) AS last_email_received,
                    COUNT(DISTINCT strftime('%Y-%m', e.date_received / 1000, 'unixepoch')) AS active_months
                FROM emails e
                WHERE e.from_address IS NOT NULL
                  AND trim(e.from_address) <> ''
                  AND (?1 IS NULL OR e.credential_id = ?1)
                GROUP BY lower(trim(e.from_address))
            ),
            reply_counts AS (
                WITH reply_events AS (
                    -- Reliable threaded replies
                    SELECT
                        lower(trim(
                            CASE
                                WHEN instr(e2.from_address, '<') > 0
                                 AND instr(e2.from_address, '>') > instr(e2.from_address, '<')
                                THEN substr(
                                    e2.from_address,
                                    instr(e2.from_address, '<') + 1,
                                    instr(e2.from_address, '>') - instr(e2.from_address, '<') - 1
                                )
                                ELSE e2.from_address
                            END
                        )) AS sender_key,
                        e1.id AS sent_id,
                        e1.date_received AS sent_date_received
                    FROM emails e1
                    JOIN emails e2 ON e1.in_reply_to = e2.message_id
                    WHERE e1.in_reply_to IS NOT NULL
                      AND e1.is_draft = false
                      AND e2.message_id IS NOT NULL
                      AND (
                        ?1 IS NULL
                        OR (e1.credential_id = ?1 AND e2.credential_id = ?1)
                      )
                      AND (
                        NOT EXISTS (SELECT 1 FROM owner_emails)
                        OR {canonical_from_expr} IN (SELECT email FROM owner_emails)
                      )

                    UNION

                    -- Direct recipient matching from Sent -> to_addresses JSON
                    SELECT
                        lower(trim(json_extract(ta.value, '$.email'))) AS sender_key,
                        e1.id AS sent_id,
                        e1.date_received AS sent_date_received
                    FROM emails e1
                    JOIN json_each(COALESCE(e1.to_addresses, '[]')) ta
                    WHERE (?1 IS NULL OR e1.credential_id = ?1)
                      AND e1.is_draft = false
                      AND json_extract(ta.value, '$.email') IS NOT NULL
                      AND trim(json_extract(ta.value, '$.email')) <> ''
                      AND (
                        NOT EXISTS (SELECT 1 FROM owner_emails)
                        OR {canonical_from_expr} IN (SELECT email FROM owner_emails)
                      )
                )
                SELECT
                    sender_key,
                    COUNT(DISTINCT sent_id) AS user_reply_count,
                    MAX(sent_date_received) AS last_user_reply_ms
                FROM reply_events
                WHERE sender_key IS NOT NULL AND sender_key <> ''
                GROUP BY sender_key
            ),
            engaged_threads AS (
                SELECT
                    lower(trim(
                        CASE
                            WHEN instr(root.from_address, '<') > 0
                             AND instr(root.from_address, '>') > instr(root.from_address, '<')
                            THEN substr(
                                root.from_address,
                                instr(root.from_address, '<') + 1,
                                instr(root.from_address, '>') - instr(root.from_address, '<') - 1
                            )
                            ELSE root.from_address
                        END
                    )) AS sender_key,
                    COUNT(DISTINCT root.thread_id) AS engaged_thread_count
                FROM emails root
                JOIN emails sent ON sent.in_reply_to = root.message_id
                WHERE root.thread_id IS NOT NULL
                  AND sent.is_draft = false
                  AND (
                    ?1 IS NULL
                    OR (root.credential_id = ?1 AND sent.credential_id = ?1)
                  )
                  AND (
                    NOT EXISTS (SELECT 1 FROM owner_emails)
                    OR lower(trim(
                        CASE
                            WHEN instr(sent.from_address, '<') > 0
                             AND instr(sent.from_address, '>') > instr(sent.from_address, '<')
                            THEN substr(
                                sent.from_address,
                                instr(sent.from_address, '<') + 1,
                                instr(sent.from_address, '>') - instr(sent.from_address, '<') - 1
                            )
                            ELSE sent.from_address
                        END
                    )) IN (SELECT email FROM owner_emails)
                  )
                GROUP BY lower(trim(
                    CASE
                        WHEN instr(root.from_address, '<') > 0
                         AND instr(root.from_address, '>') > instr(root.from_address, '<')
                        THEN substr(
                            root.from_address,
                            instr(root.from_address, '<') + 1,
                            instr(root.from_address, '>') - instr(root.from_address, '<') - 1
                        )
                        ELSE root.from_address
                    END
                ))
            )
            SELECT
                sb.sender_key,
                sb.email_count,
                sb.last_email_received,
                sb.active_months,
                COALESCE(rc.user_reply_count, 0) AS user_reply_count,
                COALESCE(rc.last_user_reply_ms, 0) AS last_user_reply_ms,
                COALESCE(et.engaged_thread_count, 0) AS engaged_thread_count
            FROM sender_base sb
            LEFT JOIN reply_counts rc ON rc.sender_key = sb.sender_key
            LEFT JOIN engaged_threads et ON et.sender_key = sb.sender_key
        "#
        );

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([credential_id], |row| {
            let sender_key: String = row.get(0)?;
            let stats = SenderAggregate {
                email_count: row.get(1)?,
                last_email_received_ms: row.get(2)?,
                active_months: row.get(3)?,
                user_reply_count: row.get(4)?,
                last_user_reply_ms: row.get(5)?,
                engaged_thread_count: row.get(6)?,
            };
            Ok((sender_key, stats))
        })?;

        for row in rows {
            let (sender_key, stats) = row?;
            if owner_sender_keys.contains(&sender_key) {
                continue;
            }
            rows_out.push((sender_key, stats));
        }

        Ok(rows_out)
    })
    .await?
}

fn query_owner_sender_keys(
    conn: &r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>,
    credential_id: Option<i64>,
) -> anyhow::Result<HashSet<String>> {
    let mut owners = HashSet::new();
    let mut stmt = conn.prepare(
        "SELECT lower(trim(identifier)) AS identifier_key,
                lower(trim(username)) AS username_key
         FROM credentials_metadata
         WHERE is_active = true
           AND (?1 IS NULL OR id = ?1)",
    )?;

    let rows = stmt.query_map([credential_id], |row| {
        let identifier_key: String = row.get(0)?;
        let username_key: String = row.get(1)?;
        Ok((identifier_key, username_key))
    })?;

    for row in rows {
        let (identifier_key, username_key) = row?;
        if looks_like_email(&identifier_key) {
            owners.insert(identifier_key);
        }
        if looks_like_email(&username_key) {
            owners.insert(username_key);
        }
    }

    Ok(owners)
}

fn looks_like_email(value: &str) -> bool {
    let v = value.trim();
    !v.is_empty() && v.contains('@') && !v.contains(' ')
}

pub async fn list_top_sender_scores(
    conn: AsyncDbConnection,
    credential_id: Option<i64>,
    limit: usize,
) -> Result<Vec<SenderScoreRow>> {
    let current_time = chrono::Utc::now().timestamp_millis();
    let aggregates = query_sender_aggregates(conn, credential_id).await?;
    let mut rows: Vec<SenderScoreRow> = aggregates
        .into_iter()
        .map(|(sender, stats)| SenderScoreRow {
            sender,
            score: compute_sender_score(&stats, current_time),
            emails_received: stats.email_count,
            emails_replied: stats.user_reply_count,
            most_recent_date_received_ms: stats.last_email_received_ms,
        })
        .collect();

    rows.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.sender.cmp(&b.sender))
    });
    rows.truncate(limit);
    Ok(rows)
}

/// Search emails using Tantivy index with financial keywords.
/// Returns a list of ranked email IDs ordered by most recent first.
pub fn search_financial_emails_with_tantivy(
    index: &TantivySearchIndex,
    credential_id: Option<i64>,
    limit: usize,
) -> Result<Vec<i64>> {
    let mut all_email_ids: HashSet<i64> = HashSet::new();

    // Search for each financial keyword.
    for keyword in FINANCIAL_KEYWORDS.iter().take(20) {
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

    let mut email_ids: Vec<i64> = all_email_ids.into_iter().collect();
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
        keywords_found: vec![],
        amounts_found: vec![],
        has_date: true,
    }
}

fn candidate_fetch_limit(limit: usize, multiplier: usize) -> usize {
    let mult = multiplier.max(1);
    limit.saturating_mul(mult).max(limit)
}

async fn candidate_email_pass(
    db: &crate::database::Database,
    credential_id: Option<i64>,
    limit: usize,
    options: &RankingPipelineOptions,
    search_index: Option<&TantivySearchIndex>,
) -> Result<Vec<Email>> {
    let fetch_limit = candidate_fetch_limit(limit, options.candidate_fetch_multiplier);

    if options.enable_tantivy_candidate_pass {
        if let Some(index) = search_index {
            let email_ids =
                search_financial_emails_with_tantivy(index, credential_id, fetch_limit)?;
            if !email_ids.is_empty() {
                return crate::database::emails::get_emails_by_ids(
                    db.async_connection.clone(),
                    &email_ids,
                )
                .await;
            }
            tracing::debug!("Tantivy candidate pass returned zero IDs; falling back to DB listing");
        } else {
            tracing::debug!(
                "Tantivy candidate pass enabled but no index provided; using DB listing"
            );
        }
    }

    crate::database::emails::list_emails(
        db.async_connection.clone(),
        credential_id,
        None,
        fetch_limit,
        0,
    )
    .await
}

fn financial_filter_pass(emails: Vec<Email>) -> Vec<Email> {
    emails
        .into_iter()
        .filter(|email| meets_extraction_criteria(email))
        .collect()
}

fn rank_pass(
    emails: Vec<Email>,
    context: &RankingContext,
    options: &RankingPipelineOptions,
) -> Vec<MultiFactorRankedEmail> {
    let mut weights = RankingWeights::default();
    if !options.enable_sender_reputation_pass {
        weights.sender_reputation_boost = 0.0;
    }

    rank_emails_multi_factor(emails, context, &weights)
}

/// Quick check version using DB, with optional Tantivy candidate pass.
pub async fn rank_emails_from_db_with_options(
    db: &crate::database::Database,
    credential_id: Option<i64>,
    limit: usize,
    options: &RankingPipelineOptions,
    search_index: Option<&TantivySearchIndex>,
) -> Result<Vec<RankedEmail>> {
    let context = build_ranking_context_with_options(
        db.async_connection.clone(),
        credential_id,
        options.enable_sender_reputation_pass,
    )
    .await?;

    let candidates = candidate_email_pass(db, credential_id, limit, options, search_index).await?;
    let filtered = financial_filter_pass(candidates);
    let ranked_multi = rank_pass(filtered, &context, options);

    Ok(ranked_multi
        .into_iter()
        .take(limit)
        .map(|m| convert_to_legacy_format(&m))
        .collect())
}

/// Backward-compatible wrapper with defaults.
pub async fn rank_emails_from_db(
    db: &crate::database::Database,
    credential_id: Option<i64>,
    limit: usize,
) -> Result<Vec<RankedEmail>> {
    rank_emails_from_db_with_options(
        db,
        credential_id,
        limit,
        &RankingPipelineOptions::default(),
        None,
    )
    .await
}

/// Rank emails with full detail (for CLI verbose mode), with modular pass toggles.
pub async fn rank_emails_from_db_detailed_with_options(
    db: &crate::database::Database,
    credential_id: Option<i64>,
    limit: usize,
    options: &RankingPipelineOptions,
    search_index: Option<&TantivySearchIndex>,
) -> Result<Vec<(MultiFactorRankedEmail, Vec<String>, Vec<String>)>> {
    let context = build_ranking_context_with_options(
        db.async_connection.clone(),
        credential_id,
        options.enable_sender_reputation_pass,
    )
    .await?;

    let candidates = candidate_email_pass(db, credential_id, limit, options, search_index).await?;
    let filtered_emails = financial_filter_pass(candidates);
    let multi_ranked = rank_pass(filtered_emails.clone(), &context, options);

    let email_map: HashMap<i64, _> = filtered_emails.into_iter().map(|e| (e.id, e)).collect();

    let detailed: Vec<_> = multi_ranked
        .into_iter()
        .take(limit)
        .map(|multi| {
            if let Some(email) = email_map.get(&multi.email_id) {
                let subject = email.subject.as_deref().unwrap_or("");
                let body_text = email.body_text.as_deref().unwrap_or("");
                let body_html = email.body_html.as_deref().unwrap_or("");
                let combined = format!("{} {} {}", subject, body_text, body_html);
                let combined_lower = combined.to_lowercase();

                let keywords: Vec<String> = FINANCIAL_KEYWORDS
                    .iter()
                    .filter(|k| combined_lower.contains(**k))
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

/// Backward-compatible wrapper with defaults.
pub async fn rank_emails_from_db_detailed(
    db: &crate::database::Database,
    credential_id: Option<i64>,
    limit: usize,
) -> Result<Vec<(MultiFactorRankedEmail, Vec<String>, Vec<String>)>> {
    rank_emails_from_db_detailed_with_options(
        db,
        credential_id,
        limit,
        &RankingPipelineOptions::default(),
        None,
    )
    .await
}
