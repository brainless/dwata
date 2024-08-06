use super::helpers::search_emails_from_tantivy;
use crate::{
    email_account::{EmailAccount, Mailbox},
    error::DwataError,
    workspace::{
        crud::{CRUDRead, CRUDReadFilter, InputValue, VecColumnNameValue},
        ModuleDataReadList, ModuleFilters,
    },
};
use log::error;
use sqlx::{Pool, Sqlite};
use std::ops::Deref;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub async fn search_emails(
    filters: ModuleFilters,
    app: AppHandle,
    db: State<'_, Pool<Sqlite>>,
) -> Result<ModuleDataReadList, DwataError> {
    match filters {
        ModuleFilters::Email(x) => {
            let column_names_values: VecColumnNameValue = x.get_column_names_values_to_filter();
            let mailbox_id = match column_names_values.find_by_name("mailbox_id") {
                Some(InputValue::ID(x)) => x,
                _ => return Err(DwataError::InvalidMailbox),
            };
            let db = db.deref();
            let mailbox: Mailbox = Mailbox::read_one_by_pk(mailbox_id, db).await?;
            let search_query: Option<String> =
                match column_names_values.find_by_name("search_query") {
                    Some(InputValue::Text(x)) => Some(x),
                    _ => None,
                };
            let email_account: EmailAccount =
                EmailAccount::read_one_by_pk(mailbox.email_account_id, db).await?;
            match search_emails_from_tantivy(
                search_query,
                &email_account,
                &mailbox,
                &app.path().app_data_dir().unwrap(),
            )
            .await
            {
                Ok(emails) => Ok(ModuleDataReadList::Email(emails)),
                Err(err) => Err(err),
            }
        }
        _ => {
            error!("Invalid module {}", filters.to_string());
            Err(DwataError::ModuleNotFound)
        }
    }
}
