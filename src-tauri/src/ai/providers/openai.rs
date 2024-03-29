use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Usage {
    pub(crate) prompt_tokens: i64,
    pub(crate) completion_tokens: i64,
    pub(crate) total_tokens: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Message {
    pub(crate) role: String,
    pub(crate) content: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct ChatCompletionChoice {
    pub(crate) index: i64,
    pub(crate) message: Message,
    pub(crate) logprobs: Option<LogProb>,
    pub(crate) finish_reason: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct OpenAIChatResponse {
    pub(crate) id: String,
    pub(crate) object: String,
    pub(crate) created: i64,
    pub(crate) model: String,
    pub(crate) system_fingerprint: String,
    pub(crate) choices: Vec<ChatCompletionChoice>,
    pub(crate) usage: Usage,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct TopLogProb {
    pub(crate) token: String,
    pub(crate) logprob: i64,
    pub(crate) bytes: Option<Vec<i64>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct LogProbContent {
    pub(crate) token: String,
    pub(crate) logprob: i64,
    pub(crate) bytes: Option<Vec<i64>>,
    pub(crate) top_logprobs: Vec<TopLogProb>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct LogProb {
    pub(crate) content: Option<Vec<LogProbContent>>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ChatRequestMessage {
    pub(crate) role: String,
    pub(crate) content: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct OpenAIChatRequest {
    pub(crate) model: String,
    pub(crate) messages: Vec<ChatRequestMessage>,
}
