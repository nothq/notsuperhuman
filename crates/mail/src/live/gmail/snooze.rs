//! Gmail keeps its own snoozes out of the API, so this app keeps them: a
//! snoozed thread leaves the inbox, and comes back unread once its time is up
//! and the inbox is next loaded.

use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Snooze {
    pub(crate) thread_id: String,
    pub(crate) remind_at: String,
}

#[derive(Clone)]
pub(crate) struct SnoozeStore {
    path: PathBuf,
}

impl SnoozeStore {
    pub(crate) fn for_account(email: &str) -> Result<Self, String> {
        let file_name = format!("{}.json", local_cache::cache_key_hash(email));
        Ok(Self {
            path: app_model::app_data_dir()?
                .join("mail")
                .join("gmail-snoozes")
                .join(file_name),
        })
    }

    pub(crate) fn all(&self) -> Vec<Snooze> {
        fs::read_to_string(&self.path)
            .ok()
            .and_then(|contents| serde_json::from_str(&contents).ok())
            .unwrap_or_default()
    }

    pub(crate) fn add(&self, thread_id: &str, remind_at: &str) -> Result<(), String> {
        let mut snoozes = self.all();
        snoozes.retain(|snooze| snooze.thread_id != thread_id);
        snoozes.push(Snooze {
            thread_id: thread_id.to_string(),
            remind_at: remind_at.to_string(),
        });
        self.write(&snoozes)
    }

    pub(crate) fn remove(&self, thread_ids: &[String]) -> Result<(), String> {
        let mut snoozes = self.all();
        let before = snoozes.len();
        snoozes.retain(|snooze| !thread_ids.contains(&snooze.thread_id));
        if snoozes.len() == before {
            return Ok(());
        }
        self.write(&snoozes)
    }

    /// Snoozes whose time has come, as of `now`.
    pub(crate) fn due(&self, now: OffsetDateTime) -> Vec<Snooze> {
        self.all()
            .into_iter()
            .filter(|snooze| {
                OffsetDateTime::parse(snooze.remind_at.as_str(), &Rfc3339)
                    .map_or(true, |remind_at| remind_at <= now)
            })
            .collect()
    }

    fn write(&self, snoozes: &[Snooze]) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
        let contents = serde_json::to_string_pretty(snoozes)
            .map_err(|error| format!("failed to encode snoozes: {error}"))?;
        fs::write(&self.path, contents)
            .map_err(|error| format!("failed to write {}: {error}", self.path.display()))
    }
}
