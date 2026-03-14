use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Direction of an email sync operation
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum EmailSyncDirection {
    /// Download emails newer than the last synced UID (forward)
    Recent,
    /// Download emails older than the oldest synced UID (backward)
    Backfill,
}

/// Request to trigger an email sync for a specific credential
#[derive(Debug, Deserialize, TS)]
pub struct TriggerEmailSyncRequest {
    pub credential_id: i64,
    pub direction: EmailSyncDirection,
}

/// Request to pause email sync for a specific credential
#[derive(Debug, Deserialize, TS)]
pub struct PauseEmailSyncRequest {
    pub credential_id: i64,
}

/// Request to resume email sync for a specific credential
#[derive(Debug, Deserialize, TS)]
pub struct ResumeEmailSyncRequest {
    pub credential_id: i64,
}

/// Request to trigger sync for all non-paused credentials
#[derive(Debug, Deserialize, TS)]
pub struct TriggerAllEmailSyncRequest {
    pub direction: EmailSyncDirection,
}

/// Persisted per-credential sync settings (stored in DB)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct EmailSyncSettings {
    pub credential_id: i64,
    pub is_paused: bool,
}
