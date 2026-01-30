use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use oauth2::PkceCodeVerifier;

pub struct OAuthStateManager {
    states: Arc<Mutex<HashMap<String, PkceCodeVerifier>>>,
}

impl OAuthStateManager {
    pub fn new() -> Self {
        Self {
            states: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn store_verifier(&self, state: String, verifier: PkceCodeVerifier) {
        let mut states = self.states.lock().await;
        states.insert(state, verifier);
    }

    pub async fn retrieve_verifier(&self, state: &str) -> Option<PkceCodeVerifier> {
        let mut states = self.states.lock().await;
        states.remove(state)
    }
}
