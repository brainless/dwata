use super::{DatabaseSource, DatabaseSourceCreateUpdate, DatabaseType};
use crate::database_source::helpers::check_database_connection;
use crate::error::DwataError;
use crate::workspace::crud::{CRUDCreateUpdate, CRUDRead, InputValue, VecColumnNameValue};
use chrono::Utc;
use sqlx::{Pool, Sqlite, SqliteConnection};
use std::str::FromStr;

impl CRUDRead for DatabaseSource {
    fn table_name() -> String {
        "database_source".to_string()
    }
}

impl CRUDCreateUpdate for DatabaseSourceCreateUpdate {
    fn table_name() -> String {
        "database_source".to_string()
    }

    fn get_column_names_values(&self) -> Result<VecColumnNameValue, DwataError> {
        let mut names_values: VecColumnNameValue = VecColumnNameValue::default();
        if let Some(x) = &self.label {
            names_values.push_name_value("label", InputValue::Text(x.clone()));
        }
        if let Some(x) = &self.database_type {
            names_values.push_name_value("database_type", InputValue::Text(x.clone()));
        }
        if let Some(x) = &self.database_name {
            names_values.push_name_value("database_name", InputValue::Text(x.clone()));
        }
        if let Some(x) = &self.database_host {
            names_values.push_name_value("database_host", InputValue::Text(x.clone()));
        }
        if let Some(x) = &self.database_port {
            names_values.push_name_value("database_port", InputValue::Text(x.to_string()));
        }
        if let Some(x) = &self.database_username {
            names_values.push_name_value("database_username", InputValue::Text(x.clone()));
        }
        if let Some(x) = &self.database_password {
            names_values.push_name_value("database_password", InputValue::Text(x.clone()));
        }
        names_values.push_name_value("created_at", InputValue::DateTime(Utc::now()));
        Ok(names_values)
    }

    async fn pre_insert(&self, _db: &Pool<Sqlite>) -> Result<VecColumnNameValue, DwataError> {
        // We connect to this database and check if it works
        check_database_connection(
            &DatabaseType::from_str(self.database_type.as_ref().unwrap())?,
            self.database_host.as_ref().unwrap(),
            self.database_port.as_ref(),
            self.database_username.as_ref().unwrap(),
            self.database_password
                .as_ref()
                .and_then(|x| Some(x.as_ref())),
            self.database_name.as_ref().unwrap(),
        )
        .await?;
        Ok(VecColumnNameValue::default())
    }
}
