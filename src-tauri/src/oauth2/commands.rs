use super::{OAuth2App, OAuth2Token};
use crate::error::DwataError;
use crate::workspace::crud::CRUDRead;
use sqlx::{Pool, Sqlite};
use std::ops::Deref;
use tauri::State;

#[tauri::command]
pub async fn get_oauth2_app_choice_list(
    db: State<'_, Pool<Sqlite>>,
) -> Result<Vec<(String, String)>, DwataError> {
    let db = db.deref();
    let all_oauth2_apps = OAuth2App::read_all(db).await?;
    Ok(all_oauth2_apps
        .iter()
        .map(|x| (format!("{}", x.id), format!("{}", x.provider.to_string())))
        .collect())
}

#[tauri::command]
pub async fn refetch_google_access_token(
    pk: i64,
    db: State<'_, Pool<Sqlite>>,
) -> Result<(), DwataError> {
    let db = db.deref();
    let mut oauth2_token: OAuth2Token = OAuth2Token::read_one_by_pk(pk, db).await?;
    oauth2_token.refetch_google_access_token(db).await?;
    Ok(())
}
