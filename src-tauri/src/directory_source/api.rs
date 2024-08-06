use super::DirectorySource;
use crate::content::content::{ContentSpec, ContentType};
use crate::content::form::FormField;
use crate::error::DwataError;
use crate::workspace::api::{Configuration, NextStep, Writable};
use sqlx::{Pool, Sqlite};

impl Writable for DirectorySource {
    fn initiate(_db: &Pool<Sqlite>) -> Result<NextStep, DwataError> {
        Ok(NextStep::Configure(Configuration::new(
            "Directory",
            "Directory containing files which match specified types",
            vec![
                FormField::new(
                    "path",
                    Some("Path to Directory"),
                    Some(
                        "The directory from which you want to read files matching specified types",
                    ),
                    ContentType::Text,
                    ContentSpec::default(),
                    Some(true),
                    Some(true),
                ),
                FormField::new(
                    "label",
                    Some("Label"),
                    Some("An easy to remember label for this directory"),
                    ContentType::Text,
                    ContentSpec::default(),
                    Some(false),
                    Some(true),
                ),
                FormField::new(
                    "includePatterns",
                    Some("Include patterns"),
                    Some("File glob patterns to include (like in .gitignore file)"),
                    ContentType::TextArray,
                    ContentSpec::default(),
                    Some(true),
                    Some(true),
                ),
            ],
        )))
    }
}
