//! Startup and sign-in. Every saved account is opened into one catalog; the
//! last account used is loaded first and the others load when the account
//! palette switches to them.

use std::sync::Arc;

use reqwest::Url;

use crate::live::{
    accounts::{self, SavedAccount},
    client::{JmapCredentials, MailLiveCatalog},
    gmail::GmailWorkspace,
    workspace_api, MailWorkspaceRuntime,
};
use crate::model::{
    MailAccountInfo, MailAccountLoadError, MailAccountWorkspace, MailSignInRequest,
    MailWorkspaceBootstrap,
};

#[derive(Clone)]
struct ProductionMailBootstrapApi;

pub fn production_mail_bootstrap_api() -> Arc<dyn crate::model::MailBootstrapApi> {
    Arc::new(ProductionMailBootstrapApi)
}

impl crate::model::MailBootstrapApi for ProductionMailBootstrapApi {
    fn has_mail_accounts(&self) -> bool {
        accounts::saved_accounts().is_ok_and(|saved| !saved.is_empty())
    }

    fn bootstrap_workspace(&self) -> Result<MailWorkspaceBootstrap, String> {
        let catalog = Arc::new(AccountCatalog::open()?);
        let account_id = accounts::active_account()
            .filter(|id| catalog.contains(id))
            .unwrap_or_else(|| catalog.first_account_id().to_string());
        let loaded = catalog
            .load(account_id.as_str())
            .map_err(|error| match error {
                MailAccountLoadError::AccessRemoved { account_id } => {
                    format!("mail account {account_id} is no longer available")
                }
                MailAccountLoadError::Failed(error) => error,
            })?;
        Ok(MailWorkspaceBootstrap::new(
            loaded.accounts,
            loaded.workspace,
            loaded.workspace_api,
            loaded.needs_initial_refresh,
            catalog,
        ))
    }

    fn sign_in(&self, request: MailSignInRequest) -> Result<(), String> {
        let account_id = match request {
            MailSignInRequest::Superhuman => super::superhuman::sign_in()?,
            MailSignInRequest::Jmap { server, token } => sign_in_with_jmap(&server, &token)?,
        };
        accounts::remember_active_account(account_id.as_str());
        Ok(())
    }
}

/// Checks the server and token by opening the JMAP session, then saves them.
fn sign_in_with_jmap(server: &str, token: &str) -> Result<String, String> {
    let token = token.trim();
    if token.is_empty() {
        return Err("Enter the API token from your mail provider".to_string());
    }
    let session_url = jmap_session_url(server)?;
    let catalog = MailLiveCatalog::discover(JmapCredentials {
        session_url,
        token: token.to_string(),
    })?;
    let account = SavedAccount::Jmap {
        session_url: catalog.session_url().to_string(),
        username: catalog.owner_username().to_string(),
    };
    accounts::save_account(&account, token)?;
    Ok(catalog.initial_account_id().to_string())
}

/// Accepts what people paste: a provider name, a host, or the full session
/// URL. Hosts without a path use JMAP's well-known discovery address.
fn jmap_session_url(server: &str) -> Result<Url, String> {
    let server = server.trim().trim_end_matches('/');
    if server.is_empty() {
        return Err("Enter your JMAP server, such as api.fastmail.com".to_string());
    }
    let with_scheme = if server.contains("://") {
        server.to_string()
    } else {
        format!("https://{server}")
    };
    let mut url = Url::parse(&with_scheme)
        .map_err(|error| format!("{server} is not a server address: {error}"))?;
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    if matches!(
        host.as_str(),
        "fastmail.com" | "www.fastmail.com" | "app.fastmail.com" | "api.fastmail.com"
    ) {
        return Url::parse("https://api.fastmail.com/jmap/session")
            .map_err(|error| error.to_string());
    }
    if url.path().is_empty() || url.path() == "/" {
        url.set_path("/.well-known/jmap");
    }
    Ok(url)
}

/// Every saved account, opened. A JMAP sign-in can hold several accounts
/// (shared mailboxes); a Gmail sign-in holds one.
struct AccountCatalog {
    entries: Vec<CatalogEntry>,
}

enum CatalogEntry {
    Jmap(MailLiveCatalog),
    Gmail(GmailWorkspace),
}

impl CatalogEntry {
    fn accounts(&self) -> Vec<MailAccountInfo> {
        match self {
            Self::Jmap(catalog) => catalog.accounts().to_vec(),
            Self::Gmail(workspace) => vec![workspace.account().clone()],
        }
    }
}

impl AccountCatalog {
    fn open() -> Result<Self, String> {
        let saved = accounts::saved_accounts()?;
        if saved.is_empty() {
            return Err("Sign in to a mail account to get started".to_string());
        }
        let mut entries = Vec::new();
        let mut first_error = None;
        for (index, account) in saved.iter().enumerate() {
            match open_account(account, index == 0) {
                Ok(entry) => entries.push(entry),
                Err(error) => {
                    eprintln!("mail account {} unavailable: {error}", account.label());
                    first_error.get_or_insert(error);
                }
            }
        }
        if entries.is_empty() {
            return Err(
                first_error.unwrap_or_else(|| "no mail account could be opened".to_string())
            );
        }
        Ok(Self { entries })
    }

    fn all_accounts(&self) -> Vec<MailAccountInfo> {
        let mut seen = std::collections::HashSet::new();
        self.entries
            .iter()
            .flat_map(CatalogEntry::accounts)
            .filter(|account| seen.insert(account.id.clone()))
            .collect()
    }

    fn contains(&self, account_id: &str) -> bool {
        self.entry_for(account_id).is_some()
    }

    fn first_account_id(&self) -> &str {
        match &self.entries[0] {
            CatalogEntry::Jmap(catalog) => catalog.initial_account_id(),
            CatalogEntry::Gmail(workspace) => workspace.account().id.as_str(),
        }
    }

    fn entry_for(&self, account_id: &str) -> Option<&CatalogEntry> {
        self.entries.iter().find(|entry| {
            entry
                .accounts()
                .iter()
                .any(|account| account.id == account_id)
        })
    }

    fn load(&self, account_id: &str) -> Result<MailAccountWorkspace, MailAccountLoadError> {
        let entry =
            self.entry_for(account_id)
                .ok_or_else(|| MailAccountLoadError::AccessRemoved {
                    account_id: account_id.to_string(),
                })?;
        let (workspace, workspace_api, needs_initial_refresh) = match entry {
            CatalogEntry::Jmap(catalog) => {
                let runtime = MailWorkspaceRuntime::for_account(catalog, account_id)
                    .map_err(MailAccountLoadError::Failed)?;
                let (workspace, needs_refresh) = runtime
                    .load_startup_workspace()
                    .map_err(MailAccountLoadError::Failed)?;
                (workspace, workspace_api(runtime), needs_refresh)
            }
            CatalogEntry::Gmail(gmail) => {
                let (workspace, needs_refresh) = gmail
                    .load_startup_workspace()
                    .map_err(MailAccountLoadError::Failed)?;
                let api: Arc<dyn crate::model::MailWorkspaceApi> = Arc::new(gmail.clone());
                (workspace, api, needs_refresh)
            }
        };
        accounts::remember_active_account(account_id);
        Ok(MailAccountWorkspace {
            accounts: self.all_accounts(),
            workspace,
            workspace_api,
            needs_initial_refresh,
        })
    }
}

impl crate::model::MailAccountLoaderApi for AccountCatalog {
    fn load_mail_account(
        &self,
        account_id: &str,
    ) -> Result<MailAccountWorkspace, MailAccountLoadError> {
        self.load(account_id)
    }
}

fn open_account(account: &SavedAccount, is_first: bool) -> Result<CatalogEntry, String> {
    match account {
        SavedAccount::Jmap { session_url, .. } => {
            let session_url = Url::parse(session_url)
                .map_err(|error| format!("invalid saved JMAP server {session_url}: {error}"))?;
            MailLiveCatalog::discover(JmapCredentials {
                session_url,
                token: accounts::account_secret(account)?,
            })
            .map(CatalogEntry::Jmap)
        }
        SavedAccount::Superhuman { .. } => {
            GmailWorkspace::open(account.clone(), is_first).map(CatalogEntry::Gmail)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::jmap_session_url;

    #[test]
    fn jmap_servers_accept_hosts_providers_and_full_urls() {
        assert_eq!(
            jmap_session_url("fastmail.com").expect("fastmail").as_str(),
            "https://api.fastmail.com/jmap/session"
        );
        assert_eq!(
            jmap_session_url("mail.example.com").expect("host").as_str(),
            "https://mail.example.com/.well-known/jmap"
        );
        assert_eq!(
            jmap_session_url("https://mail.example.com/jmap/session")
                .expect("url")
                .as_str(),
            "https://mail.example.com/jmap/session"
        );
        assert!(jmap_session_url("  ").is_err());
    }
}
