use sqlx::{Pool, Sqlite};

use super::UserAccount;
use crate::content::content::{ContentSpec, ContentType};
use crate::content::form::FormField;
use crate::error::DwataError;
use crate::workspace::api::{Configuration, NextStep, Writable};

impl Writable for UserAccount {
    fn initiate(_db: &Pool<Sqlite>) -> Result<NextStep, DwataError> {
        Ok(NextStep::Configure(Configuration::new(
            "User Account",
            "User account details",
            vec![
                FormField::new(
                    "firstName",
                    Some("First Name"),
                    None,
                    ContentType::Text,
                    ContentSpec::default(),
                    Some(true),
                    Some(true),
                ),
                FormField::new(
                    "lastName",
                    Some("Last Name"),
                    None,
                    ContentType::Text,
                    ContentSpec::default(),
                    Some(false),
                    Some(true),
                ),
                FormField::new(
                    "email",
                    Some("Email"),
                    None,
                    ContentType::Text,
                    ContentSpec::default(),
                    Some(false),
                    Some(true),
                ),
            ],
        )))
    }
}
