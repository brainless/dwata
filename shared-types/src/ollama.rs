use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaStatusResponse {
    pub running: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaModelDetails {
    pub parent_model: Option<String>,
    pub format: String,
    pub family: String,
    pub families: Option<Vec<String>>,
    pub parameter_size: String,
    pub quantization_level: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaModelInfo {
    pub name: String,
    pub model: String,
    pub modified_at: String,
    pub size: u64,
    pub digest: String,
    pub details: OllamaModelDetails,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaModelsResponse {
    pub models: Vec<OllamaModelInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaPullModelRequest {
    pub model: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaPullModelResponse {
    pub status: String,
    pub digest: Option<String>,
    pub total: Option<u64>,
    pub completed: Option<u64>,
}
