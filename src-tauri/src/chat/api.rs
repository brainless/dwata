use crate::chat::Chat;
use crate::content::content::{ContentSpec, ContentType, TextType};
use crate::content::form::FormField;
use crate::error::DwataError;
use crate::workspace::api::{Configuration, NextStep, Writable};
use sqlx::{Pool, Sqlite};

impl Writable for Chat {
    fn initiate(_db: &Pool<Sqlite>) -> Result<NextStep, DwataError> {
        Ok(NextStep::Configure(Configuration::new(
            "Chat with AI",
            "Chat with AI models, sharing your objectives and let AI help you find solutions",
            vec![
                FormField::new(
                    "message",
                    Some("Message"),
                    Some("When asking an AI model, try to keep the message short.\
                    If you have a broad task then it is better to break it into tasks in your chat."),
                    ContentType::Text,
                    ContentSpec {
                        text_type: Some(TextType::MultiLine),
                        ..ContentSpec::default()
                    },
                    Some(true),
                    Some(true)
                ),
                FormField::new(
                    "requestedAiModel",
                    Some("AI Model"),
                    Some("What AI model would you like to use?"),
                    ContentType::SingleChoice,
                    ContentSpec {
                        choices_from_function: Some("get_ai_model_choice_list".to_string()),
                        ..ContentSpec::default()
                    },
                    Some(true),
                    Some(true)
                ),
            ]
        )))
    }
}
