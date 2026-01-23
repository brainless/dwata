use std::path::PathBuf;

/// Returns the path to the dwata database based on the operating system
///
/// # Returns
///
/// A PathBuf pointing to the database file
///
/// # Platform-specific paths
///
/// - **macOS**: `~/Library/Application Support/dwata/db.duckdb`
/// - **Linux**: `~/.local/share/dwata/db.duckdb`
/// - **Windows**: `%LOCALAPPDATA%\dwata\db.duckdb`
pub fn get_db_path() -> anyhow::Result<PathBuf> {
    let home =
        home::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

    let db_path = if cfg!(target_os = "macos") {
        home.join("Library/Application Support/dwata/db.duckdb")
    } else if cfg!(target_os = "linux") {
        home.join(".local/share/dwata/db.duckdb")
    } else if cfg!(windows) {
        home.join("AppData/Local/dwata/db.duckdb")
    } else {
        anyhow::bail!("Unsupported operating system");
    };

    Ok(db_path)
}

/// Initialize the database connection
pub fn initialize_database() -> anyhow::Result<std::sync::Arc<crate::database::Database>> {
    let db_path = get_db_path()?;

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // If a previous DB file exists, remove it to avoid lock conflicts
    if db_path.exists() {
        std::fs::remove_file(&db_path).ok();
    }
    let db = crate::database::Database::new(&db_path)?;
    Ok(std::sync::Arc::new(db))
}
