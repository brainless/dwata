use anyhow::{Context, Result};
use clap::Parser;
use config::{Config, File};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;

use dwata_agents::storage::{InMemoryAgentStorage, Session};
use dwata_agents::template_financial_extractor::TemplateFinancialExtractorAgent;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::gemini::GeminiClient;
use nocodo_llm_sdk::models::gemini::GEMINI_3_FLASH_ID;

#[derive(Parser, Debug)]
#[command(
    name = "template-based-financial-extractor",
    about = "Generate a Jinja2 template from multiple emails assumed to share the same template, \
             then use an LLM agent to translate placeholder variables to financial field names.\n\n\
             Accepts two or more paths to .eml files, diffs them to find common text, and \
             replaces variable segments with {{ placeholder_N }} Jinja2 variables."
)]
struct Cli {
    /// Paths to two or more .eml (or plain-text) email files
    #[arg(required = true, num_args = 2..)]
    email_files: Vec<PathBuf>,

    /// Skip the LLM agent step and only output the raw template
    #[arg(long, default_value_t = false)]
    template_only: bool,
}

#[derive(Debug, Deserialize)]
struct ApiConfig {
    ai_provider_api_keys: Option<AiProviderApiKeysConfig>,
}

#[derive(Debug, Deserialize)]
struct AiProviderApiKeysConfig {
    gemini_api_key: Option<String>,
}

/// Holds the subject and plain-text body extracted from an email.
struct Email {
    subject: String,
    body: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let env_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .try_init();

    let cli = Cli::parse();

    // 1. Load subject + plain-text body from every email
    let emails: Vec<Email> = cli
        .email_files
        .iter()
        .map(|path| load_email(path))
        .collect::<Result<Vec<_>>>()?;

    // 2. Build subject template
    let subjects: Vec<String> = emails.iter().map(|e| e.subject.clone()).collect();
    let subject_template = build_subject_template(&subjects);

    // 3. Build body template
    let bodies: Vec<String> = emails.iter().map(|e| e.body.clone()).collect();
    let body_template = build_template_word_mode(&bodies);

    let full_template = format!("Subject: {subject_template}\n---\n{body_template}");

    println!("=== Generated Template ===");
    println!("{full_template}");
    println!("==========================\n");

    if cli.template_only {
        return Ok(());
    }

    // 4. Run the LLM agent to translate placeholders
    let config = load_api_config()?;
    let api_key = config
        .ai_provider_api_keys
        .as_ref()
        .and_then(|keys| keys.gemini_api_key.as_ref())
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Missing gemini_api_key in dwata config"))?;

    let model = GEMINI_3_FLASH_ID.to_string();
    println!("Using model: {model}");

    let llm_client: Arc<dyn LlmClient> = Arc::new(GeminiClient::new(api_key)?);
    let storage: Arc<dyn dwata_agents::AgentStorage> =
        Arc::new(InMemoryAgentStorage::new());

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
// Email loading – extracts subject and plain-text body only
// ---------------------------------------------------------------------------

fn load_email(path: &PathBuf) -> Result<Email> {
    let bytes =
        std::fs::read(path).with_context(|| format!("Failed to read file {:?}", path))?;

    // Try parsing as .eml first
    let parser = mail_parser::MessageParser::default();
    if let Some(parsed) = parser.parse(&bytes) {
        let subject = parsed.subject().unwrap_or("").to_string();
        let body = parsed
            .body_text(0)
            .map(|s| s.to_string())
            .unwrap_or_default();
        if !body.is_empty() || !subject.is_empty() {
            return Ok(Email { subject, body });
        }
    }

    // Fallback: treat the whole file as plain text (no subject)
    let text = String::from_utf8(bytes.clone())
        .or_else(|_| Ok::<String, anyhow::Error>(String::from_utf8_lossy(&bytes).to_string()))
        .context("Failed to decode file as UTF-8")?;
    Ok(Email {
        subject: String::new(),
        body: text,
    })
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
            find_anchor_pos_in_email(email_lines, alignment, next_common_idx - 1, all_lines[0].as_slice())
        } else {
            None
        };

        // Find position of the next anchor in this email.
        let next_anchor_pos = if next_common_idx < alignment.len() {
            find_anchor_pos_in_email(email_lines, alignment, next_common_idx, all_lines[0].as_slice())
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
                && (ci >= common_words.len()
                    || tokenized[0][ti] != common_words[ci].as_str())
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
fn align_multiple(
    all_lines: &[Vec<&str>],
    common_lines: &[String],
) -> Vec<(usize, usize)> {
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
        let email1 = "Dear Customer,\nYour payment of $100.00 was received.\nThank you.".to_string();
        let email2 = "Dear Customer,\nYour payment of $250.00 was received.\nThank you.".to_string();

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
        assert!(template.contains("Amount:"), "template should keep 'Amount:'");
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
