use keyring::Entry;
use shared_types::credential::CredentialType;

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

pub struct KeyringService;

impl KeyringService {
    fn keychain_username(identifier: &str, username: &str) -> String {
        format!("{}:{}", identifier, username)
    }

    pub fn set_password(
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

        Ok(())
    }

    pub fn get_password(
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

    pub fn delete_password(
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

        Ok(())
    }

    pub fn update_password(
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
                Ok(())
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
                            Ok(())
                        }
                        Err(delete_err) => {
                            tracing::warn!("Failed to delete existing entry: {}, will try to set password anyway", delete_err);
                            entry.set_password(new_password).map_err(|e| {
                                KeyringError::OperationFailed(format!(
                                    "Failed to store password: {}",
                                    e
                                ))
                            })?;
                            Ok(())
                        }
                    }
                } else {
                    Err(KeyringError::OperationFailed(format!(
                        "Failed to store password: {}",
                        e
                    )))
                }
            }
        }
    }
}
