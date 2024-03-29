use crate::ai::AiIntegration;
use crate::data_sources::helpers::check_database_connection;
use crate::data_sources::{DataSource, Database};
use crate::error::DwataError;
use crate::store::Store;
use crate::workspace::api_types::APIConfig;
use std::fs;
use tauri::State;

#[tauri::command]
pub async fn create_data_source(
    _database_type: Option<&str>,
    username: &str,
    password: Option<&str>,
    host: &str,
    port: Option<&str>,
    database: &str,
    store: State<'_, Store>,
) -> Result<String, DwataError> {
    match check_database_connection(username, password, host, port, database).await {
        Ok(_) => {
            let database: Database = Database::new(username, password, host, port, database);
            let data_source: DataSource = DataSource::new_database(database, None);
            let id = data_source.get_id().clone();
            let mut config_guard = store.config.lock().await;
            config_guard.data_source_list.push(data_source);
            match fs::write(
                &config_guard.path_to_config,
                config_guard.get_pretty_string(),
            ) {
                Ok(_) => Ok(id),
                Err(_) => Err(DwataError::CouldNotWriteConfig),
            }
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub async fn create_ai_integration(
    ai_provider: &str,
    api_key: &str,
    display_label: Option<&str>,
    store: State<'_, Store>,
) -> Result<String, DwataError> {
    let ai_integration = AiIntegration::new(ai_provider, api_key, display_label);
    let id = ai_integration.get_id().clone();
    let mut config_guard = store.config.lock().await;
    config_guard.ai_integration_list.push(ai_integration);
    match fs::write(
        &config_guard.path_to_config,
        config_guard.get_pretty_string(),
    ) {
        Ok(_) => Ok(id),
        Err(error) => {
            println!("{:?}", error);
            Err(DwataError::CouldNotWriteConfig)
        }
    }
}

#[tauri::command]
pub async fn update_ai_integration(
    id: &str,
    ai_provider: &str,
    api_key: &str,
    display_label: Option<&str>,
    store: State<'_, Store>,
) -> Result<String, DwataError> {
    let mut config_guard = store.config.lock().await;
    match config_guard
        .ai_integration_list
        .iter_mut()
        .find(|x| x.get_id() == id)
    {
        Some(ai_integration) => {
            ai_integration.update(ai_provider, api_key, display_label);
        }
        None => {}
    }
    match fs::write(
        &config_guard.path_to_config,
        config_guard.get_pretty_string(),
    ) {
        Ok(_) => Ok(id.to_string()),
        Err(error) => {
            println!("{:?}", error);
            Err(DwataError::CouldNotWriteConfig)
        }
    }
}

#[tauri::command]
pub async fn read_config(store: State<'_, Store>) -> Result<APIConfig, DwataError> {
    let guard = store.config.lock().await;
    Ok(APIConfig::from_config(&guard))
}
