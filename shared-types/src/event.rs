use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Event {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    /// Date of the event.
    /// SQLite column type: TEXT (raw) / BIGINT UTC ms (parsed)
    pub event_date_raw: Option<String>,
    pub event_date: Option<i64>,
    pub location_id: Option<i64>,
    /// Person IDs of attendees (FK to persons table).
    pub attendees: Vec<i64>,
    pub project_id: Option<i64>,
    pub task_id: Option<i64>,
    /// FK to the email this event was extracted from.
    pub source_email_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct EventsResponse {
    pub events: Vec<Event>,
}
