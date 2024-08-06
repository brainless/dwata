use super::EmailAccountStatus;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type EmailAccountsState = Arc<Mutex<Vec<EmailAccountStatus>>>;
