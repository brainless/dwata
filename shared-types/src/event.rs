use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Event {
    pub id: i64,
    pub extraction_job_id: Option<i64>,
    pub email_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub event_date: i64, // Unix timestamp
    pub location: Option<String>,
    #[ts(skip)]
    pub attendees: serde_json::Value, // Array of email addresses
    pub confidence: Option<f32>,
    pub requires_review: bool,
    pub is_confirmed: bool,
    pub project_id: Option<i64>,
    pub task_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateEventRequest {
    pub name: String,
    pub description: Option<String>,
    pub event_date: i64,
    pub location: Option<String>,
    pub attendees: Vec<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateEventRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub event_date: Option<i64>,
    pub location: Option<String>,
    pub attendees: Option<Vec<String>>,
    pub is_confirmed: Option<bool>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct EventsResponse {
    pub events: Vec<Event>,
}
