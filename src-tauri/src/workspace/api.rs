use super::ModuleDataCreateUpdate;
use crate::content::form::{FormButton, FormButtonType, FormField};
use crate::error::DwataError;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use ts_rs::TS;

#[derive(Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct Configuration {
    pub title: String,
    pub description: String,
    pub fields: Vec<FormField>,
    pub buttons: Vec<FormButton>,
    // Needed in forms with steps that need external browser or other action
    // while the form can be submitted (backend waits)
    pub submit_implicitly: bool,
}

impl Configuration {
    pub fn new(title: &str, description: &str, fields: Vec<FormField>) -> Self {
        Configuration {
            title: title.to_string(),
            description: description.to_string(),
            fields,
            buttons: vec![FormButton {
                button_type: Some(FormButtonType::Submit),
                label: "Save".to_string(),
            }],
            submit_implicitly: false,
        }
    }
}

#[derive(Deserialize, Serialize, TS)]
#[serde(tag = "type", content = "data")]
#[ts(export)]
pub enum NextStep {
    // This is when we need to update the configuration for form/API
    Configure(Configuration),
    // This is when there is nothing to say given the current data/context
    Continue,
}

pub trait Writable {
    fn initiate(_db: &Pool<Sqlite>) -> Result<NextStep, DwataError>;

    async fn on_change(
        _data: ModuleDataCreateUpdate,
        _db: &Pool<Sqlite>,
    ) -> Result<NextStep, DwataError> {
        return Ok(NextStep::Continue);
    }

    async fn next_step(
        _data: ModuleDataCreateUpdate,
        _db: &Pool<Sqlite>,
    ) -> Result<NextStep, DwataError> {
        return Ok(NextStep::Continue);
    }

    // async fn validate(_data: ModuleDataCreateUpdate) -> Result<Vec<(String, String)>, DwataError> {
    //     return Ok(vec![]);
    // }

    // async fn before_save() {}
}
