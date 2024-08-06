use super::{
    crud::{CRUDCreateUpdate, CRUDRead, InputValue, VecColumnNameValue},
    ProcessLog, ProcessLogCreateUpdate,
};
use crate::error::DwataError;
use chrono::Utc;
use log::error;
use sqlx::{query, query_as, Pool, Sqlite};
use std::{ops::Deref, time::Duration};
use tauri::{AppHandle, Emitter, State};
use tokio::time::interval;

impl CRUDRead for ProcessLog {
    fn table_name() -> String {
        "process_log".to_string()
    }
}

impl CRUDCreateUpdate for ProcessLogCreateUpdate {
    fn table_name() -> String {
        "process_log".to_string()
    }

    fn get_column_names_values(&self) -> Result<VecColumnNameValue, DwataError> {
        let mut names_values: VecColumnNameValue = VecColumnNameValue::default();
        if let Some(x) = &self.process {
            names_values.push_name_value("task", InputValue::Text(x.to_string()));
        }
        if let Some(x) = &self.arguments {
            names_values.push_name_value("arguments", InputValue::Json(serde_json::json!(x)));
        }
        if let Some(x) = &self.status {
            names_values.push_name_value("status", InputValue::Text(x.to_string()));
        }
        if let Some(x) = &self.is_sent_to_frontend {
            names_values.push_name_value("is_sent_to_frontend", InputValue::Bool(*x));
        }
        names_values.push_name_value("created_at", InputValue::DateTime(Utc::now()));
        Ok(names_values)
    }
}

#[tauri::command]
pub async fn get_process_log(
    db: State<'_, Pool<Sqlite>>,
    app: AppHandle,
) -> Result<(), DwataError> {
    let mut interval = interval(Duration::from_secs(2));
    let db = db.deref();
    loop {
        interval.tick().await;
        // Let's read the process_log table and send any pending updates to the frontend
        let updates = {
            let sql = "SELECT * FROM process_log WHERE is_sent_to_frontend = 0";
            query_as::<_, ProcessLog>(sql).fetch_all(db).await
        };
        match updates {
            Ok(updates) => {
                for update in updates.iter() {
                    // We send the update to the frontend
                    app.emit("process_log", serde_json::to_string(&update).unwrap())
                        .unwrap();
                    // Update the is_sent_to_frontend column
                    let sql = "UPDATE process_log SET is_sent_to_frontend = 1 WHERE id = ?";
                    query(sql).bind(update.id).execute(db).await.unwrap();
                }
            }
            Err(err) => {
                error!("Could not read process log\n Error: {}", err);
            }
        }
    }
}
