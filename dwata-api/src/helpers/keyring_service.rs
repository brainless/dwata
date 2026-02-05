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

        // Cache miss or expired - fetch from keychain
        tracing::debug!("Cache miss for credential: {}, fetching from keychain", cache_key);
        let password = Self::fetch_from_keychain(credential_type, identifier, username)?;

        // Update cache
        let mut cache = self.cache.write().await;
        cache.insert(
            cache_key,
            CachedPassword {
                password: password.clone(),
                expires_at: Instant::now() + self.cache_ttl,
            },
        );

        Ok(password)
    }

    /// Set password in keychain and update cache
    pub async fn set_password(
        &self,
        credential_type: &CredentialType,
        identifier: &str,
        username: &str,
        password: &str,
    ) -> Result<(), KeyringError> {
        let service = credential_type.service_name();
        let keychain_user = Self::keychain_username(identifier, username);

        let entry = Entry::new(&service, &keychain_user).map_err(|e| {
            KeyringError::ServiceUnavailable(format!("Failed to create keychain entry: {}", e))
        })?;

        entry.set_password(password).map_err(|e| {
            KeyringError::OperationFailed(format!("Failed to store password: {}", e))
        })?;

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

    /// Update password in keychain and cache
    pub async fn update_password(
        &self,
        credential_type: &CredentialType,
        identifier: &str,
        username: &str,
        new_password: &str,
    ) -> Result<(), KeyringError> {
        let service = credential_type.service_name();
        let keychain_user = Self::keychain_username(identifier, username);

        tracing::info!(
            "Attempting to update password for service: {}, user: {}",
            service,
            keychain_user
        );

        let entry = Entry::new(&service, &keychain_user).map_err(|e| {
            tracing::error!("Failed to create keychain entry: {}", e);
            KeyringError::ServiceUnavailable(format!("Failed to create keychain entry: {}", e))
        })?;

        match entry.set_password(new_password) {
            Ok(_) => {
                tracing::info!("Successfully updated password for {}", keychain_user);
            }
            Err(e) => {
                let error_msg = e.to_string();
                tracing::error!(
                    "Failed to update password for {}: {}",
                    keychain_user,
                    error_msg
                );

                if error_msg.contains("already exists") || error_msg.contains("duplicate") {
                    tracing::info!("Entry already exists, attempting to delete and recreate");
                    match entry.delete_password() {
                        Ok(_) => {
                            tracing::info!("Deleted existing entry, now setting new password");
                            entry.set_password(new_password).map_err(|e| {
                                KeyringError::OperationFailed(format!(
                                    "Failed to store password after delete: {}",
                                    e
                                ))
                            })?;
                        }
                        Err(delete_err) => {
                            tracing::warn!("Failed to delete existing entry: {}, will try to set password anyway", delete_err);
                            entry.set_password(new_password).map_err(|e| {
                                KeyringError::OperationFailed(format!(
                                    "Failed to store password: {}",
                                    e
                                ))
                            })?;
                        }
                    }
                } else {
                    return Err(KeyringError::OperationFailed(format!(
                        "Failed to store password: {}",
                        e
                    )));
                }
            }
        }

        // Update cache
        let cache_key = Self::cache_key(credential_type, identifier, username);
        let mut cache = self.cache.write().await;
        cache.insert(
            cache_key,
            CachedPassword {
                password: new_password.to_string(),
                expires_at: Instant::now() + self.cache_ttl,
            },
        );

        Ok(())
    }

    /// Delete password from keychain and remove from cache
    pub async fn delete_password(
        &self,
        credential_type: &CredentialType,
        identifier: &str,
        username: &str,
    ) -> Result<(), KeyringError> {
        let service = credential_type.service_name();
        let keychain_user = Self::keychain_username(identifier, username);

        let entry = Entry::new(&service, &keychain_user).map_err(|e| {
            KeyringError::ServiceUnavailable(format!("Failed to create keychain entry: {}", e))
        })?;

        entry.delete_password().map_err(|e| {
            if e.to_string().contains("not found") || e.to_string().contains("NotFound") {
                KeyringError::NotFound
            } else {
                KeyringError::OperationFailed(format!("Failed to delete password: {}", e))
            }
        })?;

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
    pub async fn invalidate(&self, credential_type: &CredentialType, identifier: &str, username: &str) {
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
}
