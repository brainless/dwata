use anyhow::Result;
use clap::Parser;
use config::{Config, File};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use dwata_agents::statement_extractor::{
    build_template, extract_transactions, infer_currency_from_intro, infer_field_mapping,
    read_statement_sheets,
};
use dwata_agents::storage::{InMemoryAgentStorage, Session};
use dwata_agents::template_financial_extractor::{
    TemplateFinancialExtractorAgent, TransactionField, TranslateVariablesParams,
};
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::gemini::GeminiClient;
use nocodo_llm_sdk::models::gemini::GEMINI_3_FLASH_ID;
use nocodo_llm_sdk::models::ollama::MINISTRAL_3_3B_ID;
use nocodo_llm_sdk::models::openai::GPT_5_MINI_ID;
use nocodo_llm_sdk::ollama::OllamaClient;
use nocodo_llm_sdk::openai::OpenAIClient;

#[derive(Parser, Debug)]
#[command(
    name = "statement-based-financial-extractor",
    about = "Extract bank statement rows from local XLSX files into typed transaction rows."
)]
struct Cli {
    #[arg(long, required = true)]
    input: PathBuf,

    #[arg(long)]
    sheet: Option<String>,

    #[arg(long, default_value_t = false)]
    template_only: bool,

    #[arg(long, default_value = "gemini", value_parser = ["gemini", "openai", "ollama"])]
    provider: String,

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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let sheets = read_statement_sheets(&cli.input, cli.sheet.as_deref())?;
    if sheets.is_empty() {
        return Err(anyhow::anyhow!(
            "No columnar statement table found in {}",
            cli.input.display()
        ));
    }

    let model = cli.model.unwrap_or_else(|| match cli.provider.as_str() {
        "gemini" => GEMINI_3_FLASH_ID.to_string(),
        "openai" => GPT_5_MINI_ID.to_string(),
        "ollama" => MINISTRAL_3_3B_ID.to_string(),
        _ => GEMINI_3_FLASH_ID.to_string(),
    });

    let llm_client = if cli.template_only {
        None
    } else {
        match create_client(&cli.provider) {
            Ok(client) => client,
            Err(err) => {
                eprintln!("LLM client setup failed, using heuristic mapping only: {err}");
                None
            }
        }
    };

    for sheet in sheets {
        let template = build_template(&sheet);
        println!("\n=== Sheet: {} ===", sheet.name);
        println!("Rows detected: {}", sheet.rows.len());
        println!("Headers: {}", sheet.headers.join(" | "));
        println!("Row template: {}", template.row_template);
        if !sheet.intro_lines.is_empty() {
            println!("Intro lines:");
            for line in sheet.intro_lines.iter().take(8) {
                println!("  {line}");
            }
        }

        if cli.template_only {
            continue;
        }

        let mut mapping = infer_field_mapping(&sheet.headers);
        let llm_mapping =
            llm_map_headers(&sheet.headers, &template, llm_client.clone(), &model).await;
        if let Ok(m) = llm_mapping {
            mapping = m;
            println!("Field mapping source: llm");
        } else if let Err(e) = llm_mapping {
            eprintln!("LLM mapping failed, using heuristic mapping: {e}");
            println!("Field mapping source: heuristic");
        }

        let default_currency = infer_currency_from_intro(&sheet.intro_lines);
        let txns = extract_transactions(&sheet, &mapping, default_currency.as_deref());

        println!("Extracted transactions: {}", txns.len());
        for (idx, t) in txns.iter().take(8).enumerate() {
            println!(
                "{:>3}. date={} amount={} currency={} vendor={} ref={}",
                idx + 1,
                t.transaction_date,
                t.amount,
                t.currency.clone().unwrap_or_else(|| "-".to_string()),
                t.vendor.clone().unwrap_or_else(|| "-".to_string()),
                t.transaction_reference
                    .clone()
                    .unwrap_or_else(|| "-".to_string())
            );
        }
    }

    Ok(())
}

async fn llm_map_headers(
    headers: &[String],
    template: &dwata_agents::statement_extractor::StatementTemplate,
    client: Option<Arc<dyn LlmClient>>,
    model: &str,
) -> Result<HashMap<String, TransactionField>> {
    let Some(client) = client else {
        return Err(anyhow::anyhow!("LLM disabled"));
    };
    let storage: Arc<dyn dwata_agents::AgentStorage> = Arc::new(InMemoryAgentStorage::new());
    let session_id = storage
        .create_session(Session {
            id: None,
            agent_type: "template-financial-extractor".to_string(),
            objective: "Map statement row placeholders to transaction fields".to_string(),
            context_data: None,
            status: "running".to_string(),
            result: None,
        })
        .await?;

    let template_text = format!(
        "Statement sheet: {}\nHeaders: {}\nRow: {}",
        template.sheet_name,
        headers.join(" | "),
        template.row_template
    );
    let agent = TemplateFinancialExtractorAgent::new(
        client,
        storage.clone(),
        model.to_string(),
        template_text,
    );
    let params: TranslateVariablesParams = agent.execute(session_id).await?;
    let placeholder_to_field = params.to_map();

    let mut map = HashMap::new();
    for (header, placeholder) in &template.placeholders {
        if let Some(Some(field)) = placeholder_to_field.get(placeholder) {
            map.insert(header.clone(), field.clone());
        }
    }
    Ok(map)
}

fn create_client(provider: &str) -> Result<Option<Arc<dyn LlmClient>>> {
    let config = load_api_config()?;
    let client: Arc<dyn LlmClient> = match provider {
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
        _ => return Err(anyhow::anyhow!("Unsupported provider: {provider}")),
    };
    Ok(Some(client))
}

fn load_api_config() -> Result<ApiConfig> {
    let config_path = if let Some(config_dir) = dirs::config_dir() {
        config_dir.join("dwata").join("api.toml")
    } else {
        PathBuf::from("api.toml")
    };

    if !config_path.exists() {
        return Err(anyhow::anyhow!(
            "Config file not found at {:?}",
            config_path
        ));
    }

    let builder = Config::builder()
        .add_source(File::from(config_path))
        .build()?;
    Ok(builder.try_deserialize()?)
}
