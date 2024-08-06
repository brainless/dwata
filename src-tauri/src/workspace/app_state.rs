use crate::error::DwataError;
use log::{error, info};
use sqlx::migrate::{MigrateDatabase, Migrator};
use sqlx::{Pool, Sqlite};
use std::fs::create_dir_all;
use std::path::PathBuf;

pub async fn get_database_connection(
    app_data_dir: &PathBuf,
    migrations_dir: PathBuf,
) -> Result<Pool<Sqlite>, DwataError> {
    let mut path = app_data_dir.clone();
    path.push("databases");
    path.push("sqlite3");
    if !path.as_path().exists() {
        create_dir_all(path.as_path()).unwrap_or_else(|_| {});
    }
    path.push("dwata.sqlite3");
    let db_path = path.to_str().unwrap();
    info!("Path to Dwata DB: {}", db_path);
    if !Sqlite::database_exists(db_path).await? {
        info!("Could not find existing Dwata DB, creating");
        Sqlite::create_database(db_path).await?;
    }
    match Pool::<Sqlite>::connect(db_path).await {
        Ok(connection) => match Migrator::new(migrations_dir).await {
            Ok(migrator) => {
                migrator.run(&connection).await?;
                Ok(connection)
            }
            Err(err) => {
                error!("Could not connect to Dwata DB\n Error: {}", err);
                Err(DwataError::CouldNotConnectToDwataDB)
            }
        },
        Err(err) => {
            error!("Could not connect to Dwata DB\n Error: {}", err);
            Err(DwataError::CouldNotConnectToDwataDB)
        }
    }
}
