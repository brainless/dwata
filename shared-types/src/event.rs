use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Event entity for calendar events
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Event {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub project_id: Option<i32>,
    pub task_id: Option<i32>,
    pub date: String,
    pub attendee_user_ids: Option<Vec<i32>>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Request to create a new event
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateEventRequest {
    pub name: String,
    pub description: Option<String>,
    pub project_id: Option<i32>,
    pub task_id: Option<i32>,
    pub date: String,
    pub attendee_user_ids: Option<Vec<i32>>,
}

/// Request to update an event
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UpdateEventRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub project_id: Option<i32>,
    pub task_id: Option<i32>,
    pub date: Option<String>,
    pub attendee_user_ids: Option<Vec<i32>>,
}

/// Response containing a list of events
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EventsResponse {
    pub events: Vec<Event>,
}
