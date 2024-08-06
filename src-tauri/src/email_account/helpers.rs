use super::{app_state::EmailAccountsState, EmailAccount};
use super::{EmailAccountStatus, Mailbox};
use crate::email::helpers::{parse_email_from_file, store_emails_to_tantity};
use crate::email::EmailFull;
use crate::email_account::MailboxCreateUpdate;
use crate::error::DwataError;
use crate::workspace::crud::{CRUDCreateUpdate, CRUDRead};
use crate::workspace::{ProcessInLog, ProcessLogCreateUpdate, ProcessingStatusInLog};
use chrono::{TimeDelta, Utc};
use imap::Session;
use log::{error, info};
use native_tls::TlsStream;
use slug::slugify;
use sqlx::{Pool, Sqlite};
use std::collections::HashSet;
use std::fs::{create_dir_all, read_dir, File};
use std::io::{self, Read};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::{sync::Mutex, task::JoinSet};

// pub async fn check_date_of_emails_in_storage(storage_dir: PathBuf) -> Result<bool, DwataError> {}

impl EmailAccount {
    pub async fn prep_mailbox_for_access(
        &mut self,
        mailbox: Mailbox,
        storage_dir: &PathBuf,
        needs_imap_session: bool,
        db: &Pool<Sqlite>,
        email_account_state: &EmailAccountsState,
    ) -> Result<usize, DwataError> {
        let mut num_messages: usize = 0;
        {
            // We ckeck if we have already fetched emails within last 60 seconds
            let mut locked_state = email_account_state.lock().await;
            // Filter the vec of EmailAccountStatus to find the one that matches this EmailAccount
            match locked_state
                .iter_mut()
                .find(|x| x.id == self.id && x.mailbox.id == mailbox.id)
            {
                Some(x) => {
                    if let Some(last_fetched_at) = x.last_fetched_at {
                        if last_fetched_at > Utc::now().timestamp() - 60 {
                            error!("Task EmailAccount::prep_for_access has run recently for {}, mailbox {}", self.email_address, mailbox.id);
                            return Err(DwataError::TaskHasRunRecently);
                        }
                    }
                }
                None => {
                    let mut storage_dir = storage_dir.clone();
                    storage_dir.push("emails");
                    storage_dir.push(self.email_address.clone());
                    locked_state.push(EmailAccountStatus {
                        id: self.id,
                        mailbox,
                        storage_dir: storage_dir.clone(),
                        last_fetched_at: Some(Utc::now().timestamp()),
                        imap_session: None,
                    });
                }
            }
        }

        if needs_imap_session {
            self.sync_mailboxes(db, email_account_state).await?;
            let imap_session = self.create_imap_session(db, email_account_state).await?;
            let mut imap_session = imap_session.lock().await;
            let selected_mailbox = self.get_selected_mailbox(db, email_account_state).await?;
            match imap_session.examine(&selected_mailbox.name) {
                Ok(mailbox) => {
                    info!(
                        "Selected mailbox: {}, has {} messages, uid_next: {:?}, uid_validity: {:?}",
                        selected_mailbox.name,
                        mailbox.exists,
                        mailbox.uid_next,
                        mailbox.uid_validity
                    );
                    // TODO: If uid_validity changes then we need to (perhaps delete and) refetch all the emails
                    // We check if we there are any new emails as per IMAP UID_NEXT
                    let existing = Mailbox::read_one_by_pk(selected_mailbox.id, db).await?;
                    if mailbox.uid_validity == existing.uid_validity
                        && mailbox.uid_next == existing.uid_next
                    {
                        // We have no new emails
                        info!(
                            "Mailbox {} of email account {} has no new emails",
                            selected_mailbox.name, self.email_address
                        );
                        return Err(DwataError::MailboxHasNoNewEmails);
                    }
                    MailboxCreateUpdate {
                        messages: Some(mailbox.exists),
                        uid_next: mailbox.uid_next,
                        uid_validity: mailbox.uid_validity,
                        ..Default::default()
                    }
                    .update_module_data(selected_mailbox.id, db)
                    .await?;
                    num_messages = mailbox.exists as usize;
                }
                Err(err) => {
                    error!(
                        "Error selecting mailbox {} (ID: {}) of email account {} - {}",
                        selected_mailbox.name, selected_mailbox.id, self.email_address, err
                    );
                    return Err(DwataError::CouldNotSelectMailbox);
                }
            };
        }

        Ok(num_messages)
    }

    pub async fn sync_mailboxes(
        &mut self,
        db: &Pool<Sqlite>,
        email_account_state: &EmailAccountsState,
    ) -> Result<(), DwataError> {
        // Store all the mailboxes in Dwata DB
        let imap_session = self.get_imap_session(db, email_account_state, true).await?;
        let mut imap_session = imap_session.lock().await;
        let mailboxes_in_db = Mailbox::read_all(db).await?;
        match imap_session.list(None, Some("*")) {
            Ok(mailboxes) => {
                for mailbox in mailboxes.iter() {
                    let mailbox_name = mailbox.name().trim().to_string();
                    if mailboxes_in_db.len() == 0
                        || mailboxes_in_db
                            .iter()
                            .find(|mb| mb.email_account_id == self.id && mb.name == mailbox_name)
                            .is_none()
                    {
                        // We did not find this mailbox in Dwata DB, let's add it
                        info!("Saving mailbox: {}", mailbox.name());
                        MailboxCreateUpdate {
                            email_account_id: Some(self.id),
                            name: Some(mailbox_name.clone()),
                            storage_path: Some(slugify(mailbox_name)),
                            ..Default::default()
                        }
                        .insert_module_data(db)
                        .await?;
                    }
                }
                Ok(())
            }
            Err(err) => {
                error!("Could not list mailboxes - {}", err);
                Err(DwataError::CouldNotListMailboxes)
            }
        }
    }

    pub async fn fetch_emails(
        &mut self,
        num_messages: usize,
        db: &Pool<Sqlite>,
        email_account_state: &EmailAccountsState,
    ) -> Result<(), DwataError> {
        // We read emails using IMAP and store the raw messages in a local folder
        info!("Fetching emails for {}", self.email_address);
        let shared_email_uid_list = Arc::new(Mutex::new(HashSet::<u32>::new()));
        let fetch_batch_limit = 25;
        let mut month = 0;
        let mut before = Utc::now().date_naive();

        ProcessLogCreateUpdate {
            process: Some(ProcessInLog::CheckEmails),
            arguments: Some(vec![("id".to_string(), self.id.to_string())]),
            status: Some(ProcessingStatusInLog::InProgress),
            is_sent_to_frontend: Some(false),
        }
        .insert_module_data(db)
        .await?;
        // Let's retrieve all the email Uids in the last 300 months (25 years)
        while month < 300 {
            month += 1;
            let moved_imap_session = self
                .get_imap_session(db, email_account_state, false)
                .await?;
            let moved_email_uid_list = shared_email_uid_list.clone();
            let since = before - TimeDelta::weeks(4);
            fetch_uid_list_of_emails(
                moved_email_uid_list,
                moved_imap_session,
                since.format("%d-%b-%Y").to_string(),
                before.format("%d-%b-%Y").to_string(),
            )
            .await;
            before = since;
            if num_messages > shared_email_uid_list.lock().await.len() {
                break;
            }
        }
        info!("Finished all tasks to fetch email uids");
        ProcessLogCreateUpdate {
            process: Some(ProcessInLog::CheckEmails),
            arguments: Some(vec![("id".to_string(), self.id.to_string())]),
            status: Some(ProcessingStatusInLog::Completed),
            is_sent_to_frontend: Some(false),
        }
        .insert_module_data(db)
        .await?;

        // Let's fetch the actual emails, in batches
        let email_uid_list = shared_email_uid_list.lock().await;
        let mut fetch_email_set = JoinSet::new();
        let mut fetched_email_count = 0;
        while fetched_email_count < email_uid_list.len() {
            let moved_imap_session = self
                .get_imap_session(db, email_account_state, false)
                .await?;
            let moved_storage_dir = self
                .get_selected_mailbox(db, email_account_state)
                .await?
                .get_storage_dir(email_account_state)
                .await?;
            // We select the next batch of email Uids to fetch
            let mut batch: Vec<u32> = vec![];
            for uid in email_uid_list
                .iter()
                .skip(fetched_email_count)
                .take(fetch_batch_limit)
            {
                batch.push(*uid);
            }
            if batch.len() > 0 {
                fetched_email_count += batch.len();
                fetch_email_set.spawn(async move {
                    fetch_emails_for_uid_list(batch, moved_imap_session, moved_storage_dir).await;
                });
            }
        }
        if fetch_email_set.len() > 0 {
            while let Some(_) = fetch_email_set.join_next().await {}
        }
        info!("Finished all tasks to fetch emails");

        let imap_session = self
            .get_imap_session(db, email_account_state, false)
            .await?;
        let mut imap_session = imap_session.lock().await;
        match imap_session.logout() {
            Ok(_) => {}
            Err(err) => {
                error!("Could not logout IMAP session - {}", err);
            }
        }
        Ok(())
    }

    pub async fn index_emails_for_search(
        &self,
        storage_dir: &PathBuf,
        db: &Pool<Sqlite>,
        email_account_state: &EmailAccountsState,
    ) -> Result<(), DwataError> {
        // We index emails in Tantivy search engine
        let mailbox = self.get_selected_mailbox(db, email_account_state).await?;
        let emails = mailbox
            .read_emails_from_local_storage(1000, email_account_state)
            .await?;
        store_emails_to_tantity(&self, &mailbox, &emails, storage_dir)?;

        Ok(())
    }
}

impl Mailbox {
    pub async fn get_by_name(
        email_account_id: i64,
        name: &str,
        db: &Pool<Sqlite>,
    ) -> Result<Self, DwataError> {
        let mailboxes = Mailbox::read_all(db).await?;
        let name = name.to_lowercase();
        mailboxes
            .into_iter()
            .find(|x| {
                x.email_account_id == email_account_id && x.name.trim().to_lowercase() == name
            })
            .ok_or(DwataError::InvalidMailbox)
    }

    pub async fn get_storage_dir(
        &self,
        email_account_state: &EmailAccountsState,
    ) -> Result<PathBuf, DwataError> {
        // We find the storage directory for this mailbox, which is inside the EmailAccount storage directory
        let locked_state = email_account_state.lock().await;
        // Filter the vec of EmailAccountStatus to find the one that matches this EmailAccount
        match locked_state.iter().find(|x| x.id == self.email_account_id) {
            Some(x) => {
                // Create all the directories in the storage path
                let mut storage_path = x.storage_dir.clone();
                storage_path.push(slugify(&self.storage_path));
                match create_dir_all(storage_path.as_path()) {
                    Ok(_) => Ok(x.storage_dir.join(slugify(&self.storage_path))),
                    Err(_) => Err(DwataError::CouldNotCreateLocalEmailStorage),
                }
            }
            None => Err(DwataError::AppStateNotFound),
        }
    }

    pub async fn read_emails_from_local_storage(
        &self,
        limit: usize,
        email_account_state: &EmailAccountsState,
    ) -> Result<Vec<EmailFull>, DwataError> {
        // We parse the files that are in the given email storage directory
        // and extract Email structs.
        let mut emails: Vec<EmailFull> = vec![];
        let storage_dir = self.get_storage_dir(email_account_state).await?;
        info!(
            "Reading emails from local storage directory: {}",
            self.get_storage_dir(email_account_state)
                .await?
                .to_str()
                .unwrap()
        );
        // Read all email files in the local folder
        match read_dir(&storage_dir) {
            Ok(entries) => {
                match entries
                    .map(|res| res.map(|e| e.path()))
                    .collect::<Result<Vec<_>, io::Error>>()
                {
                    Ok(mut entries) => {
                        // We sort by the file name but parsed as integer
                        entries.sort_by_key(|a| {
                            a.file_name()
                                .unwrap()
                                .to_string_lossy()
                                .to_string()
                                .parse::<i64>()
                                .unwrap_or(0)
                        });
                        entries.reverse();

                        for entry in entries.iter().take(limit) {
                            let path = entry.as_path();
                            if path.is_file() {
                                // We read the file contents and parse it to extract the email
                                let mut file = match File::open(&path) {
                                    Ok(x) => x,
                                    Err(err) => {
                                        error!(
                                            "Could not open email file {}\n Error: {}",
                                            path.to_str().unwrap(),
                                            err
                                        );
                                        continue;
                                    }
                                };
                                let mut file_contents: Vec<u8> = Vec::new();
                                match file.read_to_end(&mut file_contents) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        error!(
                                            "Could not read email file {}\n Error: {}",
                                            path.to_str().unwrap(),
                                            err
                                        );
                                        continue;
                                    }
                                };
                                // Extract the email UID from the file name, which has .email at the end
                                match parse_email_from_file(
                                    &self,
                                    path.file_name().unwrap().to_string_lossy().to_string(),
                                    &file_contents,
                                )
                                .await
                                {
                                    Ok(email) => {
                                        emails.push(email);
                                    }
                                    Err(err) => {
                                        error!("Could not parse email file\n Error: {}", err);
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                    Err(err) => {
                        error!(
                            "Could not read email storage directory {}\n Error: {}",
                            storage_dir.to_string_lossy(),
                            err
                        )
                    }
                }
            }
            Err(err) => {
                error!(
                    "Could not read email storage directory {}\n Error: {}",
                    storage_dir.to_string_lossy(),
                    err
                )
            }
        }

        Ok(emails)
    }
}

pub async fn fetch_and_save_emails_for_email_account(
    pk: i64,
    storage_dir: &PathBuf,
    db: &Pool<Sqlite>,
    email_account_state: &EmailAccountsState,
) -> Result<(), DwataError> {
    let mut email_account = EmailAccount::read_one_by_pk(pk, db).await?;
    // Let's work with the Sent folder
    let num_messages = email_account
        .prep_mailbox_for_access(
            Mailbox::get_by_name(pk, "sent", db).await?,
            storage_dir,
            true,
            db,
            email_account_state,
        )
        .await?;
    email_account
        .fetch_emails(num_messages, db, email_account_state)
        .await?;
    email_account
        .index_emails_for_search(storage_dir, db, email_account_state)
        .await?;

    // Let's work with the Inbox folder
    let num_messages = email_account
        .prep_mailbox_for_access(
            Mailbox::get_by_name(pk, "inbox", db).await?,
            storage_dir,
            true,
            db,
            email_account_state,
        )
        .await?;
    email_account
        .fetch_emails(num_messages, db, email_account_state)
        .await?;
    email_account
        .index_emails_for_search(storage_dir, db, email_account_state)
        .await?;
    Ok(())
}

pub async fn fetch_uid_list_of_emails(
    shared_email_uid_list: Arc<Mutex<HashSet<u32>>>,
    shared_imap_session: Arc<Mutex<Session<TlsStream<TcpStream>>>>,
    since_date: String,
    before_date: String,
) {
    let mut email_uid_list = shared_email_uid_list.lock().await;
    if email_uid_list.len() > 1000 {
        // Temporarily we are limiting the number of emails to fetch to 1000
        return;
    }
    let mut imap_session = shared_imap_session.lock().await;
    match imap_session.uid_search(format!("SINCE {} BEFORE {}", since_date, before_date)) {
        Ok(data) => {
            email_uid_list.extend(&data);
            info!(
                "Searched email Uids SINCE {} BEFORE {}, found {} email Uids",
                since_date,
                before_date,
                &data.len()
            );
        }
        Err(err) => {
            error!("(fetch_email_uid_list) Error searching for emails: {}", err);
        }
    }
}

pub async fn fetch_emails_for_uid_list(
    email_uid_list: Vec<u32>,
    shared_imap_session: Arc<Mutex<Session<TlsStream<TcpStream>>>>,
    storage_dir: PathBuf,
) {
    let mut imap_session = shared_imap_session.lock().await;
    // Let's find out if the files already exist.
    // If they do, then we don't need to fetch them again.
    let mut file_uid_list: Vec<u32> = vec![];
    for uid in email_uid_list {
        let mut file_path = storage_dir.clone();
        file_path.push(format!("{}", uid));
        if !file_path.exists() {
            file_uid_list.push(uid);
        }
    }

    match imap_session.uid_fetch(
        file_uid_list
            .iter()
            .map(|x| format!("{}", x))
            .collect::<Vec<String>>()
            .join(","),
        "(BODY.PEEK[] UID)",
    ) {
        Ok(messages) => {
            for message in messages.iter() {
                match message.body() {
                    Some(body) => {
                        // Store the email in a local folder
                        let mut file_path = storage_dir.clone();
                        file_path.push(format!("{}", message.uid.unwrap()));
                        std::fs::write(file_path, body).unwrap();
                    }
                    None => continue,
                }
            }
            info!("Fetched and stored {} emails", file_uid_list.len());
        }
        Err(e) => error!("Error Fetching email 1: {}", e),
    };
}

// pub async fn save_emails_to_db(email_batch: Vec<u32>, app: AppHandle) {
//     let mut db_guard = db.lock().await;
// }
