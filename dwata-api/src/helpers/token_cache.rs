use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc, Duration};

#[derive(Clone)]
pub struct CachedToken {
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
}

pub struct TokenCache {
    tokens: Arc<Mutex<HashMap<i64, CachedToken>>>,
}

impl TokenCache {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn store_token(&self, credential_id: i64, access_token: String, expires_in_seconds: i64) {
        let expires_at = Utc::now() + Duration::seconds(expires_in_seconds);
        let cached = CachedToken {
            access_token,
            expires_at,
        };

        let mut tokens = self.tokens.lock().await;
        tokens.insert(credential_id, cached);
    }

    pub async fn get_token(&self, credential_id: i64) -> Option<String> {
        let tokens = self.tokens.lock().await;
        if let Some(cached) = tokens.get(&credential_id) {
            if Utc::now() < cached.expires_at - Duration::minutes(5) {
                return Some(cached.access_token.clone());
            }
        }
        None
    }

    pub async fn invalidate_token(&self, credential_id: i64) {
        let mut tokens = self.tokens.lock().await;
        tokens.remove(&credential_id);
    }
}
