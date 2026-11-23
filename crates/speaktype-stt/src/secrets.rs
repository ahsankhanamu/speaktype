use std::collections::HashMap;
use std::sync::Mutex;

/// Keychain service all SpeakType secrets live under.
#[cfg(feature = "keychain")]
const SERVICE: &str = "app.speaktype";

#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[cfg(feature = "keychain")]
    #[error("Keychain access failed: {0}")]
    Keyring(#[from] keyring::Error),
    /// A store supplied by the host app (the phone's Keychain or Keystore) failed.
    #[error("Secret store failed: {0}")]
    Store(String),
}

pub trait SecretStore: Send + Sync {
    fn get(&self, account: &str) -> Result<Option<String>, SecretError>;
    fn set(&self, account: &str, value: &str) -> Result<(), SecretError>;
    fn delete(&self, account: &str) -> Result<(), SecretError>;
}

/// macOS Keychain store (service `app.speaktype`, accounts like `stt.groq`).
#[cfg(feature = "keychain")]
pub struct KeychainStore;

#[cfg(feature = "keychain")]
impl SecretStore for KeychainStore {
    fn get(&self, account: &str) -> Result<Option<String>, SecretError> {
        match keyring::Entry::new(SERVICE, account)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn set(&self, account: &str, value: &str) -> Result<(), SecretError> {
        keyring::Entry::new(SERVICE, account)?.set_password(value)?;
        Ok(())
    }

    fn delete(&self, account: &str) -> Result<(), SecretError> {
        match keyring::Entry::new(SERVICE, account)?.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

/// In-memory [`SecretStore`] for tests.
#[derive(Default)]
pub struct MemoryStore {
    entries: Mutex<HashMap<String, String>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecretStore for MemoryStore {
    fn get(&self, account: &str) -> Result<Option<String>, SecretError> {
        Ok(self.entries.lock().unwrap().get(account).cloned())
    }

    fn set(&self, account: &str, value: &str) -> Result<(), SecretError> {
        self.entries
            .lock()
            .unwrap()
            .insert(account.to_string(), value.to_string());
        Ok(())
    }

    fn delete(&self, account: &str) -> Result<(), SecretError> {
        self.entries.lock().unwrap().remove(account);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_store_round_trips_set_get_and_delete() {
        let store = MemoryStore::new();
        assert_eq!(store.get("stt.groq").unwrap(), None);
        store.set("stt.groq", "sk-test").unwrap();
        assert_eq!(store.get("stt.groq").unwrap(), Some("sk-test".to_string()));
        store.set("llm.api_key", "sk-other").unwrap();
        store.delete("stt.groq").unwrap();
        assert_eq!(store.get("stt.groq").unwrap(), None);
        assert_eq!(
            store.get("llm.api_key").unwrap(),
            Some("sk-other".to_string())
        );
        // Deleting an account that is not stored succeeds.
        store.delete("stt.groq").unwrap();
    }
}
