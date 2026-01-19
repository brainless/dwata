use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Project status
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectStatus {
    Active,
    Planning,
    OnHold,
    Completed,
    Archived,
}

/// Project entity for managing work and hobby projects
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Project {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub status: ProjectStatus,
    pub tasks_completed: i32,
    pub tasks_total: i32,
    pub deadline: Option<String>,
    pub notifications: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Request to create a new project
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: String,
    pub deadline: Option<String>,
}

/// Request to update a project
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<ProjectStatus>,
    pub deadline: Option<String>,
}

/// Response containing a list of projects
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectsResponse {
    pub projects: Vec<Project>,
}
