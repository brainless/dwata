use anyhow::{Result, Context};
use imap::ClientBuilder;
use mail_parser::MessageParser;
use chrono::{Utc, Duration};

pub struct RealImapClient {
    session: imap::Session<imap::Connection>,
}

impl RealImapClient {
    pub async fn connect_with_password(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<Self> {
        let client = ClientBuilder::new(host, port)
            .connect()
            .context("Failed to connect to IMAP server")?;

        let session = client
            .login(username, password)
            .map_err(|e| anyhow::anyhow!("IMAP login failed: {:?}", e))?;

        Ok(Self { session })
    }

    pub async fn connect_with_oauth(
        host: &str,
        port: u16,
        username: &str,
        access_token: &str,
    ) -> Result<Self> {
        struct GmailOAuth2 {
            user: String,
            access_token: String,
        }

        impl imap::Authenticator for GmailOAuth2 {
            type Response = String;
            #[allow(unused_variables)]
            fn process(&self, _data: &[u8]) -> Self::Response {
                format!(
                    "user={}\x01auth=Bearer {}\x01\x01",
                    self.user, self.access_token
                )
            }
        }

        let gmail_auth = GmailOAuth2 {
            user: username.to_string(),
            access_token: access_token.to_string(),
        };

        let client = ClientBuilder::new(host, port)
            .connect()
            .context("Failed to connect to IMAP server")?;

        let session = client
            .authenticate("XOAUTH2", &gmail_auth)
            .map_err(|e| anyhow::anyhow!("OAuth2 IMAP auth failed: {:?}", e))?;

        Ok(Self { session })
    }

    pub fn list_mailboxes(&mut self) -> Result<Vec<String>> {
        let mailboxes = self.session.list(None, Some("*"))?;
        Ok(mailboxes
            .iter()
            .map(|m| m.name().to_string())
            .collect())
    }

    pub fn list_folders_with_metadata(&mut self) -> Result<Vec<FolderMetadata>> {
        let mailboxes = self.session.list(None, Some("*"))?;
        let mut folders = Vec::new();

        for mailbox in mailboxes.iter() {
            let name = mailbox.name().to_string();
            let delim = mailbox.delimiter();

            // Assume folders are selectable and subscribed by default
            // TODO: Properly parse NameAttribute when API is stabilized
            let is_selectable = true;
            let is_subscribed = false;

            folders.push(FolderMetadata {
                name: name.clone(),
                imap_path: name,
                delimiter: delim.map(|d| d.to_string()),
                is_selectable,
                is_subscribed,
            });
        }

        Ok(folders)
    }

    pub fn mailbox_status(&mut self, mailbox: &str) -> Result<u32> {
        let mailbox_info = self.session.select(mailbox)?;
        Ok(mailbox_info.exists)
    }

    pub fn search_emails(
        &mut self,
        mailbox: &str,
        since_uid: Option<u32>,
        max_age_months: Option<u32>,
        limit: Option<usize>,
    ) -> Result<Vec<u32>> {
        self.session.select(mailbox)?;

        let cutoff_date = if let Some(months) = max_age_months {
            let date = Utc::now() - Duration::days((months as i64) * 30);
            Some(date)
        } else {
            None
        };

        let mut query = String::new();

        if let Some(date) = cutoff_date {
            let date_str = date.format("%d-%b-%Y").to_string();
            query.push_str(&format!("SINCE {}", date_str));
        } else {
            query.push_str("ALL");
        }

        if let Some(uid) = since_uid {
            if !query.is_empty() {
                query.push(' ');
            }
            query.push_str(&format!("UID {}:*", uid + 1));
        }

        tracing::info!("IMAP SEARCH query: {}", query);

        let uids = self.session.uid_search(&query)?;

        let limited_uids: Vec<u32> = if let Some(lim) = limit {
            uids.into_iter().take(lim).collect()
        } else {
            uids.into_iter().collect()
        };

        Ok(limited_uids)
    }

    pub fn fetch_email(&mut self, mailbox: &str, uid: u32) -> Result<ParsedEmail> {
        self.session.select(mailbox)?;

        let messages = self.session.uid_fetch(uid.to_string(), "RFC822")?;

        let message = messages.iter().next()
            .context("Email not found")?;

        let body = message.body()
            .context("Email has no body")?;

        let parser = MessageParser::default();
        let parsed = parser.parse(body)
            .context("Failed to parse email")?;

        let subject = parsed.subject().map(|s| s.to_string());
        let from = parsed.from()
            .and_then(|addrs| addrs.first())
            .map(|addr| (
                addr.address().map(|a| a.to_string()),
                addr.name().map(|n| n.to_string()),
            ));

        let to_addresses: Vec<(Option<String>, Option<String>)> = parsed.to()
            .map(|addrs| {
                addrs.iter()
                    .map(|addr| (
                        addr.address().map(|a| a.to_string()),
                        addr.name().map(|n| n.to_string()),
                    ))
                    .collect()
            })
            .unwrap_or_default();

        let body_text = parsed.body_text(0).map(|s| s.to_string());
        let body_html = parsed.body_html(0).map(|s| s.to_string());

        let date_sent = parsed.date()
            .map(|dt| dt.to_timestamp() * 1000);

        let message_id = parsed.message_id().map(|s| s.to_string());

        let flags = message.flags();
        let is_read = flags.contains(&imap::types::Flag::Seen);
        let is_flagged = flags.contains(&imap::types::Flag::Flagged);
        let is_draft = flags.contains(&imap::types::Flag::Draft);
        let is_answered = flags.contains(&imap::types::Flag::Answered);

        let date_received = message.internal_date()
            .map(|dt| dt.timestamp_millis())
            .unwrap_or_else(|| Utc::now().timestamp_millis());

        let size_bytes = message.size;

        let has_attachments = parsed.attachment_count() > 0;
        let attachment_count = parsed.attachment_count();

        Ok(ParsedEmail {
            uid,
            message_id,
            subject,
            from_address: from.as_ref().and_then(|(addr, _)| addr.clone()),
            from_name: from.and_then(|(_, name)| name),
            to_addresses,
            cc_addresses: vec![],
            bcc_addresses: vec![],
            reply_to: None,
            date_sent,
            date_received,
            body_text,
            body_html,
            is_read,
            is_flagged,
            is_draft,
            is_answered,
            has_attachments,
            attachment_count: attachment_count as i32,
            size_bytes: size_bytes.map(|s| s as i32),
            labels: vec![],
        })
    }
}

pub struct ParsedEmail {
    pub uid: u32,
    pub message_id: Option<String>,
    pub subject: Option<String>,
    pub from_address: Option<String>,
    pub from_name: Option<String>,
    pub to_addresses: Vec<(Option<String>, Option<String>)>,
    pub cc_addresses: Vec<(Option<String>, Option<String>)>,
    pub bcc_addresses: Vec<(Option<String>, Option<String>)>,
    pub reply_to: Option<String>,
    pub date_sent: Option<i64>,
    pub date_received: i64,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub is_read: bool,
    pub is_flagged: bool,
    pub is_draft: bool,
    pub is_answered: bool,
    pub has_attachments: bool,
    pub attachment_count: i32,
    pub size_bytes: Option<i32>,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FolderMetadata {
    pub name: String,
    pub imap_path: String,
    pub delimiter: Option<String>,
    pub is_selectable: bool,
    pub is_subscribed: bool,
}
