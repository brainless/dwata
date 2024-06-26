use crate::data_sources::DatabaseSource;
use crate::schema::api_types::{
    APIGridColumn, APIGridColumnBuilder, APIGridSchema, APIGridSchemaBuilder, ColumnDataType,
    IsForeignKey, TypeArrayBuilder, TypeIntegerBuilder,
};
use serde::{Deserialize, Serialize};
use sqlx::postgres::types::Oid;

pub mod metadata;

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
pub struct PostgreSQLObject {
    oid: Oid,
    schema: String,
    name: String,
    object_type: String,
    owner: String,
    comment: Option<String>,
}

impl PostgreSQLObject {
    pub fn filter_is_table(&self) -> bool {
        self.object_type == "table".to_string()
    }

    pub async fn get_table(
        &self,
        data_source: &DatabaseSource,
        with_columns: Option<bool>,
    ) -> APIGridSchema {
        let table_builder = APIGridSchemaBuilder::default()
            .source(data_source.get_id())
            .name(Some(self.name.clone()))
            .schema(Some(self.schema.clone()));
        let mut table = table_builder.build().unwrap();
        if Some(true) == with_columns {
            table.get_columns(data_source).await;
        }
        table
    }
}

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
pub struct PostgreSQLColumn {
    column_name: String,
    data_type: String,
    is_nullable: String,
    character_maximum_length: Option<i32>,
    character_set_catalog: Option<String>,
    column_default: Option<String>,
    comment: Option<String>,
}

impl PostgreSQLColumn {
    pub fn get_generic_column(&self) -> APIGridColumn {
        APIGridColumnBuilder::default()
            .name(self.column_name.clone())
            .label(None)
            .data_type(match self.data_type.as_str() {
                "integer" => {
                    ColumnDataType::SignedInteger(TypeIntegerBuilder::default().build().unwrap())
                }
                "character varying" => ColumnDataType::CharArray(
                    TypeArrayBuilder::default()
                        .length(self.character_maximum_length)
                        .build()
                        .unwrap(),
                ),
                "boolean" => ColumnDataType::Boolean,
                "text" => ColumnDataType::Text,
                _ => ColumnDataType::Unknown,
            })
            .is_nullable(match self.is_nullable.as_str() {
                "NO" => false,
                "YES" => true,
                _ => false,
            })
            .is_auto_increment(false)
            .is_primary_key(false)
            .is_indexed(false)
            .is_foreign_key(IsForeignKey::No)
            .build()
            .unwrap()
    }

    pub fn is_column_named(&self, name: String) -> bool {
        self.column_name == name
    }

    pub fn get_data_type(&self) -> String {
        self.data_type.clone()
    }
}
