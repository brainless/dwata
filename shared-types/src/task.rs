use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Review,
    Done,
    Cancelled,
}

/// Task priority
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Task entity for managing individual tasks
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Task {
    pub id: i64,
    pub project_id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub due_date: Option<String>,
    pub assigned_to: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
