use super::{
    api::{NextStep, Writable},
    crud::{CRUDCreateUpdate, InsertUpdateResponse},
    {Module, ModuleDataCreateUpdate, ModuleDataRead, ModuleDataReadList, ModuleFilters},
};
use crate::ai_integration::AIIntegration;
use crate::database_source::DatabaseSource;
use crate::directory_source::DirectorySource;
use crate::email_account::EmailAccount;
use crate::error::DwataError;
use crate::oauth2::OAuth2App;
use crate::user_account::UserAccount;
use crate::workspace::crud::CRUDRead;
use crate::{chat::Chat, email_account::Mailbox};
use log::error;
use sqlx::{Pool, Sqlite};
use std::ops::Deref;
use tauri::State;

#[tauri::command]
pub fn module_insert_or_update_initiate(
    module: Module,
    db: State<'_, Pool<Sqlite>>,
) -> Result<NextStep, DwataError> {
    let db = db.deref();
    match module {
        Module::UserAccount => UserAccount::initiate(db),
        Module::DirectorySource => DirectorySource::initiate(db),
        Module::DatabaseSource => DatabaseSource::initiate(db),
        Module::AIIntegration => AIIntegration::initiate(db),
        Module::Chat => Chat::initiate(db),
        Module::OAuth2App => OAuth2App::initiate(db),
        Module::EmailAccount => EmailAccount::initiate(db),
        Module::Mailbox => Err(DwataError::ModuleNotWritable),
        Module::Email => Err(DwataError::ModuleNotWritable),
    }
}

#[tauri::command]
pub async fn module_insert_or_update_on_change(
    module: Module,
    data: ModuleDataCreateUpdate,
    db: State<'_, Pool<Sqlite>>,
) -> Result<NextStep, DwataError> {
    let db = db.deref();
    match module {
        Module::UserAccount => UserAccount::on_change(data, db).await,
        Module::DirectorySource => DirectorySource::on_change(data, db).await,
        Module::DatabaseSource => DatabaseSource::on_change(data, db).await,
        Module::AIIntegration => AIIntegration::on_change(data, db).await,
        Module::Chat => Chat::on_change(data, db).await,
        Module::OAuth2App => OAuth2App::on_change(data, db).await,
        Module::EmailAccount => EmailAccount::on_change(data, db).await,
        Module::Mailbox => Err(DwataError::ModuleNotWritable),
        Module::Email => Err(DwataError::ModuleNotWritable),
    }
}

#[tauri::command]
pub async fn module_insert_or_update_next_step(
    module: Module,
    data: ModuleDataCreateUpdate,
    db: State<'_, Pool<Sqlite>>,
) -> Result<NextStep, DwataError> {
    let db = db.deref();
    match module {
        Module::UserAccount => UserAccount::next_step(data, db).await,
        Module::DirectorySource => DirectorySource::next_step(data, db).await,
        Module::DatabaseSource => DatabaseSource::next_step(data, db).await,
        Module::AIIntegration => AIIntegration::next_step(data, db).await,
        Module::Chat => Chat::next_step(data, db).await,
        Module::OAuth2App => OAuth2App::next_step(data, db).await,
        Module::EmailAccount => EmailAccount::next_step(data, db).await,
        Module::Mailbox => Err(DwataError::ModuleNotWritable),
        Module::Email => Err(DwataError::ModuleNotWritable),
    }
}

#[tauri::command]
pub async fn read_row_list_for_module(
    module: Module,
    db: State<'_, Pool<Sqlite>>,
) -> Result<ModuleDataReadList, DwataError> {
    let db = db.deref();
    match module {
        Module::UserAccount => UserAccount::read_all(db)
            .await
            .and_then(|result| Ok(ModuleDataReadList::UserAccount(result))),
        Module::DirectorySource => DirectorySource::read_all(db)
            .await
            .and_then(|result| Ok(ModuleDataReadList::DirectorySource(result))),
        Module::DatabaseSource => DatabaseSource::read_all(db)
            .await
            .and_then(|result| Ok(ModuleDataReadList::DatabaseSource(result))),
        Module::AIIntegration => AIIntegration::read_all(db)
            .await
            .and_then(|result| Ok(ModuleDataReadList::AIIntegration(result))),
        Module::Chat => Chat::read_all(db)
            .await
            .and_then(|result| Ok(ModuleDataReadList::Chat(result))),
        Module::OAuth2App => OAuth2App::read_all(db)
            .await
            .and_then(|result| Ok(ModuleDataReadList::OAuth2App(result))),
        Module::EmailAccount => EmailAccount::read_all(db)
            .await
            .and_then(|result| Ok(ModuleDataReadList::EmailAccount(result))),
        Module::Mailbox => Mailbox::read_all(db)
            .await
            .and_then(|result| Ok(ModuleDataReadList::Mailbox(result))),
        _ => {
            error!("Invalid module {}", module.to_string());
            Err(DwataError::ModuleNotFound)
        }
    }
}

#[tauri::command]
pub async fn read_row_list_for_module_with_filter(
    filters: ModuleFilters,
    db: State<'_, Pool<Sqlite>>,
) -> Result<ModuleDataReadList, DwataError> {
    let db = db.deref();
    match filters {
        ModuleFilters::AIIntegration(x) => AIIntegration::read_with_filter(x, db)
            .await
            .and_then(|result| Ok(ModuleDataReadList::AIIntegration(result))),
        ModuleFilters::Chat(x) => Chat::read_with_filter(x, db)
            .await
            .and_then(|result| Ok(ModuleDataReadList::Chat(result))),
        _ => {
            error!("Invalid module {}", filters.to_string());
            Err(DwataError::ModuleNotFound)
        }
    }
}

#[tauri::command]
pub async fn read_module_item_by_pk(
    module: Module,
    pk: i64,
    db: State<'_, Pool<Sqlite>>,
) -> Result<ModuleDataRead, DwataError> {
    let db = db.deref();
    match module {
        Module::UserAccount => UserAccount::read_one_by_pk(pk, db)
            .await
            .and_then(|x| Ok(ModuleDataRead::UserAccount(x))),
        Module::DirectorySource => DirectorySource::read_one_by_pk(pk, db)
            .await
            .and_then(|x| Ok(ModuleDataRead::DirectorySource(x))),
        Module::DatabaseSource => DatabaseSource::read_one_by_pk(pk, db)
            .await
            .and_then(|x| Ok(ModuleDataRead::DatabaseSource(x))),
        Module::AIIntegration => AIIntegration::read_one_by_pk(pk, db)
            .await
            .and_then(|x| Ok(ModuleDataRead::AIIntegration(x))),
        Module::Chat => Chat::read_one_by_pk(pk, db)
            .await
            .and_then(|x| Ok(ModuleDataRead::Chat(x))),
        Module::OAuth2App => OAuth2App::read_one_by_pk(pk, db)
            .await
            .and_then(|x| Ok(ModuleDataRead::OAuth2App(x))),
        Module::EmailAccount => EmailAccount::read_one_by_pk(pk, db)
            .await
            .and_then(|x| Ok(ModuleDataRead::EmailAccount(x))),
        Module::Mailbox => Mailbox::read_one_by_pk(pk, db)
            .await
            .and_then(|x| Ok(ModuleDataRead::Mailbox(x))),
        _ => {
            error!("Invalid module {}", module.to_string());
            Err(DwataError::ModuleNotFound)
        }
    }
}

pub async fn insert_helper(
    data: ModuleDataCreateUpdate,
    db: &Pool<Sqlite>,
) -> Result<InsertUpdateResponse, DwataError> {
    match data {
        ModuleDataCreateUpdate::UserAccount(x) => x.insert_module_data(db).await,
        ModuleDataCreateUpdate::DirectorySource(x) => x.insert_module_data(db).await,
        ModuleDataCreateUpdate::DatabaseSource(x) => x.insert_module_data(db).await,
        ModuleDataCreateUpdate::AIIntegration(x) => x.insert_module_data(db).await,
        ModuleDataCreateUpdate::Chat(x) => x.insert_module_data(db).await,
        ModuleDataCreateUpdate::OAuth2App(x) => x.insert_module_data(db).await,
        ModuleDataCreateUpdate::EmailAccount(x) => x.insert_module_data(db).await,
    }
}

#[tauri::command]
pub async fn insert_module_item(
    data: ModuleDataCreateUpdate,
    db: State<'_, Pool<Sqlite>>,
) -> Result<InsertUpdateResponse, DwataError> {
    insert_helper(data, db.deref()).await
}

pub async fn update_helper(
    pk: i64,
    data: ModuleDataCreateUpdate,
    db: &Pool<Sqlite>,
) -> Result<InsertUpdateResponse, DwataError> {
    match data {
        ModuleDataCreateUpdate::UserAccount(x) => x.update_module_data(pk, db).await,
        ModuleDataCreateUpdate::DirectorySource(x) => x.update_module_data(pk, db).await,
        ModuleDataCreateUpdate::DatabaseSource(x) => x.update_module_data(pk, db).await,
        ModuleDataCreateUpdate::AIIntegration(x) => x.update_module_data(pk, db).await,
        ModuleDataCreateUpdate::Chat(x) => x.update_module_data(pk, db).await,
        ModuleDataCreateUpdate::OAuth2App(x) => x.update_module_data(pk, db).await,
        ModuleDataCreateUpdate::EmailAccount(x) => x.update_module_data(pk, db).await,
    }
}

#[tauri::command]
pub async fn update_module_item(
    pk: i64,
    data: ModuleDataCreateUpdate,
    db: State<'_, Pool<Sqlite>>,
) -> Result<InsertUpdateResponse, DwataError> {
    update_helper(pk, data, db.deref()).await
}

#[tauri::command]
pub async fn upsert_module_item(
    pk: i64,
    data: ModuleDataCreateUpdate,
    db: State<'_, Pool<Sqlite>>,
) -> Result<InsertUpdateResponse, DwataError> {
    let db = db.deref();
    let existing = match data {
        ModuleDataCreateUpdate::UserAccount(_) => UserAccount::read_one_by_pk(pk, db)
            .await
            .and_then(|x| Ok(ModuleDataRead::UserAccount(x))),
        ModuleDataCreateUpdate::DirectorySource(_) => DirectorySource::read_one_by_pk(pk, db)
            .await
            .and_then(|x| Ok(ModuleDataRead::DirectorySource(x))),
        ModuleDataCreateUpdate::DatabaseSource(_) => DatabaseSource::read_one_by_pk(pk, db)
            .await
            .and_then(|x| Ok(ModuleDataRead::DatabaseSource(x))),
        ModuleDataCreateUpdate::AIIntegration(_) => AIIntegration::read_one_by_pk(pk, db)
            .await
            .and_then(|x| Ok(ModuleDataRead::AIIntegration(x))),
        ModuleDataCreateUpdate::Chat(_) => Chat::read_one_by_pk(pk, db)
            .await
            .and_then(|x| Ok(ModuleDataRead::Chat(x))),
        ModuleDataCreateUpdate::OAuth2App(_) => OAuth2App::read_one_by_pk(pk, db)
            .await
            .and_then(|x| Ok(ModuleDataRead::OAuth2App(x))),
        ModuleDataCreateUpdate::EmailAccount(_) => EmailAccount::read_one_by_pk(pk, db)
            .await
            .and_then(|x| Ok(ModuleDataRead::EmailAccount(x))),
    };
    match existing {
        Ok(_) => {
            // We found an item in the DB, let us update it
            update_helper(pk, data, db).await
        }
        Err(_) => {
            // We did not find an existing item, so we insert one
            insert_helper(data, db).await
        }
    }
}
