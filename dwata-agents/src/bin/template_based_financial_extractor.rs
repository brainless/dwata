use anyhow::{Context, Result};
use clap::Parser;
use config::{Config, File};
use rusqlite::{params, Connection};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;

use dwata_agents::storage::{InMemoryAgentStorage, Session};
use dwata_agents::template_financial_extractor::TemplateFinancialExtractorAgent;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::gemini::GeminiClient;
use nocodo_llm_sdk::models::gemini::GEMINI_3_FLASH_ID;
use nocodo_llm_sdk::models::ollama::MINISTRAL_3_3B_ID;
use nocodo_llm_sdk::models::openai::GPT_5_MINI_ID;
use nocodo_llm_sdk::ollama::OllamaClient;
use nocodo_llm_sdk::openai::OpenAIClient;

#[derive(Parser, Debug)]
#[command(
    name = "template-based-financial-extractor",
    about = "Generate a Jinja2 template from multiple emails assumed to share the same template, \
             then use an LLM agent to translate placeholder variables to financial field names.\n\n\
             Scans DB emails for a sender via --email-from, selects a cluster of similar \
             emails, then builds a support-based template that drops low-frequency noise."
)]
struct Cli {
    /// Sender email address to scan in DB (required)
    #[arg(long, required = true)]
    email_from: String,

    /// Max sender emails to scan from DB (most recent first)
    #[arg(long, default_value_t = 200)]
    max_db_emails: usize,

    /// Normalized word-edit distance threshold used to include emails in sender cluster
    #[arg(long, default_value_t = 0.35)]
    word_distance_threshold: f64,

    /// Skip the LLM agent step and only output the raw template
    #[arg(long, default_value_t = false)]
    template_only: bool,

    /// LLM provider to use
    #[arg(long, default_value = "gemini", value_parser = ["gemini", "openai", "ollama"])]
    provider: String,

    /// Model ID to use (provider-specific)
    #[arg(long)]
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiConfig {
    ai_provider_api_keys: Option<AiProviderApiKeysConfig>,
}

#[derive(Debug, Deserialize)]
struct AiProviderApiKeysConfig {
    gemini_api_key: Option<String>,
    openai_api_key: Option<String>,
}

/// Holds the subject and plain-text body extracted from an email.
struct Email {
    subject: String,
    body: String,
}

struct DbEmail {
    id: i64,
    subject: String,
    body: String,
}

struct TemplateDefaults {
    line_support: f64,
    word_support: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let env_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .try_init();

    let cli = Cli::parse();

    // 1. Load sender email cluster from DB
    let (sender_emails, defaults) = load_matching_emails_from_db(
        &cli.email_from,
        cli.max_db_emails,
        cli.word_distance_threshold,
    )?;
    let emails: Vec<Email> = sender_emails
        .into_iter()
        .map(|e| Email {
            subject: e.subject,
            body: e.body,
        })
        .collect();

    // 2. Build subject template
    let subjects: Vec<String> = emails.iter().map(|e| e.subject.clone()).collect();
    let mut placeholder_counter = 1usize;
    let subject_template = build_subject_template_with_support(
        &subjects,
        defaults.word_support,
        emails.len(),
        &mut placeholder_counter,
    );

    // 3. Build body template
    let bodies: Vec<String> = emails.iter().map(|e| e.body.clone()).collect();
    let body_template = build_template_word_mode_with_support(
        &bodies,
        defaults.line_support,
        defaults.word_support,
        emails.len(),
        &mut placeholder_counter,
    );

    let full_template = format!("Subject: {subject_template}\n---\n{body_template}");

    println!("=== Generated Template ===");
    println!("{full_template}");
    println!("==========================\n");

    if cli.template_only {
        return Ok(());
    }

    // 4. Run the LLM agent to translate placeholders
    let config = load_api_config()?;

    // Determine model based on provider
    let model = match cli.model {
        Some(ref m) => m.clone(),
        None => match cli.provider.as_str() {
            "gemini" => GEMINI_3_FLASH_ID.to_string(),
            "openai" => GPT_5_MINI_ID.to_string(),
            "ollama" => MINISTRAL_3_3B_ID.to_string(),
            _ => GEMINI_3_FLASH_ID.to_string(),
        },
    };

    println!("Using provider: {}", cli.provider);
    println!("Using model: {model}");

    let llm_client: Arc<dyn LlmClient> = match cli.provider.as_str() {
        "gemini" => {
            let api_key = config
                .ai_provider_api_keys
                .as_ref()
                .and_then(|keys| keys.gemini_api_key.as_ref())
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Missing gemini_api_key in dwata config"))?;
            Arc::new(GeminiClient::new(api_key)?)
        }
        "openai" => {
            let api_key = config
                .ai_provider_api_keys
                .as_ref()
                .and_then(|keys| keys.openai_api_key.as_ref())
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Missing openai_api_key in dwata config"))?;
            Arc::new(OpenAIClient::new(api_key)?)
        }
        "ollama" => Arc::new(OllamaClient::new()?),
        _ => {
            return Err(anyhow::anyhow!("Unsupported provider: {}", cli.provider));
        }
    };
    let storage: Arc<dyn dwata_agents::AgentStorage> = Arc::new(InMemoryAgentStorage::new());

    let session_id = storage
        .create_session(Session {
            id: None,
            agent_type: "template-financial-extractor".to_string(),
            objective: "Translate template placeholders to financial fields".to_string(),
            context_data: None,
            status: "running".to_string(),
            result: None,
        })
        .await?;

    let agent = TemplateFinancialExtractorAgent::new(
        llm_client,
        storage.clone(),
        model,
        full_template.clone(),
    );

    let result = match agent.execute(session_id).await {
        Ok(params) => {
            let _ = storage
                .update_session(Session {
                    id: Some(session_id),
                    agent_type: "template-financial-extractor".to_string(),
                    objective: String::new(),
                    context_data: None,
                    status: "completed".to_string(),
                    result: Some(serde_json::to_string(&params.to_map())?),
                })
                .await;
            params
        }
        Err(err) => {
            let _ = storage
                .update_session(Session {
                    id: Some(session_id),
                    agent_type: "template-financial-extractor".to_string(),
                    objective: String::new(),
                    context_data: None,
                    status: "failed".to_string(),
                    result: Some(err.to_string()),
                })
                .await;
            return Err(err);
        }
    };

    // 5. Apply translations to produce the final template
    let translations = result.to_map();
    let mut final_template = full_template;
    for (placeholder, value) in &translations {
        if let Some(field_template) = value {
            // Replace {{ placeholder_N }} with the field template
            let search = format!("{{{{ {placeholder} }}}}");
            final_template = final_template.replace(&search, field_template);
        }
    }

    println!("=== Translated Template ===");
    println!("{final_template}");
    println!("===========================\n");

    println!("=== Variable Mappings ===");
    for (placeholder, value) in &translations {
        match value {
            Some(field) => println!("  {placeholder} → {field}"),
            None => println!("  {placeholder} → (not a financial field)"),
        }
    }
    println!("=========================");

    Ok(())
}

fn load_api_config() -> Result<ApiConfig> {
    let config_path = if let Some(config_dir) = dirs::config_dir() {
        config_dir.join("dwata").join("api.toml")
    } else {
        PathBuf::from("api.toml")
    };

    if !config_path.exists() {
        return Err(anyhow::anyhow!(
            "Config file not found at {:?}. Run dwata-api once or create it.",
            config_path
        ));
    }

    let builder = Config::builder()
        .add_source(File::from(config_path))
        .build()?;
    let config: ApiConfig = builder.try_deserialize()?;
    Ok(config)
}

fn load_matching_emails_from_db(
    sender_email: &str,
    max_db_emails: usize,
    threshold: f64,
) -> Result<(Vec<DbEmail>, TemplateDefaults)> {
    if !(0.0..=1.0).contains(&threshold) {
        return Err(anyhow::anyhow!(
            "--word-distance-threshold must be between 0.0 and 1.0"
        ));
    }

    let db_path = get_db_path()?;
    if !db_path.exists() {
        return Err(anyhow::anyhow!(
            "Database not found at {:?}. Run dwata-api and sync emails first.",
            db_path
        ));
    }

    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open SQLite database at {:?}", db_path))?;

    let max_db_emails_i64: i64 = max_db_emails
        .try_into()
        .context("--max-db-emails is too large")?;

    let mut stmt = conn.prepare(
        "SELECT id, COALESCE(subject, ''), COALESCE(body_text, '')
         FROM emails
         WHERE LOWER(from_address) = LOWER(?1)
         ORDER BY date_received DESC
         LIMIT ?2",
    )?;

    let rows = stmt.query_map(params![sender_email, max_db_emails_i64], |row| {
        Ok(DbEmail {
            id: row.get(0)?,
            subject: row.get(1)?,
            body: row.get(2)?,
        })
    })?;

    let mut candidates = Vec::new();
    for row in rows {
        let candidate = row?;
        if !candidate.subject.trim().is_empty() || !candidate.body.trim().is_empty() {
            candidates.push(candidate);
        }
    }

    if candidates.len() < 2 {
        return Err(anyhow::anyhow!(
            "Need at least 2 non-empty emails for sender '{}', found {}.",
            sender_email,
            candidates.len()
        ));
    }

    let seed = candidates
        .first()
        .ok_or_else(|| anyhow::anyhow!("No sender emails found"))?;
    let seed_text = comparable_text(seed);

    let mut scored: Vec<(f64, DbEmail)> = candidates
        .into_iter()
        .map(|email| {
            (
                normalized_word_edit_distance(&seed_text, &comparable_text(&email)),
                email,
            )
        })
        .filter(|(dist, _)| *dist <= threshold)
        .collect();

    scored.sort_by(|a, b| a.0.total_cmp(&b.0));
    let matching_count = scored.len();
    let max_template_emails = derive_max_template_emails(matching_count);
    let selected_defaults = derive_template_defaults(matching_count);

    let selected: Vec<DbEmail> = scored
        .into_iter()
        .take(max_template_emails)
        .map(|(_, email)| email)
        .collect();

    if selected.len() < 2 {
        return Err(anyhow::anyhow!(
            "Could not find at least two matching emails for sender '{}': found {} (threshold {:.3}).",
            sender_email,
            selected.len(),
            threshold
        ));
    }

    let ids: Vec<String> = selected.iter().map(|e| e.id.to_string()).collect();
    println!(
        "Selected {} sender emails for template generation (ids: {}). line_support={:.2}, word_support={:.2}",
        selected.len(),
        ids.join(", "),
        selected_defaults.line_support,
        selected_defaults.word_support
    );

    Ok((selected, selected_defaults))
}

fn derive_max_template_emails(matching_count: usize) -> usize {
    if matching_count >= 30 {
        24
    } else if matching_count >= 20 {
        18
    } else if matching_count >= 12 {
        12
    } else {
        matching_count
    }
}

fn derive_template_defaults(matching_count: usize) -> TemplateDefaults {
    if matching_count >= 20 {
        TemplateDefaults {
            line_support: 0.8,
            word_support: 0.8,
        }
    } else if matching_count >= 10 {
        TemplateDefaults {
            line_support: 0.75,
            word_support: 0.75,
        }
    } else if matching_count >= 5 {
        TemplateDefaults {
            line_support: 0.67,
            word_support: 0.67,
        }
    } else {
        TemplateDefaults {
            line_support: 0.5,
            word_support: 0.5,
        }
    }
}

fn support_count(total_emails: usize, support_ratio: f64) -> usize {
    ((total_emails as f64) * support_ratio).ceil().max(1.0) as usize
}

fn build_subject_template_with_support(
    subjects: &[String],
    word_support: f64,
    total_emails: usize,
    placeholder_counter: &mut usize,
) -> String {
    if subjects.is_empty() {
        return String::new();
    }
    build_token_support_template(
        &subjects.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        word_support,
        total_emails,
        "subject",
        placeholder_counter,
    )
}

fn build_template_word_mode_with_support(
    bodies: &[String],
    line_support: f64,
    word_support: f64,
    total_emails: usize,
    placeholder_counter: &mut usize,
) -> String {
    let required_line_support = support_count(total_emails, line_support);
    let all_lines: Vec<Vec<&str>> = bodies.iter().map(|b| b.lines().collect()).collect();
    let max_lines = all_lines.iter().map(|lines| lines.len()).max().unwrap_or(0);
    let mut template_lines = Vec::new();

    for line_idx in 0..max_lines {
        let versions: Vec<&str> = all_lines
            .iter()
            .filter_map(|lines| lines.get(line_idx).copied())
            .filter(|line| !line.trim().is_empty())
            .collect();

        if versions.len() < required_line_support {
            continue;
        }

        let line_template = build_token_support_template(
            &versions,
            word_support,
            total_emails,
            "placeholder",
            placeholder_counter,
        );
        if !line_template.trim().is_empty() {
            template_lines.push(line_template);
        }
    }

    template_lines.join("\n")
}

fn build_token_support_template(
    versions: &[&str],
    token_support: f64,
    total_emails: usize,
    placeholder_prefix: &str,
    placeholder_counter: &mut usize,
) -> String {
    let required_token_support = support_count(total_emails, token_support);
    let tokenized: Vec<Vec<&str>> = versions
        .iter()
        .map(|line| line.split_whitespace().collect())
        .collect();

    let max_tokens = tokenized.iter().map(|t| t.len()).max().unwrap_or(0);
    let mut out_tokens: Vec<String> = Vec::new();
    let mut in_placeholder_run = false;

    for token_idx in 0..max_tokens {
        let mut bucket: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for tokens in &tokenized {
            if let Some(token) = tokens.get(token_idx) {
                *bucket.entry(token).or_insert(0) += 1;
            }
        }

        let best = bucket
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(token, count)| (token.to_string(), count));

        if let Some((token, count)) = best {
            if count >= required_token_support {
                out_tokens.push(token);
                in_placeholder_run = false;
                continue;
            }
        }

        if !in_placeholder_run {
            out_tokens.push(format!(
                "{{{{ {}_{} }}}}",
                placeholder_prefix, placeholder_counter
            ));
            *placeholder_counter += 1;
            in_placeholder_run = true;
        }
    }

    out_tokens.join(" ")
}

fn get_db_path() -> Result<PathBuf> {
    let data_dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine local data directory"))?;
    Ok(data_dir.join("dwata").join("db.sqlite"))
}

fn comparable_text(email: &DbEmail) -> String {
    format!("{}\n{}", email.subject, email.body)
}

fn normalized_word_edit_distance(a: &str, b: &str) -> f64 {
    let a_tokens: Vec<&str> = a.split_whitespace().collect();
    let b_tokens: Vec<&str> = b.split_whitespace().collect();

    if a_tokens.is_empty() && b_tokens.is_empty() {
        return 0.0;
    }

    let dist = levenshtein_words(&a_tokens, &b_tokens) as f64;
    let scale = a_tokens.len().max(b_tokens.len()) as f64;
    dist / scale
}

fn levenshtein_words(a: &[&str], b: &[&str]) -> usize {
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }

    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr: Vec<usize> = vec![0; b.len() + 1];

    for (i, a_tok) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, b_tok) in b.iter().enumerate() {
            let cost = if a_tok == b_tok { 0 } else { 1 };
            curr[j + 1] = (curr[j] + 1).min(prev[j + 1] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b.len()]
}

/// Build a Jinja2 template for the subject line by word-diffing across all
/// email subjects.
fn build_subject_template(subjects: &[String]) -> String {
    if subjects.is_empty() {
        return String::new();
    }
    // All identical → return as-is
    if subjects.iter().all(|s| s == &subjects[0]) {
        return subjects[0].clone();
    }

    let tokenized: Vec<Vec<&str>> = subjects
        .iter()
        .map(|s| s.split_whitespace().collect())
        .collect();

    let mut common = lcs(&tokenized[0], &tokenized[1]);
    for tokens in &tokenized[2..] {
        let refs: Vec<&str> = common.iter().map(|s| s.as_str()).collect();
        common = lcs(&refs, tokens);
    }

    // Align against first subject, replacing gaps with placeholders
    let mut parts: Vec<String> = Vec::new();
    let mut counter: usize = 1;
    let mut ti = 0usize;
    let mut ci = 0usize;

    while ti < tokenized[0].len() {
        if ci < common.len() && tokenized[0][ti] == common[ci].as_str() {
            parts.push(tokenized[0][ti].to_string());
            ci += 1;
            ti += 1;
        } else {
            while ti < tokenized[0].len()
                && (ci >= common.len() || tokenized[0][ti] != common[ci].as_str())
            {
                ti += 1;
            }
            parts.push(format!("{{{{ subject_{} }}}}", counter));
            counter += 1;
        }
    }

    parts.join(" ")
}

// ---------------------------------------------------------------------------
// Word-mode template generation
// ---------------------------------------------------------------------------

/// Splits every line into words, diffs at word level, and replaces variable
/// words with placeholders. This gives finer-grained templates.
fn build_template_word_mode(bodies: &[String]) -> String {
    // Split into lines, then process line-by-line across all emails.
    let all_lines: Vec<Vec<&str>> = bodies.iter().map(|b| b.lines().collect()).collect();

    // First find the common *lines* skeleton (exact-match lines) so we know
    // the structural anchors.
    let mut common_lines = lcs(&all_lines[0], &all_lines[1]);
    for lines in &all_lines[2..] {
        let refs: Vec<&str> = common_lines.iter().map(|s| s.as_str()).collect();
        common_lines = lcs(&refs, lines);
    }

    // Build alignment: map common_lines indices → first-email line indices.
    let alignment = align_multiple(&all_lines, &common_lines);

    let mut template_lines: Vec<String> = Vec::new();
    let mut placeholder_counter: usize = 1;

    let mut prev_idx: Option<usize> = None;
    for &(common_idx, first_email_idx) in &alignment {
        // Process the gap between the previous anchor and this one.
        let gap_start = prev_idx.map(|p| p + 1).unwrap_or(0);
        if gap_start < first_email_idx {
            process_gap(
                &all_lines,
                &alignment,
                common_idx,
                gap_start,
                first_email_idx,
                &mut template_lines,
                &mut placeholder_counter,
            );
        }
        prev_idx = Some(first_email_idx);

        // The anchor line is identical across all emails – keep as-is.
        template_lines.push(common_lines[common_idx].clone());
    }

    // Trailing gap after the last anchor.
    if let Some(last) = prev_idx {
        let first_len = all_lines[0].len();
        if last + 1 < first_len {
            process_gap(
                &all_lines,
                &alignment,
                common_lines.len(), // sentinel: past-the-end
                last + 1,
                first_len,
                &mut template_lines,
                &mut placeholder_counter,
            );
        }
    } else {
        // No common lines at all – word-diff every line positionally.
        let max_lines = all_lines.iter().map(|l| l.len()).max().unwrap_or(0);
        for li in 0..max_lines {
            let versions: Vec<&str> = all_lines
                .iter()
                .filter_map(|lines| lines.get(li).copied())
                .collect();
            if versions.len() >= 2 {
                template_lines.push(word_diff_template(&versions, &mut placeholder_counter));
            } else if let Some(v) = versions.first() {
                template_lines.push(format!("{{{{ placeholder_{} }}}}", placeholder_counter));
                let _ = v; // suppress unused warning
                placeholder_counter += 1;
            }
        }
    }

    template_lines.join("\n")
}

/// Process a gap region (lines between two anchors).  Instead of replacing
/// each gap line with a whole-line placeholder, we align the gap lines
/// across emails positionally and word-diff each pair.
fn process_gap(
    all_lines: &[Vec<&str>],
    alignment: &[(usize, usize)],
    next_common_idx: usize,
    gap_start_in_first: usize,
    gap_end_in_first: usize,
    template_lines: &mut Vec<String>,
    placeholder_counter: &mut usize,
) {
    // For each email, find the corresponding gap region.  The gap for email
    // E sits between the anchor *before* this gap and the anchor *at*
    // next_common_idx.  We find those boundary positions per email.
    let gap_slices: Vec<&[&str]> = collect_gap_slices(
        all_lines,
        alignment,
        next_common_idx,
        gap_start_in_first,
        gap_end_in_first,
    );

    let max_gap_len = gap_slices.iter().map(|s| s.len()).max().unwrap_or(0);

    for li in 0..max_gap_len {
        let versions: Vec<&str> = gap_slices
            .iter()
            .filter_map(|slice| slice.get(li).copied())
            .collect();

        if versions.len() >= 2 {
            template_lines.push(word_diff_template(&versions, placeholder_counter));
        } else {
            // Line only exists in some emails – whole-line placeholder.
            template_lines.push(format!("{{{{ placeholder_{} }}}}", placeholder_counter));
            *placeholder_counter += 1;
        }
    }
}

/// For every email, extract the slice of lines in the gap region that
/// corresponds to the gap in the first email between `gap_start..gap_end`.
fn collect_gap_slices<'a>(
    all_lines: &'a [Vec<&'a str>],
    alignment: &[(usize, usize)],
    next_common_idx: usize,
    gap_start_in_first: usize,
    gap_end_in_first: usize,
) -> Vec<&'a [&'a str]> {
    // For the first email, the slice is simply [gap_start..gap_end).
    // For other emails, we need to find the positions of the surrounding
    // anchors and extract the lines between them.

    let mut result: Vec<&[&str]> = Vec::new();

    for (email_idx, email_lines) in all_lines.iter().enumerate() {
        if email_idx == 0 {
            result.push(&email_lines[gap_start_in_first..gap_end_in_first]);
            continue;
        }

        // Find position of the previous anchor in this email (the anchor
        // just before next_common_idx).
        let prev_anchor_pos = if next_common_idx > 0 {
            find_anchor_pos_in_email(
                email_lines,
                alignment,
                next_common_idx - 1,
                all_lines[0].as_slice(),
            )
        } else {
            None
        };

        // Find position of the next anchor in this email.
        let next_anchor_pos = if next_common_idx < alignment.len() {
            find_anchor_pos_in_email(
                email_lines,
                alignment,
                next_common_idx,
                all_lines[0].as_slice(),
            )
        } else {
            None
        };

        let start = prev_anchor_pos.map(|p| p + 1).unwrap_or(0);
        let end = next_anchor_pos.unwrap_or(email_lines.len());
        if start <= end && end <= email_lines.len() {
            result.push(&email_lines[start..end]);
        } else {
            result.push(&[]);
        }
    }

    result
}

/// Find the position of a given anchor (by common_idx) in a specific email.
/// The anchor's text is `all_lines[0][alignment[common_idx].1]`.  We search
/// forward from the previous anchor's position in this email.
fn find_anchor_pos_in_email(
    email_lines: &[&str],
    alignment: &[(usize, usize)],
    common_idx: usize,
    first_email_lines: &[&str],
) -> Option<usize> {
    if common_idx >= alignment.len() {
        return None;
    }
    let (_, first_pos) = alignment[common_idx];
    let anchor_text = first_email_lines[first_pos];

    // Determine search start: after the previous anchor in this email.
    let search_start = if common_idx > 0 {
        // Recursively find prev anchor position, then start after it.
        find_anchor_pos_in_email(email_lines, alignment, common_idx - 1, first_email_lines)
            .map(|p| p + 1)
            .unwrap_or(0)
    } else {
        0
    };

    for i in search_start..email_lines.len() {
        if email_lines[i] == anchor_text {
            return Some(i);
        }
    }
    None
}

/// Given several versions of the same logical line, diff at word level and
/// produce a template line.
fn word_diff_template(line_versions: &[&str], counter: &mut usize) -> String {
    let tokenized: Vec<Vec<&str>> = line_versions
        .iter()
        .map(|l| l.split_whitespace().collect())
        .collect();

    // Pairwise LCS across all versions
    let mut common_words = lcs(&tokenized[0], &tokenized[1]);
    for tokens in &tokenized[2..] {
        let refs: Vec<&str> = common_words.iter().map(|s| s.as_str()).collect();
        common_words = lcs(&refs, tokens);
    }

    // Align against the first version
    let mut result_parts: Vec<String> = Vec::new();
    let mut ti = 0usize; // index into tokenized[0]
    let mut ci = 0usize; // index into common_words

    while ti < tokenized[0].len() {
        if ci < common_words.len() && tokenized[0][ti] == common_words[ci].as_str() {
            result_parts.push(tokenized[0][ti].to_string());
            ci += 1;
            ti += 1;
        } else {
            // Consume all non-common words as a single placeholder
            let mut gap = Vec::new();
            while ti < tokenized[0].len()
                && (ci >= common_words.len() || tokenized[0][ti] != common_words[ci].as_str())
            {
                gap.push(tokenized[0][ti]);
                ti += 1;
            }
            if !gap.is_empty() {
                result_parts.push(format!("{{{{ placeholder_{} }}}}", counter));
                *counter += 1;
            }
        }
    }

    // If there are remaining common words (shouldn't happen in a correct LCS)
    // just append them.
    while ci < common_words.len() {
        result_parts.push(common_words[ci].clone());
        ci += 1;
    }

    result_parts.join(" ")
}

// ---------------------------------------------------------------------------
// Alignment helpers
// ---------------------------------------------------------------------------

/// Build an alignment of common_lines indices → first-email line indices.
fn align_multiple(all_lines: &[Vec<&str>], common_lines: &[String]) -> Vec<(usize, usize)> {
    let first = &all_lines[0];
    let mut result = Vec::new();
    let mut fi = 0usize;
    for (ci, cl) in common_lines.iter().enumerate() {
        while fi < first.len() {
            if first[fi] == cl.as_str() {
                result.push((ci, fi));
                fi += 1;
                break;
            }
            fi += 1;
        }
    }
    result
}

// ---------------------------------------------------------------------------
// LCS (Longest Common Subsequence)
// ---------------------------------------------------------------------------

/// Classic DP-based LCS that works on slices of string-like items.
fn lcs<T: AsRef<str>>(a: &[T], b: &[T]) -> Vec<String> {
    let m = a.len();
    let n = b.len();

    // Build DP table
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if a[i - 1].as_ref() == b[j - 1].as_ref() {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Back-track to recover the subsequence
    let mut result = Vec::new();
    let (mut i, mut j) = (m, n);
    while i > 0 && j > 0 {
        if a[i - 1].as_ref() == b[j - 1].as_ref() {
            result.push(a[i - 1].as_ref().to_string());
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] >= dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }

    result.reverse();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lcs_identical() {
        let a = vec!["hello", "world"];
        let b = vec!["hello", "world"];
        assert_eq!(lcs(&a, &b), vec!["hello", "world"]);
    }

    #[test]
    fn test_lcs_partial() {
        let a = vec!["a", "b", "c", "d"];
        let b = vec!["a", "x", "c", "y"];
        assert_eq!(lcs(&a, &b), vec!["a", "c"]);
    }

    #[test]
    fn test_lcs_no_common() {
        let a = vec!["a", "b"];
        let b = vec!["c", "d"];
        let result = lcs(&a, &b);
        assert!(result.is_empty());
    }

    #[test]
    fn test_word_mode_template() {
        let email1 =
            "Dear Customer,\nYour payment of $100.00 was received.\nThank you.".to_string();
        let email2 =
            "Dear Customer,\nYour payment of $250.00 was received.\nThank you.".to_string();

        let template = build_template_word_mode(&[email1, email2]);
        assert!(template.contains("Dear Customer,"));
        assert!(template.contains("Thank you."));
        // The amount should be replaced with a placeholder
        assert!(template.contains("placeholder_"));
        assert!(!template.contains("$100.00"));
        assert!(!template.contains("$250.00"));
    }

    #[test]
    fn test_three_emails() {
        let e1 = "Hello,\nYour balance is $100.\nAccount: 111\nBye.".to_string();
        let e2 = "Hello,\nYour balance is $200.\nAccount: 222\nBye.".to_string();
        let e3 = "Hello,\nYour balance is $300.\nAccount: 333\nBye.".to_string();

        let template = build_template_word_mode(&[e1, e2, e3]);
        assert!(template.contains("Hello,"));
        assert!(template.contains("Bye."));
        assert!(template.contains("placeholder_"));
    }

    #[test]
    fn test_word_level_precision_in_gap_lines() {
        // "Amount: $7.3" vs "Amount: $1.78" should produce
        // "Amount: {{ placeholder_N }}" not "{{ placeholder_N }}"
        let e1 = "Hello,\nAmount: $7.3\nBye.".to_string();
        let e2 = "Hello,\nAmount: $1.78\nBye.".to_string();

        let template = build_template_word_mode(&[e1, e2]);
        assert!(
            template.contains("Amount:"),
            "template should keep 'Amount:'"
        );
        assert!(
            template.contains("Amount: {{ placeholder_"),
            "template should be 'Amount: {{{{ placeholder_N }}}}', got:\n{template}"
        );
        assert!(!template.contains("$7.3"));
        assert!(!template.contains("$1.78"));
    }

    #[test]
    fn test_subject_template_identical() {
        let subjects = vec![
            "Payment received".to_string(),
            "Payment received".to_string(),
        ];
        assert_eq!(build_subject_template(&subjects), "Payment received");
    }

    #[test]
    fn test_subject_template_variable() {
        let subjects = vec![
            "Payment of $100 received".to_string(),
            "Payment of $500 received".to_string(),
        ];
        let tpl = build_subject_template(&subjects);
        assert!(tpl.contains("Payment"));
        assert!(tpl.contains("received"));
        assert!(tpl.contains("subject_"));
        assert!(!tpl.contains("$100"));
        assert!(!tpl.contains("$500"));
    }
}
