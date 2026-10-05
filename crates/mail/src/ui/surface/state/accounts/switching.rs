use super::super::{Context, MailComposeMode, MailListSource, MailStartup, SurfaceState};
use crate::model::{MailAccountLoadError, MailAccountWorkspace, MailWorkspaceBootstrap};

struct MailAccountLoadRequest {
    bootstrap: MailWorkspaceBootstrap,
    account_id: String,
    selected_mailbox_id: Option<String>,
}

impl SurfaceState {
    pub(crate) fn switch_mail_account(
        &mut self,
        account_id: String,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.mail_account_switch_allowed(account_id.as_str(), cx) {
            return true;
        }
        let Some(bootstrap) = self.mail_account_switch_bootstrap(cx) else {
            return true;
        };
        self.start_mail_account_switch(account_id, bootstrap, cx);
        true
    }

    fn mail_account_switch_allowed(&mut self, account_id: &str, cx: &mut Context<Self>) -> bool {
        if self.mail_switching_account_id.is_some() {
            return false;
        }
        if !self
            .mail_account_catalog()
            .iter()
            .any(|account| account.id == account_id)
        {
            self.set_mail_account_palette_error("That shared account is no longer available", cx);
            return false;
        }
        if self
            .mail_workspace()
            .is_some_and(|workspace| workspace.account_id == account_id)
        {
            self.close_mail_account_palette(cx);
            return false;
        }
        if self.mail_account_switch_blocked() {
            self.set_mail_account_palette_error(
                "Finish or discard the current draft before switching accounts",
                cx,
            );
            return false;
        }
        true
    }

    fn mail_account_switch_bootstrap(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Option<MailWorkspaceBootstrap> {
        let MailStartup::Ready(bootstrap) = &self.mail_startup else {
            self.set_mail_account_palette_error("Account switching is unavailable", cx);
            return None;
        };
        Some(bootstrap.as_ref().clone())
    }

    fn start_mail_account_switch(
        &mut self,
        account_id: String,
        bootstrap: MailWorkspaceBootstrap,
        cx: &mut Context<Self>,
    ) {
        self.capture_current_mail_account_view();
        let selected_mailbox_id = self
            .mail_account_view_states
            .get(account_id.as_str())
            .map(|view| view.selected_mailbox_id.clone());
        self.mail_account_generation = self.mail_account_generation.wrapping_add(1);
        self.mail_switching_account_id = Some(account_id.clone());
        self.invalidate_mail_account_async_state();
        if let Some(state) = self.mail_account_palette.as_mut() {
            state.error = None;
        }
        cx.notify();
        let request = MailAccountLoadRequest {
            bootstrap,
            account_id: account_id.clone(),
            selected_mailbox_id,
        };
        self.spawn_background_task(request, cx, load_mail_account, move |this, result, cx| {
            this.apply_mail_account_load_result(account_id, result, cx);
        });
    }

    fn apply_mail_account_load_result(
        &mut self,
        account_id: String,
        result: Result<MailAccountWorkspace, MailAccountLoadError>,
        cx: &mut Context<Self>,
    ) {
        if self.mail_switching_account_id.as_deref() != Some(account_id.as_str()) {
            return;
        }
        match result {
            Ok(account_workspace) => {
                self.apply_loaded_mail_account(account_id, account_workspace, cx)
            }
            Err(MailAccountLoadError::AccessRemoved { account_id }) => {
                self.apply_removed_mail_account(account_id, cx);
            }
            Err(MailAccountLoadError::Failed(error)) => {
                self.mail_switching_account_id = None;
                self.set_mail_account_palette_error(
                    format!("Could not open this account: {error}"),
                    cx,
                );
            }
        }
    }

    fn apply_removed_mail_account(&mut self, account_id: String, cx: &mut Context<Self>) {
        if let MailStartup::Ready(bootstrap) = &mut self.mail_startup {
            bootstrap
                .accounts
                .retain(|account| account.id != account_id);
        }
        self.mail_account_view_states.remove(account_id.as_str());
        self.mail_switching_account_id = None;
        self.set_mail_account_palette_error(
            "Access removed. This shared account is no longer available",
            cx,
        );
    }

    pub(super) fn mail_account_switch_blocked(&self) -> bool {
        self.mail_compose_mode != MailComposeMode::Closed
            || self.mail_compose_pending
            || self.mail_compose_draft_id.is_some()
            || self.mail_compose_autosave_scheduled_generation.is_some()
            || self.mail_compose_autosave_in_flight.is_some()
            || self.mail_compose_autosave_queued.is_some()
            || self.mail_compose_send_after_autosave.is_some()
    }

    pub(super) fn apply_loaded_mail_account(
        &mut self,
        account_id: String,
        account_workspace: MailAccountWorkspace,
        cx: &mut Context<Self>,
    ) {
        let MailAccountWorkspace {
            accounts,
            workspace,
            workspace_api,
            needs_initial_refresh,
        } = account_workspace;
        assert_eq!(
            workspace.account_id, account_id,
            "loaded Mail workspace account must match requested account"
        );
        let (split_preferences_supported, split_definitions) =
            match load_mail_account_split_preferences(workspace_api.as_ref()) {
                Ok(preferences) => preferences,
                Err(error) => {
                    self.mail_switching_account_id = None;
                    self.set_mail_account_palette_error(error, cx);
                    return;
                }
            };
        let MailStartup::Ready(bootstrap) = &mut self.mail_startup else {
            return;
        };
        bootstrap.accounts = accounts;
        bootstrap.workspace = workspace;
        bootstrap.workspace_api = workspace_api;
        bootstrap.needs_initial_refresh = needs_initial_refresh;
        self.mail_switching_account_id = None;
        self.mail_account_recovery_in_flight = false;
        self.mail_account_palette = None;
        self.reset_mail_account_surface_state();
        self.mail_split_preferences_supported = split_preferences_supported;
        self.mail_split_definitions = split_definitions;
        self.restore_mail_account_view(account_id.as_str());
        cx.notify();
        self.resume_loaded_mail_account_view(needs_initial_refresh, cx);
    }

    fn resume_loaded_mail_account_view(
        &mut self,
        needs_initial_refresh: bool,
        cx: &mut Context<Self>,
    ) {
        if self.mail_starred_view_open() {
            self.refresh_current_mail_search_results(cx);
        } else if matches!(
            &self.mail_list_source,
            MailListSource::Split(_) | MailListSource::Other
        ) {
            self.open_mail_split_view(self.mail_list_source.clone(), cx);
        }
        if let Some(thread_id) = self.mail_open_thread_id.clone() {
            self.load_mail_thread_in_background(thread_id, cx);
        }
        if needs_initial_refresh {
            let mailbox_id = self.mail_selected_tab_id.clone();
            if !mailbox_id.is_empty() {
                self.refresh_mail_mailbox_in_background(mailbox_id, cx);
            }
        }
        self.ensure_mail_identity_loaded(cx);
    }

    fn reset_mail_account_surface_state(&mut self) {
        self.mail_thread_cache.clear();
        self.mail_thread_detail_cache.clear();
        self.clear_mail_contact_histories();
        self.mail_open_thread_body_list_state.reset(0);
        self.mail_open_thread_body_list_thread_id = None;
        self.mail_loading_thread_id = None;
        self.mail_mark_read_after_triage.clear();
        self.mail_mark_read_suppressed.clear();
        self.mail_triage_preflights.clear();
        self.mail_triage_intents.clear();
        self.mail_triage_summaries.clear();
        self.mail_triage_reconciled_generation = self.mail_triage_generation;
        self.mail_loading_message_page = None;
        self.mail_workspace_refresh_generation =
            self.mail_workspace_refresh_generation.wrapping_add(1);
        self.mail_workspace_actions_in_flight = 0;
        self.mail_identity = None;
        self.mail_identity_loading = false;
        self.mail_hovered_thread_id = None;
        self.mail_hovered_message_card_id = None;
        self.mail_hovered_message_action = None;
        self.mail_hovered_history_message_id = None;
        self.mail_expanded_history_message_ids.clear();
        self.mail_expanded_message_header_ids.clear();
        self.mail_action_palette = None;
        self.mail_split_definitions.clear();
        self.mail_split_preferences_supported = false;
        self.mail_split_settings = None;
        self.mail_split_text_inputs.borrow_mut().clear();
        self.mail_error = None;
        self.clear_mail_compose_state();
        self.clear_mail_remote_images();
        *self.mail_search_input.borrow_mut() = None;
    }
}

fn load_mail_account(
    request: MailAccountLoadRequest,
) -> Result<MailAccountWorkspace, MailAccountLoadError> {
    let mut account_workspace = request.bootstrap.load_account(&request.account_id)?;
    if let Some(selected_mailbox_id) = request.selected_mailbox_id.filter(|mailbox_id| {
        mailbox_id != &account_workspace.workspace.selected_mailbox_id
            && account_workspace
                .workspace
                .mailboxes
                .iter()
                .any(|mailbox| mailbox.id == *mailbox_id)
    }) {
        let cached = account_workspace
            .workspace_api
            .load_cached_mail_workspace(&selected_mailbox_id)
            .map_err(MailAccountLoadError::Failed)?;
        account_workspace.workspace = match cached {
            Some(workspace) => {
                account_workspace.needs_initial_refresh = true;
                workspace
            }
            None => account_workspace
                .workspace_api
                .load_mail_workspace(&selected_mailbox_id)
                .map_err(MailAccountLoadError::Failed)?,
        };
    }
    if account_workspace.workspace.account_id != request.account_id {
        return Err(MailAccountLoadError::Failed(format!(
            "Mail account {} returned workspace for {}",
            request.account_id, account_workspace.workspace.account_id
        )));
    }
    Ok(account_workspace)
}

/// Whether the account supports split preferences, and its stored splits.
type AccountSplitPreferences = (bool, Vec<crate::model::MailSplitDefinition>);

fn load_mail_account_split_preferences(
    workspace_api: &dyn crate::model::MailWorkspaceApi,
) -> Result<AccountSplitPreferences, String> {
    let supported = workspace_api.supports_mail_split_preferences();
    let definitions = if supported {
        workspace_api
            .load_mail_split_definitions()
            .map_err(|error| format!("Could not load Mail split preferences: {error}"))?
    } else {
        Vec::new()
    };
    Ok((supported, definitions))
}
