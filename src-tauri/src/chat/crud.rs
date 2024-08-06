use super::{Chat, ChatCreateUpdate, ChatFilters, ProcessStatus, Role};
use crate::content::content::{Content, ContentType};
use crate::error::DwataError;
use crate::workspace::{
    crud::{
        CRUDCreateUpdate, CRUDRead, CRUDReadFilter, InputValue, InsertUpdateResponse,
        VecColumnNameValue,
    },
    ModuleFilters,
};
use chrono::Utc;
use sqlx::{Pool, Sqlite};
use std::str::FromStr;

impl CRUDRead for Chat {
    fn table_name() -> String {
        "chat".to_string()
    }

    fn set_default_filters() -> Option<ModuleFilters> {
        Some(ModuleFilters::Chat(ChatFilters {
            root_chat_null: Some(true),
            ..Default::default()
        }))
    }
}

impl CRUDCreateUpdate for ChatCreateUpdate {
    fn table_name() -> String {
        "chat".to_string()
    }

    fn get_column_names_values(&self) -> Result<VecColumnNameValue, DwataError> {
        let mut names_values: VecColumnNameValue = VecColumnNameValue::default();
        if let Some(x) = &self.root_chat_id {
            names_values.push_name_value("root_chat_id", InputValue::ID(*x));
        }
        if let Some(x) = &self.compared_to_root_chat_id {
            names_values.push_name_value("compared_to_root_chat_id", InputValue::ID(*x));
        }
        if let Some(x) = &self.message {
            if x.chars().count() == 0 {
                return Err(DwataError::ChatHasNoMessage);
            }
            names_values.push_name_value("message", InputValue::Text(x.clone()));
        }
        if let Some(x) = &self.requested_ai_model {
            names_values.push_name_value("requested_ai_model", InputValue::Text(x.clone()));
        }
        if let Some(x) = &self.process_status {
            match ProcessStatus::from_str(x) {
                Ok(x) => {
                    names_values.push_name_value("process_status", InputValue::Text(x.to_string()))
                }
                Err(_) => {
                    return Err(DwataError::InvalidProcessStatus);
                }
            }
        }
        if let Some(x) = &self.role {
            names_values.push_name_value("role", InputValue::Text(x.clone()));
        } else {
            names_values.push_name_value("role", InputValue::Text(Role::User.to_string()));
        }
        // names_values.push_name_value("is_system_chat", InputValue::Bool(false));
        names_values.push_name_value("created_at", InputValue::DateTime(Utc::now()));
        Ok(names_values)
    }

    async fn post_insert(
        &self,
        response: InsertUpdateResponse,
        _db: &Pool<Sqlite>,
    ) -> Result<InsertUpdateResponse, DwataError> {
        Ok(InsertUpdateResponse {
            next_task: Some("chat_with_ai".to_string()),
            arguments: Some(vec![(
                "chatId".to_string(),
                ContentType::ID,
                Content::ID(response.pk),
            )]),
            ..response
        })
    }
}

impl CRUDReadFilter for ChatFilters {
    fn get_column_names_values_to_filter(&self) -> VecColumnNameValue {
        let mut name_values: VecColumnNameValue = VecColumnNameValue::default();
        if let Some(x) = &self.role {
            name_values.push_name_value("role", InputValue::Text(x.clone()));
        }
        if let Some(x) = &self.root_chat_id {
            name_values.push_name_value("root_chat_id", InputValue::ID(*x));
        }
        if let Some(true) = &self.root_chat_null {
            name_values.push_name_value("root_chat_id", InputValue::Null);
        }
        if let Some(x) = &self.compared_to_root_chat_id {
            name_values.push_name_value("compared_to_root_chat_id", InputValue::ID(*x));
        }
        if let Some(true) = &self.compared_to_root_chat_null {
            name_values.push_name_value("compared_to_root_chat_id", InputValue::Null);
        }
        if let Some(x) = &self.requested_ai_model {
            name_values.push_name_value("requested_ai_model", InputValue::Text(x.clone()));
        }
        name_values
    }
}
