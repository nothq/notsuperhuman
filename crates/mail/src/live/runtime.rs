use std::sync::Arc;

use crate::model::MailWorkspace;

use crate::live::{
    client::MailLiveCatalog, MailLiveClient, MailLiveConfig, MailSplitPreferencesStore,
    MailWorkspaceCache,
};

mod workspace_api;

/// One JMAP mail account, bound to its server, cache and split preferences.
#[derive(Clone)]
pub struct MailWorkspaceRuntime {
    pub(crate) client: MailLiveClient,
    pub(crate) cache: Option<MailWorkspaceCache>,
    pub(crate) split_preferences: MailSplitPreferencesStore,
}

impl MailWorkspaceRuntime {
    pub(crate) fn for_account(catalog: &MailLiveCatalog, account_id: &str) -> Result<Self, String> {
        let config = MailLiveConfig {
            server: catalog.session_url().to_string(),
        };
        let identity_subject = catalog.owner_username();
        let client = catalog.bind(account_id)?;
        let split_preferences = MailSplitPreferencesStore::open(
            &config,
            &client.context().api_url,
            identity_subject,
            account_id,
        )?;
        let cache = match MailWorkspaceCache::from_config(&config, identity_subject, account_id) {
            Ok(cache) => Some(cache),
            Err(error) => {
                eprintln!("mail cache unavailable: {error}");
                None
            }
        };
        Ok(Self {
            client,
            cache,
            split_preferences,
        })
    }

    pub fn load_startup_workspace(&self) -> Result<(MailWorkspace, bool), String> {
        if let Some(workspace) = self.cached_workspace(None)? {
            return Ok((workspace, true));
        }
        self.load_workspace().map(|workspace| (workspace, false))
    }

    pub(crate) fn load_workspace(&self) -> Result<MailWorkspace, String> {
        self.refresh_workspace_summary(None)
    }

    pub(crate) fn cached_workspace(
        &self,
        selected_mailbox_id: Option<&str>,
    ) -> Result<Option<MailWorkspace>, String> {
        let Some(cache) = self.cache.as_ref() else {
            return Ok(None);
        };
        cache.load_cached_workspace(selected_mailbox_id)
    }

    pub(crate) fn refresh_workspace_summary(
        &self,
        selected_mailbox_id: Option<&str>,
    ) -> Result<MailWorkspace, String> {
        let cached_summary = selected_mailbox_id
            .map(|mailbox_id| self.cached_workspace(Some(mailbox_id)))
            .transpose()?
            .flatten();
        let cached_messages = cached_summary
            .as_ref()
            .map(|workspace| workspace.messages.as_slice());
        let cached_query_state = selected_mailbox_id
            .map(|mailbox_id| {
                self.cache
                    .as_ref()
                    .map(|cache| cache.cached_mailbox_query_state(mailbox_id))
                    .unwrap_or(Ok(None))
            })
            .transpose()?
            .flatten();
        let load = self.client.load_workspace_summary(
            selected_mailbox_id,
            cached_query_state.as_deref(),
            cached_messages,
        )?;
        if let Some(cache) = self.cache.as_ref() {
            cache.persist_workspace(&load.workspace, load.mailbox_state, load.query_state);
        }
        Ok(load.workspace)
    }
}

pub fn workspace_api(runtime: MailWorkspaceRuntime) -> Arc<dyn crate::model::MailWorkspaceApi> {
    Arc::new(runtime)
}
