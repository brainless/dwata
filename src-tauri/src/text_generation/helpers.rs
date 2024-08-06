use super::{TextGenerationRequest, TextGenerationResponse};
use crate::ai_integration::models::AIModel;
use crate::ai_integration::AIIntegration;
use crate::chat::{Chat, ChatCreateUpdate, ChatFilters, ProcessStatus, Role};
use crate::error::DwataError;
use crate::workspace::crud::{CRUDCreateUpdate, CRUDRead, InsertUpdateResponse};
use chrono::{Duration, Utc};
use sqlx::{Pool, Sqlite};

/// Generates response for a chat thread.
/// This function sends all previous messages for this thread to the AI model.
/// It is possible that the entire has already been sent to an AI model and you want to try another model.
///
/// # Arguments
///
/// * `root_chat_id` - The ID of the root chat of this thread to generate text for.
/// * `db` - The Dwata database connection.
///
/// # Returns
///
/// * `Result<InsertUpdateResponse, DwataError>` - The response from the AI model.
pub async fn generate_text_for_chat(
    chat_id: i64,
    db: &Pool<Sqlite>,
) -> Result<InsertUpdateResponse, DwataError> {
    let chat: Chat;
    let root_chat: Chat;
    let model: AIModel;
    let ai_integration: AIIntegration;
    let chats: Vec<Chat>;
    let mut messages_to_send: Vec<TextGenerationRequest>;

    // We use a scope so that the DB connection is closed after it
    // Text generation with Ollama takes a lot of time and we don't want to keep the connection open during this
    chat = Chat::read_one_by_pk(chat_id, db).await?;
    if chat.message.is_none() {
        return Err(DwataError::ChatHasNoMessage);
    }
    if chat.requested_ai_model.is_none() {
        return Err(DwataError::NoRequestedAIModel);
    }
    match chat.process_status {
        Some(ProcessStatus::Pending) => {
            // Check current time and see if the chat is more than 2 minutes old
            // If it is, then we allow the user to resend the chat to the AI model
            let current_time = Utc::now();
            if current_time < chat.created_at + Duration::minutes(2) {
                return Err(DwataError::BeingProcessedByAI);
            }
        }
        Some(ProcessStatus::ResponseReceived) | Some(ProcessStatus::ErrorReceived) => {
            return Err(DwataError::AlreadyProcessedByAI)
        }
        _ => {}
    }
    model = AIModel::from_string(chat.requested_ai_model.unwrap()).await?;
    ai_integration = model.get_integration(db).await?;

    // Get the root chat of this thread (the first chat in the thread)
    match chat.root_chat_id {
        None => {
            root_chat = Chat::read_one_by_pk(chat.id, db).await?;
        }
        Some(x) => {
            root_chat = Chat::read_one_by_pk(x, db).await?;
        }
    }

    // Get all the chats in this thread
    chats = Chat::read_with_filter(
        ChatFilters {
            root_chat_id: Some(root_chat.id),
            ..Default::default()
        },
        db,
    )
    .await?;

    ChatCreateUpdate {
        process_status: Some(ProcessStatus::Pending.to_string()),
        ..ChatCreateUpdate::default()
    }
    .update_module_data(chat_id, db)
    .await?;

    // Convert chats into a list of messages to send to AI model
    // Convert the root chat
    messages_to_send = vec![TextGenerationRequest {
        role: Role::User,
        content: root_chat.message.unwrap(),
    }];

    // Add all the other chats in this thread
    for chat in chats {
        messages_to_send.push(TextGenerationRequest {
            role: chat.role.unwrap(),
            content: chat.message.unwrap(),
        });
    }

    // Calling the AI model outside of the DB transaction
    let reply_from_ai = ai_integration
        .generate_text_with_ai(model, messages_to_send)
        .await?;

    ChatCreateUpdate {
        process_status: Some(ProcessStatus::ResponseReceived.to_string()),
        ..ChatCreateUpdate::default()
    }
    .update_module_data(chat_id, db)
    .await?;
    match reply_from_ai {
        TextGenerationResponse::Message(message) => {
            let new_chat = ChatCreateUpdate {
                root_chat_id: Some(root_chat.id),
                message: Some(message),
                role: Some(Role::Assistant.to_string()),
                ..ChatCreateUpdate::default()
            };
            new_chat.insert_module_data(db).await
        } // TextGenerationResponse::Tool(tool_response) => {
          //     let new_chat = ChatCreateUpdate {
          //         root_chat_id: Some(root_chat.id),
          //         tool_response: Some(tool_response),
          //         role: Some(Role::Assistant.to_string()),
          //         ..ChatCreateUpdate::default()
          //     };
          //     new_chat.insert_module_data(db).await
          //     return Err(DwataError::ToolUseNotSupported);
          // }
    }
}
