use anyhow::{Context, Result};
use clap::{ArgGroup, Parser};
use config::{Config, File};
use rusqlite::Connection;
use serde::Deserialize;
use shared_types::FinancialPattern;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use dwata_agents::financial_extractor::FinancialExtractorAgent;
use dwata_agents::storage::{sqlite_storage::SqliteAgentStorage, Session};
use dwata_agents::tools::DwataToolExecutor;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::gemini::GeminiClient;
use nocodo_llm_sdk::models::gemini::GEMINI_3_FLASH_ID;

#[derive(Parser, Debug)]
#[command(name = "financial-extractor", about = "Run the financial extractor agent on an email")]
#[command(group(
    ArgGroup::new("input")
        .required(true)
        .args(["email_id", "eml_path", "email_body"]),
))]
struct Cli {
    /// Email ID from the dwata database
    #[arg(long, group = "input")]
    email_id: Option<i64>,

    /// Path to a .eml file
    #[arg(long, value_name = "PATH", group = "input")]
    eml_path: Option<PathBuf>,

    /// Raw email body text
    #[arg(long, group = "input")]
    email_body: Option<String>,

    /// Optional subject override (used with --email-body)
    #[arg(long)]
    subject: Option<String>,

    /// Override the Gemini model ID
    #[arg(long, default_value = GEMINI_3_FLASH_ID)]
    model: String,
}

#[derive(Debug, Deserialize, Clone)]
struct ApiConfig {
    api_keys: Option<ApiKeysConfig>,
    database: Option<DatabaseConfig>,
}

#[derive(Debug, Deserialize, Clone)]
struct ApiKeysConfig {
    gemini_api_key: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct DatabaseConfig {
    path: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let cli = Cli::parse();

    let (config, config_path) = load_api_config().context("Failed to load dwata API config")?;
    let api_key = config
        .api_keys
        .as_ref()
        .and_then(|keys| keys.gemini_api_key.as_ref())
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Missing gemini_api_key in config at {:?}", config_path))?;

    let db_path = resolve_db_path(&config)?;
    let conn = Arc::new(Mutex::new(
        Connection::open(&db_path).with_context(|| format!("Failed to open db at {:?}", db_path))?,
    ));

    let (subject, body) = match (&cli.email_id, &cli.eml_path, &cli.email_body) {
        (Some(email_id), None, None) => load_email_from_db(conn.clone(), *email_id)?,
        (None, Some(path), None) => load_email_from_eml(path)?,
        (None, None, Some(body)) => (cli.subject.clone().unwrap_or_default(), body.clone()),
        _ => unreachable!("clap enforces exactly one input"),
    };

    let patterns = load_active_patterns(conn.clone())?;
    let email_content = format!("{}\n\n{}", subject, body);

    let tool_executor = Arc::new(DwataToolExecutor::new(conn.clone(), email_content));
    let storage: Arc<dyn dwata_agents::AgentStorage> =
        Arc::new(SqliteAgentStorage::new(conn.clone()));

    let llm_client: Arc<dyn LlmClient> = Arc::new(GeminiClient::new(api_key)?);

    let session_id = storage
        .create_session(Session {
            id: None,
            agent_type: "financial-extractor".to_string(),
            objective: "Generate financial extraction pattern".to_string(),
            context_data: Some(
                serde_json::json!({
                    "source": if cli.email_id.is_some() {
                        "db"
                    } else if cli.eml_path.is_some() {
                        "eml"
                    } else {
                        "raw"
                    },
                    "email_id": cli.email_id,
                    "eml_path": cli.eml_path.as_ref().map(|p| p.display().to_string()),
                })
                .to_string(),
            ),
            status: "running".to_string(),
            result: None,
        })
        .await?;

    let agent = FinancialExtractorAgent::new(
        llm_client,
        storage.clone(),
        tool_executor,
        cli.model,
        subject,
        body,
        patterns,
    );

    let result = match agent.execute(session_id).await {
        Ok(result) => result,
        Err(err) => {
            let _ = storage
                .update_session(Session {
                    id: Some(session_id),
                    agent_type: "financial-extractor".to_string(),
                    objective: String::new(),
                    context_data: None,
                    status: "failed".to_string(),
                    result: Some(err.to_string()),
                })
                .await;
            return Err(err);
        }
    };

    let _ = storage
        .update_session(Session {
            id: Some(session_id),
            agent_type: "financial-extractor".to_string(),
            objective: String::new(),
            context_data: None,
            status: "completed".to_string(),
            result: Some(result.clone()),
        })
        .await;

    println!("{result}");
    Ok(())
}

fn init_tracing() {
    let env_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .try_init();
}

fn load_api_config() -> Result<(ApiConfig, PathBuf)> {
    let config_path = get_config_path();
    if !config_path.exists() {
        return Err(anyhow::anyhow!(
            "Config file not found at {:?}. Run dwata-api once or create it.",
            config_path
        ));
    }

    let builder = Config::builder()
        .add_source(File::from(config_path.clone()))
        .build()?;

    let config: ApiConfig = builder.try_deserialize()?;
    Ok((config, config_path))
}

fn get_config_path() -> PathBuf {
    if let Some(config_dir) = dirs::config_dir() {
        config_dir.join("dwata").join("api.toml")
    } else {
        PathBuf::from("api.toml")
    }
}

fn resolve_db_path(config: &ApiConfig) -> Result<PathBuf> {
    if let Some(path) = config
        .database
        .as_ref()
        .and_then(|db| db.path.as_ref())
    {
        return Ok(PathBuf::from(path));
    }

    let data_dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine local data directory"))?;
    Ok(data_dir.join("dwata").join("db.sqlite"))
}

fn load_email_from_eml(path: &Path) -> Result<(String, String)> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read .eml file at {:?}", path))?;
    let parser = mail_parser::MessageParser::default();
    let parsed = parser
        .parse(&bytes)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse .eml file"))?;

    let subject = parsed.subject().map(|s| s.to_string()).unwrap_or_default();
    let body = parsed
        .body_text(0)
        .map(|s| s.to_string())
        .or_else(|| parsed.body_html(0).map(|s| s.to_string()))
        .ok_or_else(|| anyhow::anyhow!("Email has no body text or HTML"))?;

    Ok((subject, body))
}

fn load_email_from_db(conn: Arc<Mutex<Connection>>, email_id: i64) -> Result<(String, String)> {
    let conn = conn.lock().unwrap();

    let mut stmt = conn.prepare(
        "SELECT subject, body_text, body_html FROM emails WHERE id = ?",
    )?;

    let row = stmt.query_row([email_id], |row| {
        let subject: Option<String> = row.get(0)?;
        let body_text: Option<String> = row.get(1)?;
        let body_html: Option<String> = row.get(2)?;
        Ok((subject, body_text, body_html))
    });

    let (subject, body_text, body_html) = match row {
        Ok(values) => values,
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            return Err(anyhow::anyhow!("No email found with id {}", email_id))
        }
        Err(err) => return Err(err.into()),
    };

    let body = body_text.or(body_html).ok_or_else(|| {
        anyhow::anyhow!("Email {} has no body_text or body_html", email_id)
    })?;

    Ok((subject.unwrap_or_default(), body))
}

fn load_active_patterns(conn: Arc<Mutex<Connection>>) -> Result<Vec<FinancialPattern>> {
    let conn = conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, regex_pattern, description, document_type, status, confidence,\n                amount_group, vendor_group, date_group, is_default, is_active,\n                match_count, last_matched_at, created_at, updated_at\n         FROM financial_patterns\n         WHERE is_active = true",
    )?;

    let patterns = stmt
        .query_map([], |row| {
            Ok(FinancialPattern {
                id: row.get(0)?,
                name: row.get(1)?,
                regex_pattern: row.get(2)?,
                description: row.get(3)?,
                document_type: row.get(4)?,
                status: row.get(5)?,
                confidence: row.get(6)?,
                amount_group: row.get::<_, i64>(7)? as usize,
                vendor_group: row.get::<_, Option<i64>>(8)?.map(|v| v as usize),
                date_group: row.get::<_, Option<i64>>(9)?.map(|v| v as usize),
                is_default: row.get(10)?,
                is_active: row.get(11)?,
                match_count: row.get(12)?,
                last_matched_at: row.get(13)?,
                created_at: row.get(14)?,
                updated_at: row.get(15)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(patterns)
}
