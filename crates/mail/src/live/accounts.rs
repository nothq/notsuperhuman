//! The mail accounts this app is signed in to. The list itself lives in the
//! app data directory; each account's token lives in the private credential
//! file next to the other secrets.

use std::{fs, path::PathBuf};

use secret_store::SecretStore;
use serde::{Deserialize, Serialize};

const ACCOUNTS_FILE: &str = "accounts.json";
const ACTIVE_ACCOUNT_FILE: &str = "active-account";

/// One sign-in: a JMAP server opened with an API token, or a Gmail account
/// opened through a Superhuman session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub(crate) enum SavedAccount {
    Jmap {
        session_url: String,
        username: String,
    },
    Superhuman {
        email: String,
        google_id: String,
    },
}

impl SavedAccount {
    /// The credential-file key holding this account's token.
    pub(crate) fn secret_key(&self) -> String {
        match self {
            Self::Jmap {
                session_url,
                username,
            } => format!("mail:jmap:{username}@{session_url}"),
            Self::Superhuman { email, .. } => {
                format!("mail:superhuman:{}", email.to_ascii_lowercase())
            }
        }
    }

    pub(crate) fn label(&self) -> &str {
        match self {
            Self::Jmap { username, .. } => username,
            Self::Superhuman { email, .. } => email,
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
struct AccountsFile {
    accounts: Vec<SavedAccount>,
}

pub(crate) fn saved_accounts() -> Result<Vec<SavedAccount>, String> {
    let path = accounts_path()?;
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("failed to read {}: {error}", path.display())),
    };
    serde_json::from_str::<AccountsFile>(&contents)
        .map(|file| file.accounts)
        .map_err(|error| format!("failed to decode {}: {error}", path.display()))
}

/// Stores the account's token and adds it to the list, or refreshes the token
/// of an account that is already there.
pub(crate) fn save_account(account: &SavedAccount, secret: &str) -> Result<(), String> {
    update_account_secret(account, secret)?;
    let mut accounts = saved_accounts()?;
    if !accounts.contains(account) {
        accounts.push(account.clone());
    }
    write_accounts(accounts)
}

pub(crate) fn update_account_secret(account: &SavedAccount, secret: &str) -> Result<(), String> {
    secret_store()?
        .upsert_secret(account.secret_key().as_str(), secret)
        .map_err(|error| format!("failed to store the {} token: {error}", account.label()))
}

pub(crate) fn account_secret(account: &SavedAccount) -> Result<String, String> {
    secret_store()?
        .read_secret(account.secret_key().as_str())
        .map_err(|error| format!("failed to read the {} token: {error}", account.label()))?
        .ok_or_else(|| format!("{} is signed out; sign in again", account.label()))
}

/// The account the app showed last, so the next launch opens on it.
pub(crate) fn active_account() -> Option<String> {
    let path = app_model::app_data_dir().ok()?.join(ACTIVE_ACCOUNT_FILE);
    let id = fs::read_to_string(path).ok()?;
    let id = id.trim();
    (!id.is_empty()).then(|| id.to_string())
}

pub(crate) fn remember_active_account(account_id: &str) {
    let Ok(dir) = app_model::app_data_dir() else {
        return;
    };
    if fs::create_dir_all(&dir).is_ok() {
        let _ = fs::write(dir.join(ACTIVE_ACCOUNT_FILE), account_id);
    }
}

fn write_accounts(accounts: Vec<SavedAccount>) -> Result<(), String> {
    let path = accounts_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let contents = serde_json::to_string_pretty(&AccountsFile { accounts })
        .map_err(|error| format!("failed to encode the account list: {error}"))?;
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, contents)
        .map_err(|error| format!("failed to write {}: {error}", temp_path.display()))?;
    fs::rename(&temp_path, &path)
        .map_err(|error| format!("failed to replace {}: {error}", path.display()))
}

fn accounts_path() -> Result<PathBuf, String> {
    Ok(app_model::app_data_dir()?.join(ACCOUNTS_FILE))
}

fn secret_store() -> Result<SecretStore, String> {
    SecretStore::notsuperhuman().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::SavedAccount;

    #[test]
    fn saved_accounts_round_trip_with_a_kind_tag() {
        let accounts = vec![
            SavedAccount::Jmap {
                session_url: "https://mail.example.com/jmap/session".to_string(),
                username: "ada@example.com".to_string(),
            },
            SavedAccount::Superhuman {
                email: "ada@gmail.com".to_string(),
                google_id: "1234".to_string(),
            },
        ];
        let encoded = serde_json::to_string(&accounts).expect("encode");
        assert!(encoded.contains("\"kind\":\"jmap\""));
        assert!(encoded.contains("\"kind\":\"superhuman\""));
        let decoded: Vec<SavedAccount> = serde_json::from_str(&encoded).expect("decode");
        assert_eq!(decoded, accounts);
    }

    #[test]
    fn superhuman_secret_keys_ignore_address_case() {
        let upper = SavedAccount::Superhuman {
            email: "Ada@Gmail.com".to_string(),
            google_id: "1234".to_string(),
        };
        let lower = SavedAccount::Superhuman {
            email: "ada@gmail.com".to_string(),
            google_id: "1234".to_string(),
        };
        assert_eq!(upper.secret_key(), lower.secret_key());
    }
}
