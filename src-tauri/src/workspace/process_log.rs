use super::{
    crud::{CRUDCreateUpdate, CRUDRead, InputValue, VecColumnNameValue},
    db::DwataDB,
    ProcessLog, ProcessLogCreateUpdate,
};
use crate::error::DwataError;
use chrono::Utc;
use log::error;
use rocksdb::IteratorMode;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tokio::time::interval;

impl CRUDRead for ProcessLog {
    fn table_name() -> String {
        "process_log".to_string()
    }
}

// impl CRUDCreateUpdate for ProcessLog {
//     type Payload = ProcessLogCreateUpdate;

//     fn table_name() -> String {
//         "process_log".to_string()
//     }

//     fn get_parsed_item(&self) -> Result<VecColumnNameValue, DwataError> {
//         let mut names_values: VecColumnNameValue = VecColumnNameValue::default();
//         if let Some(x) = &self.process {
//             names_values.push_name_value("task", InputValue::Text(x.to_string()));
//         }
//         if let Some(x) = &self.arguments {
//             names_values.push_name_value("arguments", InputValue::Json(serde_json::json!(x)));
//         }
//         if let Some(x) = &self.status {
//             names_values.push_name_value("status", InputValue::Text(x.to_string()));
//         }
//         if let Some(x) = &self.is_sent_to_frontend {
//             names_values.push_name_value("is_sent_to_frontend", InputValue::Bool(*x));
//         }
//         names_values.push_name_value("created_at", InputValue::DateTime(Utc::now()));
//         Ok(names_values)
//     }
// }

#[tauri::command]
pub async fn get_process_log(app: AppHandle, db: State<'_, DwataDB>) -> Result<(), DwataError> {
    let mut interval = interval(Duration::from_secs(2));
    loop {
        interval.tick().await;
        // Let's read the process_log table and send any pending updates to the frontend
        let updates = db.get_db("process_log", None)?;
        for row in updates.iterator(IteratorMode::Start) {
            match row {
                Ok((key, value)) => {
                    // We send the update to the frontend
                    app.emit("process_log", &value).unwrap();
                    // Update the is_sent_to_frontend column
                    let mut update = serde_json::from_slice::<ProcessLog>(&value).unwrap();
                    update.is_sent_to_frontend = true;
                    updates.put(key, serde_json::to_string(&update).unwrap().as_bytes());
                }
                Err(_) => {}
            }
        }
    }
}
