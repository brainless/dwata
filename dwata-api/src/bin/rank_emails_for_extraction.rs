use anyhow::{Context, Result};
use clap::Parser;
use dwata_api::database::credentials as credentials_db;
use dwata_api::email_ranking::tantivy_ranking::{
    list_top_sender_scores, rank_emails_from_db_detailed_with_options, RankingPipelineOptions,
};
use dwata_api::helpers::database::initialize_database;
use std::sync::Arc;

#[derive(Debug, Parser)]
#[command(
    name = "rank_emails_for_extraction",
    about = "Rank emails by financial relevance for KG extraction"
)]
struct Args {
    /// Credential ID to filter emails (if not provided, searches all accounts)
    #[arg(short, long)]
    credential_id: Option<i64>,

    /// Maximum number of emails to return
    #[arg(short, long, default_value_t = 50)]
    limit: usize,

    /// Show detailed scoring information
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// Minimum score threshold (0-100)
    #[arg(short, long, default_value_t = 0.0)]
    min_score: f64,

    /// Enable Tantivy keyword candidate pass before DB ranking
    #[arg(long, default_value_t = false)]
    use_tantivy_candidates: bool,

    /// Disable sender reputation pass
    #[arg(long, default_value_t = false)]
    disable_sender_reputation: bool,

    /// Candidate fetch multiplier (higher = broader candidate set)
    #[arg(long, default_value_t = 6)]
    candidate_multiplier: usize,

    /// Number of top senders to show (0 disables)
    #[arg(long, default_value_t = 25)]
    top_senders: usize,
}

// ---------------------------------------------------------------------------
// Simple ASCII table printer
// ---------------------------------------------------------------------------

fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    if rows.is_empty() {
        println!("  (no results)");
        return;
    }

    // Compute column widths
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    let separator: String = widths
        .iter()
        .map(|w| "─".repeat(w + 2))
        .collect::<Vec<_>>()
        .join("─┼─");
    let top_sep: String = widths
        .iter()
        .map(|w| "─".repeat(w + 2))
        .collect::<Vec<_>>()
        .join("─┬─");
    let bot_sep: String = widths
        .iter()
        .map(|w| "─".repeat(w + 2))
        .collect::<Vec<_>>()
        .join("─┴─");

    println!("  ┌─{}─┐", top_sep);
    let header_cells: String = headers
        .iter()
        .zip(widths.iter())
        .map(|(h, w)| format!(" {:width$} ", h, width = w))
        .collect::<Vec<_>>()
        .join("│");
    println!("  │{}│", header_cells);
    println!("  ├─{}─┤", separator);

    for row in rows {
        let cells: String = row
            .iter()
            .zip(widths.iter())
            .map(|(c, w)| format!(" {:width$} ", c, width = w))
            .collect::<Vec<_>>()
            .join("│");
        println!("  │{}│", cells);
    }

    println!("  └─{}─┘", bot_sep);
}

fn format_timestamp(ts: i64) -> String {
    use chrono::{DateTime, Local, TimeZone};
    let dt: DateTime<Local> = match Local.timestamp_millis_opt(ts) {
        chrono::LocalResult::Single(dt) => dt,
        _ => Local::now(),
    };
    dt.format("%Y-%m-%d %H:%M").to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("dwata_api=info".parse()?),
        )
        .with_target(false)
        .init();

    let args = Args::parse();

    let db = initialize_database().context("Failed to initialize database")?;

    // Get credential info for display
    let _credential_info = if let Some(cred_id) = args.credential_id {
        match credentials_db::get_credential(db.async_connection.clone(), cred_id).await {
            Ok(cred) => {
                println!(
                    "Searching emails for account: {} ({})",
                    cred.identifier, cred_id
                );
                Some((cred_id, cred.identifier))
            }
            Err(_) => {
                println!("Warning: Credential {} not found", cred_id);
                None
            }
        }
    } else {
        let creds = credentials_db::list_credentials(db.async_connection.clone(), false)
            .await
            .context("Failed to list credentials")?;
        println!("Searching emails across {} account(s)", creds.len());
        for cred in &creds {
            println!("  - {} ({})", cred.identifier, cred.id);
        }
        None
    };
    println!();

    // Rank emails with detailed information
    println!("Analyzing emails for financial content and user engagement...");
    println!(
        "  Factors: Financial (40%), User Engagement (30%), Conversation (20%), Recency (10%)"
    );
    println!(
        "  Passes: Tantivy Candidates={}, Sender Reputation={}",
        args.use_tantivy_candidates, !args.disable_sender_reputation
    );
    println!();

    let pipeline_options = RankingPipelineOptions {
        enable_tantivy_candidate_pass: args.use_tantivy_candidates,
        enable_sender_reputation_pass: !args.disable_sender_reputation,
        candidate_fetch_multiplier: args.candidate_multiplier.max(1),
    };

    let search_index = if args.use_tantivy_candidates {
        let (config, _) = dwata_api::config::ApiConfig::load()
            .map_err(|e| anyhow::anyhow!("Failed to load config: {e}"))?;
        let search_index_path = config
            .search
            .as_ref()
            .and_then(|s| s.index_path.as_ref())
            .map(std::path::PathBuf::from)
            .or_else(|| dirs::data_local_dir().map(|d| d.join("dwata").join("tantivy-index")))
            .context("Failed to resolve search index path")?;

        Some(Arc::new(
            dwata_api::search::tantivy::open_or_create_index_preserving(&search_index_path)
                .with_context(|| {
                    format!(
                        "Failed to open/create Tantivy index at {}",
                        search_index_path.display()
                    )
                })?,
        ))
    } else {
        None
    };

    let ranked_detailed = rank_emails_from_db_detailed_with_options(
        &db,
        args.credential_id,
        args.limit,
        &pipeline_options,
        search_index.as_deref(),
    )
    .await
    .context("Failed to rank emails")?;

    // Filter by minimum score if specified
    let filtered: Vec<_> = ranked_detailed
        .into_iter()
        .filter(|(email, _, _)| email.final_score >= args.min_score)
        .collect();

    if filtered.is_empty() {
        println!("No emails found matching financial criteria.");
        return Ok(());
    }

    println!("Found {} emails with financial content:\n", filtered.len());

    // Print summary table with factor breakdown
    let rows: Vec<Vec<String>> = filtered
        .iter()
        .map(|(email, _, _)| {
            vec![
                email.email_id.to_string(),
                truncate(&email.from_address, 35),
                truncate(email.subject.as_deref().unwrap_or("(no subject)"), 50),
                format_timestamp(email.date_received),
                format!("{:.1}", email.final_score),
                format!("{:.1}", email.factor_scores.sender_reputation),
                format!("{:.0}", email.factor_scores.user_engagement),
            ]
        })
        .collect();

    print_table(
        &[
            "ID", "From", "Subject", "Date", "Score", "Sender", "Engaged",
        ],
        &rows,
    );

    if !args.disable_sender_reputation && args.top_senders > 0 {
        println!();
        println!("Top {} Sender Scores:", args.top_senders);
        let top_senders = list_top_sender_scores(
            db.async_connection.clone(),
            args.credential_id,
            args.top_senders,
        )
        .await
        .context("Failed to compute top sender scores")?;

        let sender_rows: Vec<Vec<String>> = top_senders
            .iter()
            .map(|s| {
                vec![
                    truncate(&s.sender, 45),
                    s.emails_received.to_string(),
                    s.emails_replied.to_string(),
                    format_timestamp(s.most_recent_date_received_ms),
                    format!("{:.1}", s.score),
                ]
            })
            .collect();
        print_table(
            &["Sender", "Received", "Replied", "Most Recent", "Score"],
            &sender_rows,
        );
    }

    // Print detailed information if verbose mode
    if args.verbose {
        println!();
        println!("═══ Detailed Email Analysis ═══");

        for (email, keywords, amounts) in &filtered {
            println!();
            println!("Email ID: {}", email.email_id);
            println!("  From: {}", email.from_address);
            println!(
                "  Subject: {}",
                email.subject.as_deref().unwrap_or("(no subject)")
            );
            println!("  Date: {}", format_timestamp(email.date_received));
            println!();
            println!("  ═══ Multi-Factor Scores ═══");
            println!("    Base Email Score: {:.2}/100", email.base_score);
            println!("    Final Score:      {:.2}/100", email.final_score);
            println!(
                "    ├─ Financial Content: {:.1} (weight: 0.40)",
                email.factor_scores.financial_content
            );
            println!(
                "    ├─ User Engagement:   {:.1} (weight: 0.30)",
                email.factor_scores.user_engagement
            );
            println!(
                "    ├─ Conversation:      {:.1} (weight: 0.20)",
                email.factor_scores.conversation_thread
            );
            println!(
                "    ├─ Recency:           {:.1} (weight: 0.10)",
                email.factor_scores.recency
            );
            println!(
                "    └─ Sender Reputation: {:.1} (boosted separately)",
                email.factor_scores.sender_reputation
            );
            println!();
            println!(
                "  Keywords found ({}): {}",
                keywords.len(),
                if keywords.is_empty() {
                    "(none)".to_string()
                } else {
                    keywords.join(", ")
                }
            );
            println!(
                "  Amounts found ({}): {}",
                amounts.len(),
                if amounts.is_empty() {
                    "(none)".to_string()
                } else {
                    amounts.join(", ")
                }
            );
        }
    }

    // Print summary
    println!();
    println!("═══ Summary ═══");
    println!("Total matched emails: {}", filtered.len());

    if !filtered.is_empty() {
        let avg_score: f64 =
            filtered.iter().map(|(e, _, _)| e.final_score).sum::<f64>() / filtered.len() as f64;
        let highest_score = filtered
            .iter()
            .map(|(e, _, _)| e.final_score)
            .fold(0.0, f64::max);
        let lowest_score = filtered
            .iter()
            .map(|(e, _, _)| e.final_score)
            .fold(100.0, f64::min);

        println!(
            "Score range: {:.1} - {:.1} (avg: {:.1})",
            lowest_score, highest_score, avg_score
        );

        // Count engaged vs non-engaged
        let engaged_count = filtered
            .iter()
            .filter(|(e, _, _)| e.factor_scores.user_engagement > 0.0)
            .count();
        if engaged_count > 0 {
            println!("User replied to sender: {} email(s)", engaged_count);
        }

        // Group by sender
        let mut sender_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for (email, _, _) in &filtered {
            *sender_counts.entry(email.from_address.clone()).or_insert(0) += 1;
        }

        println!("\nTop senders:");
        let mut sender_vec: Vec<_> = sender_counts.iter().collect();
        sender_vec.sort_by(|a, b| b.1.cmp(a.1));
        for (sender, count) in sender_vec.iter().take(5) {
            println!("  {} - {} email(s)", truncate(sender, 40), count);
        }

        // Print email IDs for easy copy-paste to extract_kg_entities
        println!();
        println!("Email IDs for extraction:");
        let ids: Vec<String> = filtered
            .iter()
            .map(|(e, _, _)| e.email_id.to_string())
            .collect();
        println!("{}", ids.join(", "));
    }

    Ok(())
}
