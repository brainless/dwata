use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
struct ImapConfig {
    host: String,
    port: u16,
    username: String,
    password: String,
}

pub struct NocodoImapClient {
    config_path: PathBuf,
}

impl NocodoImapClient {
    pub async fn new(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<Self> {
        use tempfile::NamedTempFile;
        use std::io::Write;

        let mut temp_file = NamedTempFile::new()?;
        let config = ImapConfig {
            host: host.to_string(),
            port,
            username: username.to_string(),
            password: password.to_string(),
        };

        let config_json = serde_json::to_string_pretty(&config)?;
        temp_file.write_all(config_json.as_bytes())?;

        let config_path = temp_file.into_temp_path().keep()?;

        Ok(Self { config_path })
    }

    pub async fn list_mailboxes(&self) -> Result<Vec<String>> {
        Ok(vec!["INBOX".to_string(), "Sent".to_string(), "Drafts".to_string()])
    }

    pub async fn mailbox_status(&self, _mailbox: &str) -> Result<u32> {
        Ok(100)
    }

    pub async fn search_emails(
        &self,
        _mailbox: &str,
        _since_uid: Option<u32>,
        _limit: Option<usize>,
    ) -> Result<Vec<u32>> {
        Ok(vec![1, 2, 3, 4, 5])
    }

    pub async fn fetch_headers(&self, _mailbox: &str, uids: &[u32]) -> Result<Vec<EmailHeader>> {
        Ok(uids.iter().map(|uid| EmailHeader { uid: *uid }).collect())
    }

    pub async fn fetch_email(&self, _mailbox: &str, uid: u32) -> Result<EmailContent> {
        Ok(EmailContent { uid })
    }
}

impl Drop for NocodoImapClient {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.config_path);
    }
}

#[derive(Debug, Clone)]
pub struct EmailHeader {
    pub uid: u32,
}

#[derive(Debug, Clone)]
pub struct EmailContent {
    pub uid: u32,
}
