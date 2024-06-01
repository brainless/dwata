use crate::content::{
    content::{ContentSpec, ContentType},
    form::FormField,
};
use crate::workspace::configuration::{Configurable, Configuration};

use super::{AIIntegration, AIProvider};

impl Configurable for AIIntegration {
    fn get_schema() -> Configuration {
        let ai_provider_spec: ContentSpec = ContentSpec {
            text_type: None,
            length_limits: None,
            choices: Some(vec![
                (String::from(AIProvider::OpenAI), "OpenAI".to_string()),
                (String::from(AIProvider::Groq), "Groq".to_string()),
                // (String::from(AIProvider::Anthropic), "Anthropic".to_string()),
                // (String::from(AIProvider::Ollama), "Ollama".to_string()),
                // (String::from(AIProvider::Mistral), "Mistral".to_string()),
            ]),
            is_prompt: None,
        };

        Configuration::new(
            "AI Integration",
            "API key based integration to an AI providers with your own API key.
            You can have more than one integration to the same provider.",
            vec![
                FormField::new(
                    "label",
                    "Label",
                    Some("An easy to remember label for this AI integration"),
                    ContentType::Text,
                    ContentSpec::default(),
                    Some(false),
                    Some(true),
                ),
                FormField::new(
                    "aiProvider",
                    "Select AI provider",
                    None,
                    ContentType::SingleChoice,
                    ai_provider_spec,
                    Some(true),
                    Some(true),
                ),
                FormField::new(
                    "apiKey",
                    "API key",
                    Some("You will find this in your AI providers account settings"),
                    ContentType::Text,
                    ContentSpec::default(),
                    Some(false),
                    Some(true),
                ),
            ],
        )
    }
}
