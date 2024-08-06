use super::helpers::generate_text_for_chat;
use std::ops::Deref;
// use crate::ai::AITools;
// use crate::chat::api_types::{APIChatContextNode, APIChatReply, APIChatThread};
use crate::error::DwataError;
use crate::workspace::crud::InsertUpdateResponse;
use sqlx::{Pool, Sqlite};
use tauri::State;

/// Tauri command to generate response for a chat thread.
///
/// # Arguments
///
/// * `root_chat_id` - The ID of the root chat of this thread to generate text for.
/// * `db` - The Dwata database connection.
///
/// # Returns
///
/// * `Result<InsertUpdateResponse, DwataError>` - The response from the AI model.
#[tauri::command]
pub async fn chat_with_ai(
    chat_id: i64,
    db: State<'_, Pool<Sqlite>>,
) -> Result<InsertUpdateResponse, DwataError> {
    generate_text_for_chat(chat_id, db.deref()).await
}

// #[tauri::command]
// pub(crate) async fn fetch_chat_context_node_list(
//     node_path: Vec<String>,
//     store: State<'_, Store>,
// ) -> Result<Vec<APIChatContextNode>, DwataError> {
//     let config_guard = store.config.lock().await;
//     Ok(config_guard.get_next_chat_context_node_list(&node_path[..]))
// }

// #[tauri::command]
// pub(crate) async fn fetch_chat_context(
//     node_path: Vec<String>,
//     store: State<'_, Store>,
// ) -> Result<String, DwataError> {
//     let config_guard = store.config.lock().await;
//     config_guard.get_chat_context(&node_path[..]).await
// }
