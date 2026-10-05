use std::path::PathBuf;

use anyhow::{anyhow, Result};

const AUTH_FILE_NAME: &str = "auth.json";
const CREDENTIAL_FILE_VERSION: u32 = 1;

mod credential_file;
mod filesystem;
mod secrets;

use credential_file::CredentialFile;
use filesystem::*;

pub struct SecretStore {
    auth_path: PathBuf,
}

impl SecretStore {
    pub fn notsuperhuman() -> Result<Self> {
        Ok(Self {
            auth_path: default_auth_path()?,
        })
    }

    pub fn read_cache_key(&self, cache_key: &str) -> Result<Option<String>> {
        validate_cache_key(cache_key)?;
        self.load_credentials()
            .map(|credentials| credentials.cache_keys.get(cache_key).cloned())
    }

    pub fn upsert_cache_key(&self, cache_key: &str, key: &str) -> Result<()> {
        validate_cache_key(cache_key)?;
        let key = non_empty_value(key, "cache key")?;
        self.update_credentials(|credentials| {
            credentials
                .cache_keys
                .insert(cache_key.to_string(), key.to_string());
        })
    }

    fn load_credentials(&self) -> Result<CredentialFile> {
        let _guard = credential_file_lock()
            .lock()
            .map_err(|error| anyhow!("notsuperhuman credential file lock poisoned: {error}"))?;
        CredentialFile::load(&self.auth_path)
    }

    fn update_credentials(&self, update: impl FnOnce(&mut CredentialFile)) -> Result<()> {
        let _guard = credential_file_lock()
            .lock()
            .map_err(|error| anyhow!("notsuperhuman credential file lock poisoned: {error}"))?;
        let mut credentials = CredentialFile::load(&self.auth_path)?;
        update(&mut credentials);
        credentials.save(&self.auth_path)
    }

    #[cfg(test)]
    fn for_test(auth_path: impl Into<PathBuf>) -> Self {
        Self {
            auth_path: auth_path.into(),
        }
    }
}

#[cfg(test)]
mod tests;
