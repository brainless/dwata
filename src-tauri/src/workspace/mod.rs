use crate::ai_integration::{AIIntegration, AIIntegrationCreateUpdate, AIIntegrationFilters};
use crate::chat::{Chat, ChatCreateUpdate, ChatFilters};
use crate::database_source::{DatabaseSource, DatabaseSourceCreateUpdate};
use crate::directory_source::{DirectorySource, DirectorySourceCreateUpdate};
use crate::email::{Email, EmailFilters};
use crate::email_account::{EmailAccount, EmailAccountCreateUpdate, Mailbox};
use crate::oauth2::{OAuth2App, OAuth2AppCreateUpdate};
use crate::user_account::{UserAccount, UserAccountCreateUpdate};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{types::Json, FromRow, Type};
use strum::{Display, EnumString};
use ts_rs::TS;

pub mod api;
pub mod app_state;
pub mod commands;
pub mod crud;
pub mod process_log;

#[derive(Deserialize, Display, TS)]
#[ts(export)]
pub enum Module {
    UserAccount,
    DirectorySource,
    DatabaseSource,
    AIIntegration,
    Chat,
    OAuth2App,
    EmailAccount,
    Mailbox,
    Email,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub enum ModuleDataRead {
    UserAccount(UserAccount),
    DirectorySource(DirectorySource),
    DatabaseSource(DatabaseSource),
    AIIntegration(AIIntegration),
    Chat(Chat),
    OAuth2App(OAuth2App),
    EmailAccount(EmailAccount),
    Mailbox(Mailbox),
    Email(Email),
}

#[derive(Serialize, TS)]
#[serde(tag = "type", content = "data")]
#[ts(export)]
pub enum ModuleDataReadList {
    UserAccount(Vec<UserAccount>),
    DirectorySource(Vec<DirectorySource>),
    DatabaseSource(Vec<DatabaseSource>),
    AIIntegration(Vec<AIIntegration>),
    Chat(Vec<Chat>),
    OAuth2App(Vec<OAuth2App>),
    EmailAccount(Vec<EmailAccount>),
    Mailbox(Vec<Mailbox>),
    Email(Vec<Email>),
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub enum ModuleDataCreateUpdate {
    UserAccount(UserAccountCreateUpdate),
    DirectorySource(DirectorySourceCreateUpdate),
    DatabaseSource(DatabaseSourceCreateUpdate),
    AIIntegration(AIIntegrationCreateUpdate),
    Chat(ChatCreateUpdate),
    OAuth2App(OAuth2AppCreateUpdate),
    EmailAccount(EmailAccountCreateUpdate),
}

#[derive(Deserialize, Display, TS)]
#[ts(export)]
pub enum ModuleFilters {
    AIIntegration(AIIntegrationFilters),
    Chat(ChatFilters),
    Email(EmailFilters),
}

#[derive(Clone, Serialize, Type, TS, EnumString, Display)]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
#[ts(export)]
pub enum ProcessInLog {
    CheckEmails,
    FetchEmails,
    ParseEmails,
    IndexEmails,
}

#[derive(Clone, Serialize, Type, TS, EnumString, Display)]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
#[ts(export)]
pub enum ProcessingStatusInLog {
    Pending,
    InProgress,
    Completed,
}

#[derive(Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ProcessLog {
    #[ts(type = "number")]
    pub id: i64,
    pub task: ProcessInLog,
    #[ts(type = "Array<[string, string]>")]
    pub arguments: Json<Vec<(String, String)>>,
    pub status: ProcessingStatusInLog,
    pub is_sent_to_frontend: bool,

    pub created_at: DateTime<Utc>,
    pub modified_at: Option<DateTime<Utc>>,
}

pub struct ProcessLogCreateUpdate {
    pub process: Option<ProcessInLog>,
    pub arguments: Option<Vec<(String, String)>>,
    pub status: Option<ProcessingStatusInLog>,
    pub is_sent_to_frontend: Option<bool>,
}

// impl ChatContextNode for Config {
//     fn get_self_chat_context_node(&self) -> APIChatContextNode {
//         todo!()
//     }

//     fn get_next_chat_context_node_list(&self, node_path: &[String]) -> Vec<APIChatContextNode> {
//         if node_path.is_empty() {
//             let list: Vec<APIChatContextNode> = self
//                 .data_source_list
//                 .iter()
//                 .map(|x| x.get_self_chat_context_node())
//                 .collect();
//             // list.push(APIChatContextNode::new(
//             //     "__upload_file__".to_string(),
//             //     APIChatContextType::ContentsFromFile,
//             //     "Contents of a file".to_string(),
//             //     false,
//             // ));
//             list
//         } else {
//             let data_source = self
//                 .data_source_list
//                 .iter()
//                 .find(|x| x.get_id() == node_path[0]);
//             data_source
//                 .unwrap()
//                 .get_next_chat_context_node_list(&node_path[1..])
//         }
//     }

//     async fn get_chat_context(&self, node_path: &[String]) -> Result<String, DwataError> {
//         let data_source = self
//             .data_source_list
//             .iter()
//             .find(|x| x.get_id() == node_path[0]);
//         data_source.unwrap().get_chat_context(&node_path[1..]).await
//     }
// }

// impl AITools for Config {
//     fn get_self_tool_list(&self) -> Vec<Tool> {
//         let data_source_list: Vec<String> = self
//             .data_source_list
//             .iter()
//             .map(|x| x.get_tool_name().clone())
//             .collect();
//         let first_source = data_source_list.clone().first().unwrap().clone();
//         vec![Tool::new(
//             "get_schema_of_selected_data_source".to_string(),
//             "I have multiple business databases.\
//              This function retrieves the schema of all tables of the selected data source.\
//               You can use the schema to generate SQL."
//                 .to_string(),
//             vec![ToolParameter::new(
//                 "data_source_id".to_string(),
//                 ToolParameterType::Enum(data_source_list),
//                 format!(
//                     "Select a data source from these options, example {}",
//                     first_source
//                 ),
//             )],
//             vec!["data_source_id".to_string()],
//         )]
//     }
// }
