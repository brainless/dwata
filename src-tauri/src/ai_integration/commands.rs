use log::error;
use tauri::State;
use tokio::task::spawn_blocking;

use crate::{
    error::DwataError,
    workspace::{crud::CRUDRead, DwataDb},
};

use super::{models::AIModel, AIIntegration, AIProvider};

#[tauri::command]
pub async fn get_ai_model_list(_usable_only: Option<bool>) -> Result<Vec<AIModel>, DwataError> {
    Ok(AIModel::get_all_models().await)
}

#[tauri::command]
pub async fn get_ai_model_choice_list(
    usable_only: Option<bool>,
    db: State<'_, DwataDb>,
) -> Result<Vec<(String, String)>, DwataError> {
    let mut result: Vec<AIModel> = vec![];
    if let Some(false) = usable_only {
        // We load all the AI models
        result.extend(AIModel::get_all_models().await);
    } else {
        let mut db_guard = db.lock().await;
        // We load all the AI providers that are usable
        let ai_integrations: Vec<AIIntegration> = AIIntegration::read_all(&mut db_guard).await?;
        for ai_integration in ai_integrations {
            match ai_integration.ai_provider {
                AIProvider::OpenAI => result.extend(AIModel::get_models_for_openai()),
                AIProvider::Groq => result.extend(AIModel::get_models_for_groq()),
                AIProvider::Ollama => match AIModel::get_models_for_ollama().await {
                    Ok(models) => result.extend(models),
                    Err(err) => {
                        error!("Could not get Ollama models\n Error: {}", err);
                    }
                },
            }
        }
    }

    Ok(result
        .iter()
        .map(|x| {
            (
                format!(
                    "{}::{}",
                    x.ai_provider.clone().to_string(),
                    x.api_name.clone()
                ),
                format!(
                    "{} - {}",
                    x.ai_provider.clone().to_string(),
                    x.label.clone()
                ),
            )
        })
        .collect())
}
