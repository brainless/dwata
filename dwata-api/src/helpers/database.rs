use std::path::PathBuf;

/// Returns the path to the dwata database based on the operating system
///
/// # Returns
///
/// A PathBuf pointing to the database file
///
/// # Platform-specific paths
///
/// - **macOS**: `~/Library/Application Support/dwata/db.sqlite`
/// - **Linux**: `~/.local/share/dwata/db.sqlite`
/// - **Windows**: `%LOCALAPPDATA%\dwata\db.sqlite`
pub fn get_db_path() -> anyhow::Result<PathBuf> {
    let data_dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine local data directory"))?;

    let db_path = data_dir.join("dwata").join("db.sqlite");

    Ok(db_path)
}

/// Initialize the database connection
pub fn initialize_database() -> anyhow::Result<std::sync::Arc<crate::database::Database>> {
    let db_path = get_db_path()?;

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = crate::database::Database::new(&db_path)?;
    Ok(std::sync::Arc::new(db))
}
