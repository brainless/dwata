use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
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
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Task entity for managing individual tasks
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Task {
    pub id: i32,
    pub project_id: Option<i32>,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub due_date: Option<String>,
    pub assigned_to: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Request to create a new task
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateTaskRequest {
    pub project_id: Option<i32>,
    pub title: String,
    pub description: Option<String>,
    pub priority: TaskPriority,
    pub due_date: Option<String>,
}

/// Request to update a task
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UpdateTaskRequest {
    pub project_id: Option<i32>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub due_date: Option<String>,
}

/// Response containing a list of tasks
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TasksResponse {
    pub tasks: Vec<Task>,
}
