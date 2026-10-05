use std::{collections::HashMap, fs, path::Path};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    create_credentials_dir, credentials_temp_path, set_credentials_file_permissions,
    write_credentials_file, CREDENTIAL_FILE_VERSION,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CredentialFile {
    version: u32,
    #[serde(default)]
    pub(crate) secrets: HashMap<String, String>,
    #[serde(default, alias = "mail_cache_keys")]
    pub(crate) cache_keys: HashMap<String, String>,
}

impl Default for CredentialFile {
    fn default() -> Self {
        Self {
            version: CREDENTIAL_FILE_VERSION,
            secrets: HashMap::new(),
            cache_keys: HashMap::new(),
        }
    }
}

impl CredentialFile {
    pub(crate) fn load(path: &Path) -> Result<Self> {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "failed to read notsuperhuman credentials {}",
                        path.display()
                    )
                });
            }
        };
        let credentials = serde_json::from_str::<Self>(&contents).with_context(|| {
            format!(
                "failed to decode notsuperhuman credentials {}",
                path.display()
            )
        })?;
        if credentials.version != CREDENTIAL_FILE_VERSION {
            return Err(anyhow!(
                "unsupported notsuperhuman credentials version {} in {}; expected {}",
                credentials.version,
                path.display(),
                CREDENTIAL_FILE_VERSION
            ));
        }
        Ok(credentials)
    }

    pub(crate) fn save(&self, path: &Path) -> Result<()> {
        let parent = path.parent().ok_or_else(|| {
            anyhow!(
                "notsuperhuman credentials path has no parent directory: {}",
                path.display()
            )
        })?;
        create_credentials_dir(parent)?;
        let contents = serde_json::to_string_pretty(self)
            .context("failed to encode notsuperhuman credentials")?;
        let temp_path = credentials_temp_path(path);
        write_credentials_file(&temp_path, contents.as_bytes())?;
        fs::rename(&temp_path, path).with_context(|| {
            format!(
                "failed to replace notsuperhuman credentials {} with {}",
                path.display(),
                temp_path.display()
            )
        })?;
        set_credentials_file_permissions(path)?;
        Ok(())
    }
}
