// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unused)]
// use email_account::app_state::EmailAccountsState;
use env_logger;
use log::info;
use tauri::{App, Manager};
use workspace::{config::DwataConfig, db::DwataDB};

// mod chat;
// mod database_source;
// mod directory_source;
mod error;
// mod labels;
// mod relational_database;
// mod saved_query;
// mod ai_integration;
mod content;
// mod embedding;
// mod text_generation;
// mod schema;
// mod user_account;
// mod workflow;
// mod contacts;
mod email;
mod email_account;
mod oauth2;
mod workspace;

fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    info!("Setting up Dwata");
    #[cfg(debug_assertions)] // only include this code on debug builds
    {
        let window = app.get_webview_window("main").unwrap();
        window.open_devtools();
        window.close_devtools();
    }
    let app_data_dir = app.path().app_data_dir().unwrap();
    info!(
        "Storage directory for dwata: {}",
        app_data_dir.to_str().unwrap()
    );
    app.manage(DwataDB::new(&app_data_dir));
    app.manage(DwataConfig::new(&app_data_dir));
    // app.manage(EmailAccountsState::new(Mutex::new(vec![])));
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(setup)
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            workspace::commands::module_insert_or_update_initiate,
            workspace::commands::module_insert_or_update_on_change,
            workspace::commands::module_insert_or_update_next_step,
            workspace::commands::read_row_list_for_module,
            // workspace::commands::read_row_list_for_module_with_filter,
            workspace::commands::read_module_item_by_pk,
            workspace::commands::insert_module_item,
            // workspace::commands::update_module_item,
            // workspace::commands::upsert_module_item,
            workspace::process_log::get_process_log,
            // directory_source::commands::fetch_file_list_in_directory,
            // directory_source::commands::fetch_file_content_list,
            // ai_integration::commands::get_ai_model_list,
            // ai_integration::commands::get_ai_model_choice_list,
            // text_generation::commands::chat_with_ai,
            // email_account::commands::fetch_emails,
            // email::commands::search_emails,
            // oauth2::commands::get_oauth2_app_choice_list,
            // schema::commands::read_schema,
            // relational_database::commands::load_data,
            // chat::commands::start_chat_thread,
            // chat::commands::fetch_chat_thread_list,
            // chat::commands::fetch_chat_thread_detail,
            // chat::commands::fetch_chat_reply_list,
            // chat::commands::fetch_chat_context_node_list,
            // chat::commands::fetch_chat_context,
            // embedding::commands::generate_text_embedding,
        ])
        .run(tauri::generate_context!())
        .expect("Error while running Dwata");
}
