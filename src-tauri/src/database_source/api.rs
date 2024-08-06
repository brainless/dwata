use crate::content::content::{ContentSpec, ContentType};
use crate::content::form::FormField;
use crate::database_source::DatabaseSource;
use crate::error::DwataError;
use crate::workspace::api::{Configuration, NextStep, Writable};
use sqlx::{Pool, Sqlite};

impl Writable for DatabaseSource {
    fn initiate(_db: &Pool<Sqlite>) -> Result<NextStep, DwataError> {
        let database_type_content_spec: ContentSpec = ContentSpec {
            choices: Some(vec![
                ("PostgreSQL".to_string(), "PostgreSQL".to_string()),
                ("MySQL".to_string(), "MySQL".to_string()),
                ("SQLite".to_string(), "SQLite".to_string()),
            ]),
            ..ContentSpec::default()
        };

        Ok(NextStep::Configure(Configuration::new(
            "Database Source",
            "Database source details",
            vec![
                FormField::new(
                    "label",
                    Some("Label"),
                    None,
                    ContentType::Text,
                    ContentSpec::default(),
                    Some(true),
                    Some(true),
                ),
                FormField::new(
                    "databaseType",
                    Some("Database type"),
                    None,
                    ContentType::SingleChoice,
                    database_type_content_spec,
                    Some(true),
                    Some(true),
                ),
                FormField::new(
                    "databaseName",
                    Some("Database name"),
                    None,
                    ContentType::Text,
                    ContentSpec::default(),
                    Some(true),
                    Some(true),
                ),
                FormField::new(
                    "databaseHost",
                    Some("Database host"),
                    None,
                    ContentType::Text,
                    ContentSpec::default(),
                    Some(true),
                    Some(true),
                ),
                FormField::new(
                    "databasePort",
                    Some("Database port"),
                    None,
                    ContentType::Text,
                    ContentSpec::default(),
                    Some(false),
                    Some(true),
                ),
                FormField::new(
                    "databaseUsername",
                    Some("Database username"),
                    None,
                    ContentType::Text,
                    ContentSpec::default(),
                    Some(false),
                    Some(true),
                ),
                FormField::new(
                    "databasePassword",
                    Some("Database password"),
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
