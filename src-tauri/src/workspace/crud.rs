use super::ModuleFilters;
use crate::content::content::{Content, ContentType};
use crate::error::DwataError;
use chrono::{DateTime, Utc};
use log::{error, info};
use serde::Serialize;
use serde_json::Value;
use sqlx::sqlite::SqliteRow;
use sqlx::{query_as, Execute, FromRow, Pool, QueryBuilder, Sqlite};
use ts_rs::TS;

pub trait CRUDRead {
    fn table_name() -> String;

    fn set_default_filters() -> Option<ModuleFilters> {
        None
    }

    async fn read_all_helper(db: &Pool<Sqlite>) -> Result<Vec<Self>, DwataError>
    where
        Self: Sized + Send + Unpin,
        for<'r> Self: FromRow<'r, SqliteRow>,
    {
        match query_as(&format!("SELECT * FROM {}", Self::table_name()))
            .fetch_all(db)
            .await
        {
            Ok(result) => {
                info!(
                    "Fetched {} rows from Dwata DB, table {}",
                    result.len(),
                    Self::table_name()
                );
                Ok(result)
            }
            Err(err) => {
                error!(
                    "Could not fetch rows from Dwata DB, table {}.\nError: {}",
                    Self::table_name(),
                    err
                );
                Err(DwataError::CouldNotFetchRowsFromDwataDB)
            }
        }
    }

    async fn read_all(db: &Pool<Sqlite>) -> Result<Vec<Self>, DwataError>
    where
        Self: Sized + Send + Unpin,
        for<'r> Self: FromRow<'r, SqliteRow>,
    {
        // We check if there are default filters to apply
        let default_filters = Self::set_default_filters();
        match default_filters {
            Some(filters) => match filters {
                ModuleFilters::AIIntegration(x) => Self::read_with_filter_helper(x, db).await,
                ModuleFilters::Chat(x) => Self::read_with_filter_helper(x, db).await,
                _ => {
                    error!("Invalid module {}", filters.to_string());
                    Err(DwataError::ModuleNotFound)
                }
            },
            None => Self::read_all_helper(db).await,
        }
    }

    async fn read_one_by_pk(pk: i64, db: &Pool<Sqlite>) -> Result<Self, DwataError>
    where
        Self: Sized + Send + Unpin,
        for<'r> Self: FromRow<'r, SqliteRow>,
    {
        let result: Result<Self, sqlx::Error> = query_as(&format!(
            "SELECT * FROM {} WHERE id = ?",
            Self::table_name()
        ))
        .bind(pk)
        .fetch_one(db)
        .await;
        match result {
            Ok(row) => {
                info!(
                    "Fetched one row from Dwata DB, table {}, ID {}",
                    Self::table_name(),
                    pk
                );
                Ok(row)
            }
            Err(err) => {
                error!(
                    "Could not fetch rows from Dwata DB, table {}.\nError: {}",
                    Self::table_name(),
                    err
                );
                Err(DwataError::CouldNotFetchRowsFromDwataDB)
            }
        }
    }

    async fn read_with_filter_helper<T>(
        filters: T,
        db: &Pool<Sqlite>,
    ) -> Result<Vec<Self>, DwataError>
    where
        Self: Sized + Send + Unpin,
        for<'r> Self: FromRow<'r, SqliteRow>,
        T: CRUDReadFilter,
    {
        let mut builder: QueryBuilder<Sqlite> =
            QueryBuilder::new(format!("SELECT * FROM {} WHERE ", Self::table_name()));
        let column_names_values: VecColumnNameValue = filters.get_column_names_values_to_filter();
        let mut count = 0;
        let limit = column_names_values.0.len();
        for (name, data) in column_names_values.0 {
            match data {
                InputValue::Null => {
                    builder.push(format!("{} IS null", name));
                }
                _ => {
                    builder.push(format!("{} = ", name));
                    match data {
                        InputValue::Text(x) => {
                            builder.push_bind(x.clone());
                        }
                        InputValue::ID(x) => {
                            builder.push_bind(x);
                        }
                        InputValue::Bool(x) => {
                            builder.push_bind(x);
                        }
                        InputValue::Json(x) => {
                            builder.push_bind(x);
                        }
                        InputValue::DateTime(x) => {
                            builder.push_bind(x);
                        }
                        _ => {}
                    }
                }
            }
            count += 1;
            if count < limit {
                builder.push(" AND ");
            }
        }
        let executable = builder.build_query_as();
        let sql = &executable.sql();
        match executable.fetch_all(db).await {
            Ok(rows) => {
                info!(
                    "Fetched {} rows from Dwata DB, table {}",
                    rows.len(),
                    Self::table_name(),
                );
                Ok(rows)
            }
            Err(err) => {
                error!(
                    "Could not fetch rows from Dwata DB, table {}.\nSQL: {}\nError: {}",
                    Self::table_name(),
                    sql,
                    err
                );
                Err(DwataError::CouldNotFetchRowsFromDwataDB)
            }
        }
    }

    async fn read_with_filter<T>(filters: T, db: &Pool<Sqlite>) -> Result<Vec<Self>, DwataError>
    where
        Self: Sized + Send + Unpin,
        for<'r> Self: FromRow<'r, SqliteRow>,
        T: CRUDReadFilter,
    {
        Self::read_with_filter_helper(filters, db).await
    }
}

pub trait CRUDReadFilter {
    fn get_column_names_values_to_filter(&self) -> VecColumnNameValue {
        VecColumnNameValue::default()
    }
}

#[derive(Clone)]
pub enum InputValue {
    Text(String),
    ID(i64),
    Bool(bool),
    Json(Value),
    DateTime(DateTime<Utc>),
    Null,
}

#[derive(Default)]
pub struct VecColumnNameValue(pub Vec<(String, InputValue)>);

impl VecColumnNameValue {
    pub fn push_name_value(&mut self, name: &str, value: InputValue) {
        self.0.push((name.to_string(), value));
    }

    pub fn find_by_name(&self, name: &str) -> Option<InputValue> {
        self.0
            .iter()
            .find_map(|(x, y)| if x == name { Some(y.clone()) } else { None })
    }
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct InsertUpdateResponse {
    pub pk: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_task: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<(String, ContentType, Content)>>,
}

pub trait CRUDCreateUpdate {
    fn table_name() -> String;

    fn get_column_names_values(&self) -> Result<VecColumnNameValue, DwataError>;

    async fn pre_insert(&self, _db: &Pool<Sqlite>) -> Result<VecColumnNameValue, DwataError> {
        Ok(VecColumnNameValue::default())
    }

    async fn insert_module_data(
        &self,
        db: &Pool<Sqlite>,
    ) -> Result<InsertUpdateResponse, DwataError> {
        let column_names_values: VecColumnNameValue = self.get_column_names_values()?;
        let other_column_names_values: VecColumnNameValue = self.pre_insert(db).await?;
        let column_names_values = VecColumnNameValue {
            0: [column_names_values.0, other_column_names_values.0].concat(),
        };
        let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(format!(
            "INSERT INTO {} ({}) VALUES (",
            Self::table_name(),
            column_names_values
                .0
                .iter()
                .map(|x| x.0.clone())
                .collect::<Vec<String>>()
                .join(", ")
        ));
        let mut count = 0;
        let limit = column_names_values.0.len();
        for column in column_names_values.0 {
            match column.1 {
                InputValue::Text(x) => {
                    builder.push_bind(x.clone());
                }
                InputValue::Json(x) => {
                    builder.push_bind(x);
                }
                InputValue::DateTime(x) => {
                    builder.push_bind(x);
                }
                InputValue::ID(x) => {
                    builder.push_bind(x);
                }
                InputValue::Bool(x) => {
                    builder.push_bind(x);
                }
                _ => {}
            }
            count += 1;
            if count < limit {
                builder.push(", ");
            }
        }
        builder.push(")");
        let executable = builder.build();
        let sql = &executable.sql();
        match executable.execute(db).await {
            Ok(result) => {
                info!(
                    "Inserted into Dwata DB, table {}:\nSQL: {}",
                    Self::table_name(),
                    sql
                );
                self.post_insert(
                    InsertUpdateResponse {
                        pk: result.last_insert_rowid(),
                        next_task: None,
                        arguments: None,
                    },
                    db,
                )
                .await
            }
            Err(err) => {
                error!(
                    "Could not insert into Dwata DB, table {}:\nSQL: {}\nError: {}",
                    Self::table_name(),
                    sql,
                    err
                );
                Err(DwataError::CouldNotInsertToDwataDB)
            }
        }
    }

    async fn post_insert(
        &self,
        response: InsertUpdateResponse,
        _db: &Pool<Sqlite>,
    ) -> Result<InsertUpdateResponse, DwataError> {
        Ok(response)
    }

    async fn update_module_data(
        &self,
        pk: i64,
        db: &Pool<Sqlite>,
    ) -> Result<InsertUpdateResponse, DwataError> {
        let mut builder: QueryBuilder<Sqlite> =
            QueryBuilder::new(format!("UPDATE {} SET ", Self::table_name()));
        let column_names_values: VecColumnNameValue = self.get_column_names_values()?;
        let mut count = 0;
        let limit = column_names_values.0.len();
        for (name, data) in column_names_values.0 {
            builder.push(format!("{} = ", name));
            match data {
                InputValue::Text(x) => {
                    builder.push_bind(x.clone());
                }
                InputValue::Json(x) => {
                    builder.push_bind(x);
                }
                InputValue::DateTime(x) => {
                    builder.push_bind(x);
                }
                InputValue::ID(x) => {
                    builder.push_bind(x);
                }
                InputValue::Bool(x) => {
                    builder.push_bind(x);
                }
                _ => {}
            }
            count += 1;
            if count < limit {
                builder.push(", ");
            }
        }
        builder.push(" WHERE id = ");
        builder.push_bind(pk);
        let executable = builder.build();
        let sql = &executable.sql();
        match executable.execute(db).await {
            Ok(_) => {
                info!(
                    "Updated Dwata DB, table {}:\nSQL: {}",
                    Self::table_name(),
                    sql
                );
                Ok(InsertUpdateResponse {
                    pk,
                    next_task: None,
                    arguments: None,
                })
            }
            Err(err) => {
                error!(
                    "Could not update Dwata DB, table {}:\nSQL: {}\nError: {}",
                    Self::table_name(),
                    sql,
                    err
                );
                Err(DwataError::CouldNotUpdateDwataDB)
            }
        }
    }
}
