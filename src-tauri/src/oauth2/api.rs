use super::{OAuth2App, OAuth2Provider};
use crate::content::content::Content;
use crate::content::form::{FormButton, FormButtonType};
use crate::content::{
    content::{ContentSpec, ContentType},
    form::FormField,
};
use crate::error::DwataError;
use crate::workspace::api::{Configuration, NextStep, Writable};
use sqlx::{Pool, Sqlite};

impl Writable for OAuth2App {
    fn initiate(_db: &Pool<Sqlite>) -> Result<NextStep, DwataError> {
        Ok(NextStep::Configure(Configuration {
            title: "OAuth2 credentials".to_string(),
            description: "OAuth2 credentials for third party services".to_string(),
            fields: vec![
                FormField {
                    name: "provider".to_string(),
                    label: Some("Provider".to_string()),
                    content_type: ContentType::SingleChoice,
                    content_spec: ContentSpec {
                        choices: Some(vec![(
                            OAuth2Provider::Google.to_string(),
                            "Google".to_string(),
                        )]),
                        ..ContentSpec::default()
                    },
                    default_value: Some(Content::Text("google".to_string())),
                    ..Default::default()
                },
                FormField::text_field("clientId", "Client ID"),
                FormField::text_field("clientSecret", "Client Secret"),
            ],
            buttons: vec![FormButton {
                button_type: Some(FormButtonType::Submit),
                label: "Save OAuth2 app".to_string(),
            }],
            submit_implicitly: false,
        }))
    }
}
