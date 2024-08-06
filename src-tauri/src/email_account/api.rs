use super::{EmailAccount, EmailProvider};
use crate::content::content::{Content, ContentSpec, ContentType, TextType};
use crate::content::form::{FormButton, FormButtonType, FormField};
use crate::error::DwataError;
use crate::oauth2::helpers::get_google_oauth2_authorize_url;
use crate::oauth2::{OAuth2App, OAuth2Provider};
use crate::workspace::api::{Configuration, NextStep, Writable};
use crate::workspace::crud::CRUDRead;
use crate::workspace::ModuleDataCreateUpdate;
use sqlx::{Pool, Sqlite};
use std::str::FromStr;

impl EmailAccount {
    fn get_initial_form_fields() -> Vec<FormField> {
        let provider_spec: ContentSpec = ContentSpec {
            choices: Some(vec![
                (EmailProvider::Gmail.to_string(), "Gmail".to_string()),
                (
                    EmailProvider::ProtonMail.to_string(),
                    "ProtonMail".to_string(),
                ),
            ]),
            ..ContentSpec::default()
        };

        vec![FormField {
            name: "provider".to_string(),
            label: Some("Provider".to_string()),
            content_type: ContentType::SingleChoice,
            content_spec: provider_spec,
            ..Default::default()
        }]
    }

    fn get_fields_for_email_password_auth() -> Vec<FormField> {
        vec![
            FormField {
                name: "emailAddress".to_string(),
                label: Some("Email address".to_string()),
                content_spec: ContentSpec {
                    text_type: Some(TextType::Email),
                    ..ContentSpec::default()
                },
                ..Default::default()
            },
            FormField {
                name: "password".to_string(),
                label: Some("Password".to_string()),
                content_spec: ContentSpec {
                    text_type: Some(TextType::Password),
                    ..ContentSpec::default()
                },
                ..Default::default()
            },
        ]
    }

    async fn get_fields_for_oauth2_auth(
        email_provider: &EmailProvider,
        db: &Pool<Sqlite>,
    ) -> Vec<FormField> {
        // If provider is Gmail, then we show OAuth2 app list if there are more than one for Google,
        // else we add a hidden field with the OAuth2 app ID
        match email_provider {
            EmailProvider::Gmail => {
                match OAuth2App::read_all(db).await {
                    Ok(oauth2_apps) => {
                        // Check if we have only one OAuth2 app for this provider
                        let mut apps_iter = oauth2_apps
                            .iter()
                            .filter(|app| matches!(app.provider, OAuth2Provider::Google));
                        if apps_iter.clone().count() == 1 {
                            vec![FormField {
                                name: "oauth2AppId".to_string(),
                                content_type: ContentType::Text,
                                content_spec: ContentSpec::default(),
                                is_hidden: Some(true),
                                default_value: Some(Content::ID(apps_iter.next().unwrap().id)),
                                ..Default::default()
                            }]
                        } else {
                            vec![FormField {
                                name: "oauth2AppId".to_string(),
                                label: Some("Select OAuth2 app".to_string()),
                                content_type: ContentType::SingleChoice,
                                content_spec: ContentSpec {
                                    choices_from_function: Some(
                                        "get_oauth2_app_choice_list".to_string(),
                                    ),
                                    ..ContentSpec::default()
                                },
                                ..Default::default()
                            }]
                        }
                    }
                    Err(_) => {
                        vec![FormField {
                            name: "oauth2AppId".to_string(),
                            label: Some("Select OAuth2 app".to_string()),
                            content_type: ContentType::SingleChoice,
                            content_spec: ContentSpec {
                                choices_from_function: Some(
                                    "get_oauth2_app_choice_list".to_string(),
                                ),
                                ..ContentSpec::default()
                            },
                            ..Default::default()
                        }]
                    }
                }
            }
            _ => vec![],
        }
    }

    fn get_initial_configuration() -> Configuration {
        Configuration {
            title: "Email account".to_string(),
            description: "dwata uses IMAP to read your email inbox. Gmail needs OAuth2 credentials and Protonmail needs email address and password with Proton Mail Bridge.".to_string(),
            fields: Self::get_initial_form_fields(),
            buttons: vec![FormButton {
                button_type: Some(FormButtonType::Submit),
                label: "Save".to_string(),
            }],
            submit_implicitly: false
        }
    }
}

impl Writable for EmailAccount {
    fn initiate(_db: &Pool<Sqlite>) -> Result<NextStep, DwataError> {
        Ok(NextStep::Configure(Self::get_initial_configuration()))
    }

    async fn next_step(
        data: ModuleDataCreateUpdate,
        db: &Pool<Sqlite>,
    ) -> Result<NextStep, DwataError> {
        match data {
            ModuleDataCreateUpdate::EmailAccount(x) => {
                // Add the Google authorization URL as a field
                if x.provider.is_none()
                    || x.provider == Some("".to_string())
                    || x.oauth2_app_id.is_none()
                {
                    return Ok(NextStep::Continue);
                }

                let oauth2_app = OAuth2App::read_one_by_pk(x.oauth2_app_id.unwrap(), db).await?;
                let authorize_url = get_google_oauth2_authorize_url(
                    oauth2_app.client_id,
                    oauth2_app.client_secret.unwrap(),
                )
                .await?;

                Ok(NextStep::Configure(
                        Configuration {
                            title: "OAuth2 credentials".to_string(),
                            description: "Waiting for your authorization at the URL".to_string(),
                            fields: vec![FormField {
                                name: "googleAuthLink".to_string(),
                                label: Some("Link to authorize".to_string()),
                                description: Some("Please copy this link to your browser and authorize data access with Google".to_string()),
                                content_spec: ContentSpec {
                                    text_type: Some(TextType::Link),
                                    ..Default::default()
                                },
                                default_value: Some(Content::Text(authorize_url)),
                                is_required: Some(false),
                                is_editable: Some(false),
                                ..Default::default()
                            }],
                            buttons: vec![],
                            submit_implicitly: true
                        })
                    )
            }
            _ => Err(DwataError::NextStepNotAvailable),
        }
    }

    async fn on_change(
        data: ModuleDataCreateUpdate,
        db: &Pool<Sqlite>,
    ) -> Result<NextStep, DwataError> {
        match data {
            ModuleDataCreateUpdate::EmailAccount(x) => {
                // Check if the provider is set
                if x.provider.is_none() || x.provider == Some("".to_string()) {
                    return Ok(NextStep::Continue);
                }
                let provider = match EmailProvider::from_str(&x.provider.as_ref().unwrap()) {
                    Ok(x) => x,
                    Err(_) => return Err(DwataError::InvalidEmailProvider),
                };

                let mut fields = Self::get_initial_form_fields();
                match provider {
                    EmailProvider::Gmail => {
                        fields.extend(
                            Self::get_fields_for_oauth2_auth(&provider, db)
                                .await
                                .into_iter(),
                        );
                        Ok(NextStep::Configure(Configuration::new(
                            "Email Account",
                            "Email account details",
                            fields,
                        )))
                    }
                    EmailProvider::ProtonMail => {
                        fields.extend(Self::get_fields_for_email_password_auth().into_iter());
                        Ok(NextStep::Configure(Configuration::new(
                            "Email Account",
                            "Email account details",
                            fields,
                        )))
                    }
                }
            }
            _ => Err(DwataError::NextStepNotAvailable),
        }
    }
}
