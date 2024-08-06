use chrono::{DateTime, Utc};
use log::error;
use reqwest;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::collections::HashSet;
use std::default::Default;
use ts_rs::TS;
use url::Url;

use crate::{error::DwataError, workspace::crud::CRUDRead};

use super::{AIIntegration, AIIntegrationFilters, AIProvider};

#[derive(Debug, Eq, Hash, PartialEq, Serialize, TS)]
#[ts(export)]
pub enum AIModelFeatures {
    TextGeneration,
    // Embedding,
    ImageRecognition,
    // ImageGeneration,
    // SpeechRecognition,
    // SpeechGeneration,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub enum AIModelDeveloper {
    SameAsProvider,
    // OpenAI,
    // Anthropic,
    Mistral,
    Meta,
    Google,
}

#[derive(Debug, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct AIModel {
    pub label: String,
    pub ai_provider: AIProvider,
    pub developer: Option<AIModelDeveloper>,
    pub features: HashSet<AIModelFeatures>,

    pub api_name: String,
    pub latest_version_api_name: Option<String>,
    // Optionally needed for Ollama
    pub tag: Option<String>,

    pub context_window: Option<i32>,
    // Prices are in US cents
    pub price_per_million_input_tokens: Option<i32>,
    pub price_per_million_output_tokens: Option<i32>,
    pub link_to_model_documentation: Option<Url>,
}

impl AIModel {
    pub async fn get_all_models() -> Vec<AIModel> {
        let mut result: Vec<AIModel> = vec![];
        result.extend(AIModel::get_models_for_openai());
        result.extend(AIModel::get_models_for_groq());
        result.extend(AIModel::get_models_for_anthropic());
        match AIModel::get_models_for_ollama().await {
            Ok(models) => result.extend(models),
            Err(err) => {
                error!("Could not get Ollama models\n Error: {}", err);
            }
        }
        result.extend(AIModel::get_models_for_mistral());
        result
    }

    pub async fn from_string(requested_ai_model: String) -> Result<Self, DwataError> {
        let (provider_name, model_name) = requested_ai_model.split_once("/").unwrap();
        let all_models = Self::get_all_models().await;
        // TODO: If AI provider is invalid, then return corresponding error
        let model = all_models
            .into_iter()
            .find(|x| x.ai_provider.to_string() == provider_name && x.api_name == model_name)
            .ok_or(DwataError::InvalidAIModel)?;
        Ok(model)
    }

    pub async fn get_integration(&self, db: &Pool<Sqlite>) -> Result<AIIntegration, DwataError> {
        let filters = AIIntegrationFilters {
            ai_provider: Some(self.ai_provider.to_string()),
            ..Default::default()
        };
        match AIIntegration::read_with_filter(filters, db).await {
            Ok(results) => match results.into_iter().nth(0) {
                Some(x) => Ok(x),
                None => {
                    error!("Could not find AI Integration");
                    Err(DwataError::CouldNotConnectToAIProvider)
                }
            },
            Err(x) => Err(x),
        }
    }
}

impl AIModel {
    pub fn get_models_for_openai() -> Vec<AIModel> {
        let mut models: Vec<AIModel> = vec![];
        models.push(AIModel {
            label: "GPT-3.5 Turbo".to_string(),
            ai_provider: AIProvider::OpenAI,
            developer: Some(AIModelDeveloper::SameAsProvider),
            features: HashSet::from([AIModelFeatures::TextGeneration]),
            api_name: "gpt-3.5-turbo".to_string(),
            latest_version_api_name: Some("gpt-3.5-turbo-0125".to_string()),
            context_window: Some(16_385),
            ..Default::default()
        });
        models.push(AIModel {
            label: "GPT-4 Turbo".to_string(),
            ai_provider: AIProvider::OpenAI,
            developer: Some(AIModelDeveloper::SameAsProvider),
            features: HashSet::from([
                AIModelFeatures::TextGeneration,
                AIModelFeatures::ImageRecognition,
            ]),
            api_name: "gpt-4-turbo".to_string(),
            latest_version_api_name: Some("gpt-4-turbo-2024-04-09".to_string()),
            context_window: Some(128_000),
            ..Default::default()
        });
        models.push(AIModel {
            label: "GPT-4o".to_string(),
            ai_provider: AIProvider::OpenAI,
            developer: Some(AIModelDeveloper::SameAsProvider),
            features: HashSet::from([
                AIModelFeatures::TextGeneration,
                AIModelFeatures::ImageRecognition,
            ]),
            api_name: "gpt-4o".to_string(),
            latest_version_api_name: Some("gpt-4o-2024-05-13".to_string()),
            context_window: Some(128_000),
            ..Default::default()
        });
        models
    }
}

impl AIModel {
    pub fn get_models_for_groq() -> Vec<AIModel> {
        let mut models: Vec<AIModel> = vec![];
        models.push(AIModel {
            label: "LLaMA3 8b".to_string(),
            ai_provider: AIProvider::Groq,
            developer: Some(AIModelDeveloper::Meta),
            features: HashSet::from([AIModelFeatures::TextGeneration]),
            api_name: "llama3-8b-8192".to_string(),
            context_window: Some(8_192),
            ..Default::default()
        });
        models.push(AIModel {
            label: "LLaMA3 70b".to_string(),
            ai_provider: AIProvider::Groq,
            developer: Some(AIModelDeveloper::Meta),
            features: HashSet::from([AIModelFeatures::TextGeneration]),
            api_name: "llama3-70b-8192".to_string(),
            context_window: Some(8_192),
            ..Default::default()
        });
        models.push(AIModel {
            label: "Mixtral 8x7b".to_string(),
            ai_provider: AIProvider::Groq,
            developer: Some(AIModelDeveloper::Mistral),
            features: HashSet::from([AIModelFeatures::TextGeneration]),
            api_name: "mixtral-8x7b-32768".to_string(),
            context_window: Some(32_768),
            ..Default::default()
        });
        models.push(AIModel {
            label: "Gemma 7b".to_string(),
            ai_provider: AIProvider::Groq,
            developer: Some(AIModelDeveloper::Google),
            features: HashSet::from([AIModelFeatures::TextGeneration]),
            api_name: "gemma-7b-it".to_string(),
            context_window: Some(8_192),
            ..Default::default()
        });
        models
    }
}

impl AIModel {
    pub fn get_models_for_anthropic() -> Vec<AIModel> {
        vec![]
    }
}

#[derive(Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub modified_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct OllamaModelsAPIResponse {
    pub models: Vec<OllamaModel>,
}

impl AIModel {
    pub async fn get_models_for_ollama() -> Result<Vec<AIModel>, DwataError> {
        // We get the list of models from the Ollama API running on localhost
        let mut models: Vec<AIModel> = vec![];
        let ollama_models: OllamaModelsAPIResponse =
            reqwest::get("http://localhost:11434/api/tags")
                .await?
                .json::<OllamaModelsAPIResponse>()
                .await?;
        for model in ollama_models.models {
            models.push(AIModel {
                label: model.name.clone(),
                ai_provider: AIProvider::Ollama,
                api_name: model.name.clone(),
                ..Default::default()
            });
        }
        Ok(models)
    }
}

impl AIModel {
    pub fn get_models_for_mistral() -> Vec<AIModel> {
        vec![]
    }
}
