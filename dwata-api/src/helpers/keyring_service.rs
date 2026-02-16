use keyring::Entry;
use shared_types::credential::CredentialType;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug)]
pub enum KeyringError {
    NotFound,
    ServiceUnavailable(String),
    OperationFailed(String),
}

impl std::fmt::Display for KeyringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyringError::NotFound => write!(f, "Credential not found in keychain"),
            KeyringError::ServiceUnavailable(msg) => {
                write!(f, "Keychain service unavailable: {}", msg)
            }
            KeyringError::OperationFailed(msg) => write!(f, "Keychain operation failed: {}", msg),
        }
    }
}

impl std::error::Error for KeyringError {}

#[derive(Clone)]
struct CachedPassword {
    password: String,
    expires_at: Instant,
}

/// KeyringService with in-memory caching to reduce OS keychain prompts
///
/// The cache stores passwords with a TTL (default 1 hour) to balance
/// security and convenience. On macOS, when first prompted for keychain
/// access, users should select "Always Allow" to avoid repeated prompts.
#[derive(Clone)]
pub struct KeyringService {
    cache: Arc<RwLock<HashMap<String, CachedPassword>>>,
    cache_ttl: Duration,
}

impl KeyringService {
    /// Create a new KeyringService with default cache TTL of 1 hour
    pub fn new() -> Self {
        Self::with_ttl(Duration::from_secs(3600))
    }

    /// Create a new KeyringService with a custom cache TTL
    pub fn with_ttl(cache_ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl,
        }
    }

    /// Generate cache key for a credential
    fn cache_key(credential_type: &CredentialType, identifier: &str, username: &str) -> String {
        format!("{}:{}:{}", credential_type.as_str(), identifier, username)
    }

    /// Generate keychain username (identifier:username)
    fn keychain_username(identifier: &str, username: &str) -> String {
        format!("{}:{}", identifier, username)
    }

    /// Fetch password from OS keychain (bypassing cache)
    fn fetch_from_keychain(
        credential_type: &CredentialType,
        identifier: &str,
        username: &str,
    ) -> Result<String, KeyringError> {
        let service = credential_type.service_name();
        let keychain_user = Self::keychain_username(identifier, username);

        let entry = Entry::new(&service, &keychain_user).map_err(|e| {
            KeyringError::ServiceUnavailable(format!("Failed to create keychain entry: {}", e))
        })?;

        entry.get_password().map_err(|e| {
            if e.to_string().contains("not found") || e.to_string().contains("NotFound") {
                KeyringError::NotFound
            } else {
                KeyringError::OperationFailed(format!("Failed to retrieve password: {}", e))
            }
        })
    }

    /// Get password with caching - checks cache first, then falls back to keychain
    /// Always uses master credentials mode (single keychain entry)
    pub async fn get_password(
        &self,
        credential_type: &CredentialType,
        identifier: &str,
        username: &str,
    ) -> Result<String, KeyringError> {
        let cache_key = Self::cache_key(credential_type, identifier, username);

        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                if cached.expires_at > Instant::now() {
                    tracing::debug!("Cache hit for credential: {}", cache_key);
                    return Ok(cached.password.clone());
                } else {
                    tracing::debug!("Cache expired for credential: {}", cache_key);
                }
            }
        }

        // Cache miss or expired - fetch from master keychain entry
        tracing::debug!(
            "Cache miss for credential: {}, fetching from master keychain",
            cache_key
        );

        // Load all credentials from master entry (only 1 keychain prompt)
        let all_creds = self.get_master_credentials().await?;

        // Find the requested credential
        for (cred_type, cred_id, cred_user, password) in all_creds {
            if cred_type == *credential_type && cred_id == identifier && cred_user == username {
                return Ok(password);
            }
        }

        Err(KeyringError::NotFound)
    }

    /// Set password in keychain and update cache
    /// Always uses master credentials mode (single keychain entry)
    pub async fn set_password(
        &self,
        credential_type: &CredentialType,
        identifier: &str,
        username: &str,
        password: &str,
    ) -> Result<(), KeyringError> {
        // Load existing credentials from master entry
        let mut all_creds = match self.get_master_credentials().await {
            Ok(creds) => creds,
            Err(KeyringError::NotFound) => {
                // Master entry doesn't exist yet, create empty list
                Vec::new()
            }
            Err(e) => return Err(e),
        };

        // Update or add the credential
        let mut found = false;
        for (cred_type, cred_id, cred_user, cred_pass) in &mut all_creds {
            if *cred_type == *credential_type && cred_id == identifier && cred_user == username {
                *cred_pass = password.to_string();
                found = true;
                break;
            }
        }

        if !found {
            all_creds.push((
                credential_type.clone(),
                identifier.to_string(),
                username.to_string(),
                password.to_string(),
            ));
        }

        // Save back to master entry
        self.set_master_credentials(all_creds).await?;

        // Update cache
        let cache_key = Self::cache_key(credential_type, identifier, username);
        let mut cache = self.cache.write().await;
        cache.insert(
            cache_key,
            CachedPassword {
                password: password.to_string(),
                expires_at: Instant::now() + self.cache_ttl,
            },
        );

        Ok(())
    }

    /// Update password in keychain and cache (uses master mode)
    pub async fn update_password(
        &self,
        credential_type: &CredentialType,
        identifier: &str,
        username: &str,
        new_password: &str,
    ) -> Result<(), KeyringError> {
        // Just call set_password, which handles updates in master mode
        self.set_password(credential_type, identifier, username, new_password)
            .await
    }

    /// Delete password from keychain and remove from cache (uses master mode)
    pub async fn delete_password(
        &self,
        credential_type: &CredentialType,
        identifier: &str,
        username: &str,
    ) -> Result<(), KeyringError> {
        // Load all credentials from master entry
        let mut all_creds = self.get_master_credentials().await?;

        // Remove the credential
        all_creds.retain(|(cred_type, cred_id, cred_user, _)| {
            !(*cred_type == *credential_type && cred_id == identifier && cred_user == username)
        });

        // Save back to master entry
        self.set_master_credentials(all_creds).await?;

        // Remove from cache
        let cache_key = Self::cache_key(credential_type, identifier, username);
        let mut cache = self.cache.write().await;
        cache.remove(&cache_key);

        Ok(())
    }

    /// Preload credentials into cache from keychain
    ///
    /// This should be called at startup with all active credentials to populate
    /// the cache and trigger any OS keychain prompts upfront (where users can
    /// select "Always Allow" on macOS).
    pub async fn preload_credentials(
        &self,
        credentials: Vec<(CredentialType, String, String)>, // (type, identifier, username)
    ) {
        tracing::info!("Preloading {} credentials into cache", credentials.len());

        let mut success_count = 0;
        let mut error_count = 0;

        for (credential_type, identifier, username) in credentials {
            match Self::fetch_from_keychain(&credential_type, &identifier, &username) {
                Ok(password) => {
                    let cache_key = Self::cache_key(&credential_type, &identifier, &username);
                    let mut cache = self.cache.write().await;
                    cache.insert(
                        cache_key,
                        CachedPassword {
                            password,
                            expires_at: Instant::now() + self.cache_ttl,
                        },
                    );
                    success_count += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to preload credential {}:{} - {}",
                        identifier,
                        username,
                        e
                    );
                    error_count += 1;
                }
            }
        }

        tracing::info!(
            "Preload complete: {} successful, {} errors",
            success_count,
            error_count
        );
    }

    /// Invalidate a specific credential in the cache
    pub async fn invalidate(
        &self,
        credential_type: &CredentialType,
        identifier: &str,
        username: &str,
    ) {
        let cache_key = Self::cache_key(credential_type, identifier, username);
        let mut cache = self.cache.write().await;
        cache.remove(&cache_key);
        tracing::debug!("Invalidated cache for credential: {}", cache_key);
    }

    /// Clear entire cache (useful for security purposes)
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        let count = cache.len();
        cache.clear();
        tracing::info!("Cleared {} credentials from cache", count);
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        let total = cache.len();
        let now = Instant::now();
        let expired = cache.values().filter(|c| c.expires_at <= now).count();
        (total, expired)
    }

    // ============================================================================
    // Master Credentials Mode - Single keychain entry for all credentials
    // ============================================================================

    /// Store all credentials in a single master keychain entry (batched mode)
    ///
    /// This reduces OS keychain prompts from N to 1 by storing all credentials
    /// as encrypted JSON in a single keychain entry. Works identically on all OS.
    ///
    /// Trade-off: All credentials in one basket vs. individual isolation
    pub async fn set_master_credentials(
        &self,
        credentials: Vec<(CredentialType, String, String, String)>, // (type, identifier, username, password)
    ) -> Result<(), KeyringError> {
        use serde_json::json;

        // Build JSON structure
        let credentials_json: Vec<_> = credentials
            .iter()
            .map(|(cred_type, identifier, username, password)| {
                json!({
                    "type": cred_type.as_str(),
                    "identifier": identifier,
                    "username": username,
                    "password": password,
                })
            })
            .collect();

        let master_data = json!({
            "version": 1,
            "credentials": credentials_json,
        });

        let json_string = serde_json::to_string(&master_data).map_err(|e| {
            KeyringError::OperationFailed(format!("JSON serialization failed: {}", e))
        })?;

        // Store in keychain under a single master entry
        let entry = Entry::new("dwata-master", "all-credentials").map_err(|e| {
            KeyringError::ServiceUnavailable(format!("Failed to create master entry: {}", e))
        })?;

        entry.set_password(&json_string).map_err(|e| {
            KeyringError::OperationFailed(format!("Failed to store master credentials: {}", e))
        })?;

        tracing::info!(
            "Stored {} credentials in master keychain entry",
            credentials.len()
        );

        // Update cache with all credentials
        let mut cache = self.cache.write().await;
        for (cred_type, identifier, username, password) in credentials {
            let cache_key = Self::cache_key(&cred_type, &identifier, &username);
            cache.insert(
                cache_key,
                CachedPassword {
                    password,
                    expires_at: Instant::now() + self.cache_ttl,
                },
            );
        }

        Ok(())
    }

    /// Retrieve all credentials from master keychain entry
    pub async fn get_master_credentials(
        &self,
    ) -> Result<Vec<(CredentialType, String, String, String)>, KeyringError> {
        use serde_json::Value;

        let entry = Entry::new("dwata-master", "all-credentials").map_err(|e| {
            KeyringError::ServiceUnavailable(format!("Failed to create master entry: {}", e))
        })?;

        let json_string = entry.get_password().map_err(|e| {
            if e.to_string().contains("not found") || e.to_string().contains("NotFound") {
                KeyringError::NotFound
            } else {
                KeyringError::OperationFailed(format!(
                    "Failed to retrieve master credentials: {}",
                    e
                ))
            }
        })?;

        let master_data: Value = serde_json::from_str(&json_string)
            .map_err(|e| KeyringError::OperationFailed(format!("JSON parsing failed: {}", e)))?;

        let credentials_array = master_data["credentials"].as_array().ok_or_else(|| {
            KeyringError::OperationFailed("Invalid master credentials format".to_string())
        })?;

        let mut credentials = Vec::new();
        for cred in credentials_array {
            let cred_type_str = cred["type"].as_str().ok_or_else(|| {
                KeyringError::OperationFailed("Missing credential type".to_string())
            })?;
            let identifier = cred["identifier"]
                .as_str()
                .ok_or_else(|| KeyringError::OperationFailed("Missing identifier".to_string()))?
                .to_string();
            let username = cred["username"]
                .as_str()
                .ok_or_else(|| KeyringError::OperationFailed("Missing username".to_string()))?
                .to_string();
            let password = cred["password"]
                .as_str()
                .ok_or_else(|| KeyringError::OperationFailed("Missing password".to_string()))?
                .to_string();

            let cred_type = CredentialType::from_str(cred_type_str);
            credentials.push((cred_type, identifier, username, password));
        }

        tracing::info!(
            "Retrieved {} credentials from master keychain entry",
            credentials.len()
        );

        // Update cache
        let mut cache = self.cache.write().await;
        for (cred_type, identifier, username, password) in &credentials {
            let cache_key = Self::cache_key(cred_type, identifier, username);
            cache.insert(
                cache_key,
                CachedPassword {
                    password: password.clone(),
                    expires_at: Instant::now() + self.cache_ttl,
                },
            );
        }

        Ok(credentials)
    }

    /// Check if master credentials entry exists
    pub fn has_master_credentials() -> bool {
        if let Ok(entry) = Entry::new("dwata-master", "all-credentials") {
            entry.get_password().is_ok()
        } else {
            false
        }
    }
}
