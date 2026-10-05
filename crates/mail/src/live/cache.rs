use std::{
    fmt::Write as _,
    path::{Path, PathBuf},
};

use crate::model::{MailMessage, MailWorkspace, Mailbox};
use serde::{Deserialize, Serialize};

use crate::live::{sanitize_cached_mail_summary_messages, MailLiveConfig};

pub const MAIL_CACHE_DIR_ENV: &str = "NOTSUPERHUMAN_MAIL_CACHE_DIR";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct MailboxesCache {
    account_id: String,
    mailbox_email: Option<String>,
    display_name: String,
    owner_username: String,
    selected_mailbox_id: String,
    mailbox_state: String,
    mailboxes: Vec<Mailbox>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct MailboxSummaryCache {
    account_id: String,
    mailbox_id: String,
    query_state: String,
    messages: Vec<MailMessage>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MailCachePaths {
    account_dir: PathBuf,
}

impl MailCachePaths {
    fn new(cache_root: &Path, cache_key: &str) -> Self {
        Self {
            account_dir: local_cache::account_cache_dir(cache_root, cache_key),
        }
    }

    fn mailboxes_path(&self) -> PathBuf {
        self.account_dir.join("mailboxes.bin")
    }

    fn mailbox_summary_path(&self, mailbox_id: &str) -> PathBuf {
        self.account_dir.join(format!(
            "mailbox-{}.bin",
            local_cache::cache_key_hash(mailbox_id)
        ))
    }
}

#[derive(Clone)]
pub struct MailWorkspaceCache {
    paths: MailCachePaths,
    key: [u8; 32],
    account_id: String,
}

impl MailWorkspaceCache {
    pub fn from_config(
        config: &MailLiveConfig,
        identity_subject: &str,
        account_id: &str,
    ) -> Result<Self, String> {
        validate_cache_scope("account identity", identity_subject)?;
        validate_cache_scope("JMAP mail account ID", account_id)?;
        let cache_key = mail_cache_key(config, identity_subject, account_id);
        let key = local_cache::load_or_create_cache_key(&cache_key, "mail")?;
        let cache_root = default_mail_cache_root_dir()?;
        Ok(Self {
            paths: MailCachePaths::new(&cache_root, &cache_key),
            key,
            account_id: account_id.to_string(),
        })
    }

    pub fn load_cached_workspace(
        &self,
        selected_mailbox_id: Option<&str>,
    ) -> Result<Option<MailWorkspace>, String> {
        let Some(mailboxes_cache) = local_cache::read_encrypted_json::<MailboxesCache>(
            &self.paths.mailboxes_path(),
            &self.key,
        )?
        else {
            return Ok(None);
        };
        if mailboxes_cache.account_id != self.account_id {
            return Ok(None);
        }
        let mailbox_id =
            selected_mailbox_id.unwrap_or(mailboxes_cache.selected_mailbox_id.as_str());
        let Some(summary_cache) = local_cache::read_encrypted_json::<MailboxSummaryCache>(
            &self.paths.mailbox_summary_path(mailbox_id),
            &self.key,
        )?
        else {
            return Ok(None);
        };
        if summary_cache.account_id != self.account_id
            || summary_cache.mailbox_id != mailbox_id
            || !mailbox_exists(&mailboxes_cache, mailbox_id)
        {
            return Ok(None);
        }
        let messages = sanitize_cached_mail_summary_messages(summary_cache.messages);
        Ok(Some(MailWorkspace {
            account_id: self.account_id.clone(),
            mailbox_email: mailboxes_cache.mailbox_email,
            display_name: mailboxes_cache.display_name,
            owner_username: mailboxes_cache.owner_username,
            selected_mailbox_id: mailbox_id.to_string(),
            mailboxes: mailboxes_cache.mailboxes,
            messages,
            message_next_position: None,
        }))
    }

    pub fn cached_mailbox_query_state(&self, mailbox_id: &str) -> Result<Option<String>, String> {
        local_cache::read_encrypted_json::<MailboxSummaryCache>(
            &self.paths.mailbox_summary_path(mailbox_id),
            &self.key,
        )
        .map(|summary| {
            summary.and_then(|summary| {
                (summary.account_id == self.account_id && summary.mailbox_id == mailbox_id)
                    .then_some(summary.query_state)
            })
        })
    }

    pub fn persist_workspace(
        &self,
        workspace: &MailWorkspace,
        mailbox_state: String,
        query_state: String,
    ) {
        if workspace.account_id != self.account_id {
            eprintln!(
                "notsuperhuman mail cache rejected workspace for account {}; expected {}",
                workspace.account_id, self.account_id
            );
            return;
        }
        let mailboxes_cache = MailboxesCache {
            account_id: self.account_id.clone(),
            mailbox_email: workspace.mailbox_email.clone(),
            display_name: workspace.display_name.clone(),
            owner_username: workspace.owner_username.clone(),
            selected_mailbox_id: workspace.selected_mailbox_id.clone(),
            mailbox_state,
            mailboxes: workspace.mailboxes.clone(),
        };
        if let Err(error) = local_cache::write_encrypted_json(
            &self.paths.mailboxes_path(),
            &self.key,
            &mailboxes_cache,
        ) {
            eprintln!("notsuperhuman mail cache write failed: {error}");
            return;
        }
        let summary_cache = MailboxSummaryCache {
            account_id: self.account_id.clone(),
            mailbox_id: workspace.selected_mailbox_id.clone(),
            query_state,
            messages: sanitize_cached_mail_summary_messages(workspace.messages.clone()),
        };
        if let Err(error) = local_cache::write_encrypted_json(
            &self
                .paths
                .mailbox_summary_path(workspace.selected_mailbox_id.as_str()),
            &self.key,
            &summary_cache,
        ) {
            eprintln!("notsuperhuman mail cache write failed: {error}");
        }
    }
}

pub fn default_mail_cache_root_dir() -> Result<PathBuf, String> {
    local_cache::default_cache_root_dir(MAIL_CACHE_DIR_ENV, "mail", "mail")
}

fn mail_cache_key(config: &MailLiveConfig, identity_subject: &str, account_id: &str) -> String {
    framed_mail_scope_key(
        "mail-v3",
        &[identity_subject, account_id, config.server.as_str()],
    )
}

pub(crate) fn framed_mail_scope_key(prefix: &str, values: &[&str]) -> String {
    let mut key = prefix.to_string();
    for value in values {
        write!(key, "|{}:{value}", value.len()).expect("writing a String cannot fail");
    }
    key
}

pub(crate) fn validate_cache_scope(label: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.trim() != value {
        return Err(format!(
            "{label} must be nonempty without surrounding whitespace"
        ));
    }
    Ok(())
}

fn mailbox_exists(mailboxes_cache: &MailboxesCache, mailbox_id: &str) -> bool {
    mailboxes_cache
        .mailboxes
        .iter()
        .any(|mailbox| mailbox.id == mailbox_id)
}
