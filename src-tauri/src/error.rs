use log::error;
use serde::{Deserialize, Serialize};
use sqlx::migrate::MigrateError;
use std::error::Error;

#[derive(Debug, Deserialize, Serialize)]
pub enum DwataError {
    // External Database sources
    DatabaseNotFound,
    CouldNotConnectToDatabase,
    CouldNotQueryDatabase,
    CouldNotCreateDatabase,
    DatabaseTypeNotSupported,

    // Internal SQLite DB for Dwata
    CouldNotCreateDwataDB,
    CouldNotConnectToDwataDB,
    CouldNotInsertToDwataDB,
    CouldNotUpdateDwataDB,
    CouldNotFetchRowsFromDwataDB,
    CouldNotMigrateDwataDB,

    // Blanket error for sqlx
    SqlxError,

    // Workspace and configuration
    ModuleNotFound,

    // AI providers/models/features
    InvalidAIProvider,
    CouldNotConnectToAIProvider,
    CouldNotGenerateEmbedding,
    FeatureNotAvailableWithAIProvider,

    // Integrated vector DB
    CouldNotConnectToVectorDB,

    // Chat context related
    CouldNotFindNode,

    // Directory related
    CouldNotOpenDirectory,
}

impl From<sqlx::Error> for DwataError {
    fn from(err: sqlx::Error) -> Self {
        error!("Got an sqlx error\n Error: {}", err);
        DwataError::SqlxError
    }
}

impl From<MigrateError> for DwataError {
    fn from(err: MigrateError) -> Self {
        error!("Could not migrate Dwata DB\n Error: {}", err);
        DwataError::CouldNotMigrateDwataDB
    }
}

impl Error for DwataError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl std::fmt::Display for DwataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
