mod hover;
mod mailbox;
mod queries;

use super::{Context, ListState, MailComposeMode, MailListSource, MailWorkspace, SurfaceState};

impl SurfaceState {
    pub(crate) fn open_mail_thread(
        &mut self,
        thread_id: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        let thread_id = thread_id.into();
        self.reset_mail_open_thread_read_session();
        self.mail_selected_thread_id = Some(thread_id.clone());
        self.mail_open_thread_id = Some(thread_id.clone());
        self.mail_thread_detail_cache.remove(&thread_id);
        self.mail_remote_image_discovery_thread_id = None;
        self.sync_mail_open_thread_body_list_state();
        self.mail_hovered_thread_id = None;
        self.mail_hovered_message_card_id = None;
        self.mail_hovered_message_action = None;
        self.mail_hovered_history_message_id = None;
        self.mail_expanded_history_message_ids.clear();
        self.mail_expanded_message_header_ids.clear();
        self.mail_action_palette = None;
        self.mail_error = None;
        self.mark_mail_thread_read_on_open(thread_id.as_str(), cx);
        cx.notify();
        self.load_mail_thread_in_background(thread_id, cx);
    }

    pub(crate) fn close_mail_thread(&mut self, cx: &mut Context<Self>) {
        if self.mail_open_thread_id.is_none() {
            return;
        }
        self.mail_open_thread_id = None;
        self.reset_mail_open_thread_read_session();
        self.deselect_mail_contact_history();
        self.mail_remote_image_discovery_thread_id = None;
        self.mail_hovered_thread_id = None;
        self.mail_hovered_message_card_id = None;
        self.mail_hovered_message_action = None;
        self.mail_hovered_history_message_id = None;
        self.mail_expanded_history_message_ids.clear();
        self.mail_expanded_message_header_ids.clear();
        self.mail_action_palette = None;
        cx.notify();
    }

    pub(crate) fn refresh_mail_mailbox_in_background(
        &mut self,
        mailbox_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace_api) = self.mail_workspace_api() else {
            return;
        };
        self.mail_workspace_refresh_generation =
            self.mail_workspace_refresh_generation.wrapping_add(1);
        let workspace_refresh_generation = self.mail_workspace_refresh_generation;
        let triage_generation = self.mail_triage_generation;
        let workspace_was_quiescent = self.mail_workspace_actions_in_flight == 0
            && self.mail_triage_preflights.is_empty()
            && self.mail_triage_intents.is_empty();
        self.spawn_background_task(
            mailbox_id.clone(),
            cx,
            move |mailbox_id| workspace_api.refresh_mail_workspace(&mailbox_id),
            move |this, result, cx| {
                if this.mail_workspace_refresh_generation != workspace_refresh_generation
                    || this.mail_selected_tab_id != mailbox_id
                {
                    return;
                }
                match result {
                    Ok(workspace) => {
                        if workspace_was_quiescent
                            && this.mail_triage_generation == triage_generation
                            && this.mail_workspace_actions_in_flight == 0
                            && this.mail_triage_preflights.is_empty()
                            && this.mail_triage_intents.is_empty()
                        {
                            this.mail_triage_summaries.clear();
                            this.mail_triage_reconciled_generation = triage_generation;
                        }
                        this.apply_mail_workspace_refresh(workspace, cx);
                    }
                    Err(message) => {
                        this.handle_mail_workspace_error(message, cx);
                    }
                }
            },
        );
    }

    pub(crate) fn prefetch_mail_mailbox_in_background(
        &mut self,
        mailbox_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace_api) = self.mail_workspace_api() else {
            return;
        };
        self.spawn_background_task(
            mailbox_id,
            cx,
            move |mailbox_id| workspace_api.refresh_mail_workspace(&mailbox_id),
            |_this, _result, _cx| {},
        );
    }

    pub(crate) fn apply_mail_workspace_refresh(
        &mut self,
        mut workspace: MailWorkspace,
        cx: &mut Context<Self>,
    ) {
        self.apply_pending_mail_triage_to_workspace(&mut workspace);
        let search_open = self.mail_search_state.session().is_some();
        let previous_mailbox_thread_ids = self
            .mail_workspace()
            .map(mail_workspace_thread_ids)
            .unwrap_or_default();
        let refreshed_mailbox_thread_ids = mail_workspace_thread_ids(&workspace);
        let selected_thread_id = self.refreshed_mail_selected_thread_id(&workspace, search_open);
        self.replace_mail_workspace(workspace.clone());
        self.mail_thread_detail_cache.clear();
        self.mail_remote_image_discovery_thread_id = None;
        self.mail_open_thread_body_list_thread_id = None;
        self.mail_loading_message_page = None;
        self.mail_selected_tab_id = workspace.selected_mailbox_id;
        if !search_open {
            self.mail_list_source = MailListSource::Mailbox(self.mail_selected_tab_id.clone());
        }
        self.mail_selected_thread_id = selected_thread_id.clone();
        if search_open {
            reconcile_mailbox_list_state(
                &self.mail_list_state,
                &previous_mailbox_thread_ids,
                &refreshed_mailbox_thread_ids,
            );
        }
        self.sync_mail_list_rows();
        self.mail_action_palette = None;
        if !search_open
            && self
                .mail_open_thread_id
                .as_ref()
                .is_some_and(|thread_id| selected_thread_id.as_ref() != Some(thread_id))
        {
            self.reset_mail_open_thread_read_session();
            self.mail_open_thread_id = None;
        }
        self.mail_error = None;
        cx.notify();
        if let Some(thread_id) = self.mail_open_thread_id.clone() {
            self.load_mail_thread_in_background(thread_id, cx);
        }
    }

    fn refreshed_mail_selected_thread_id(
        &self,
        workspace: &MailWorkspace,
        search_open: bool,
    ) -> Option<String> {
        if search_open {
            return self.mail_selected_thread_id.clone();
        }
        self.mail_selected_thread_id
            .clone()
            .filter(|thread_id| {
                workspace
                    .messages
                    .iter()
                    .any(|message| message.thread_id == *thread_id)
            })
            .or_else(|| {
                workspace
                    .messages
                    .first()
                    .map(|message| message.thread_id.clone())
            })
    }

    pub(crate) fn load_mail_thread_in_background(
        &mut self,
        thread_id: String,
        cx: &mut Context<Self>,
    ) {
        if self.mail_thread_cache.contains_key(&thread_id)
            || self.mail_loading_thread_id.as_deref() == Some(thread_id.as_str())
        {
            return;
        }
        let Some(workspace_api) = self.mail_workspace_api() else {
            return;
        };
        self.mail_loading_thread_id = Some(thread_id.clone());
        cx.notify();
        self.spawn_background_task(
            thread_id.clone(),
            cx,
            move |thread_id| workspace_api.load_mail_thread(&thread_id),
            move |this, result, cx| {
                if this.mail_loading_thread_id.as_deref() != Some(thread_id.as_str()) {
                    return;
                }
                this.mail_loading_thread_id = None;
                match result {
                    Ok(mut thread) => {
                        this.apply_pending_mail_triage_to_thread(&mut thread);
                        let thread_id = thread.id.clone();
                        let mark_read = this.mail_open_thread_id.as_deref()
                            == Some(thread_id.as_str())
                            && thread.messages.iter().any(|message| message.is_unread);
                        this.apply_mail_thread_summary_state(&thread);
                        this.mail_thread_cache.insert(thread_id.clone(), thread);
                        this.mail_thread_detail_cache.remove(&thread_id);
                        this.mail_remote_image_discovery_thread_id = None;
                        this.ensure_mail_active_thread_detail_cached();
                        this.sync_mail_open_thread_body_list_state();
                        this.sync_mail_list_rows();
                        if mark_read {
                            this.mark_mail_thread_read_on_open(thread_id.as_str(), cx);
                        }
                        cx.notify();
                    }
                    Err(message) => {
                        this.handle_mail_workspace_error(message, cx);
                    }
                }
            },
        );
    }

    pub(crate) fn ensure_mail_identity_loaded(&mut self, cx: &mut Context<Self>) {
        if !self.mail_active_account_can_submit() {
            self.mail_identity = None;
            self.mail_identity_loading = false;
            return;
        }
        if self.mail_identity.is_some() || self.mail_identity_loading {
            return;
        }
        let Some(workspace_api) = self.mail_workspace_api() else {
            return;
        };
        self.mail_identity_loading = true;
        self.spawn_background_task(
            workspace_api,
            cx,
            move |workspace_api| workspace_api.load_mail_identity(),
            |this, result, cx| {
                this.mail_identity_loading = false;
                match result {
                    Ok(identity) => {
                        this.mail_identity = Some(identity);
                        if this.mail_compose_mode == MailComposeMode::New
                            && this.mail_compose_draft_id.is_none()
                            && !this.mail_compose_pending
                        {
                            this.open_new_mail_composer(cx);
                        } else {
                            cx.notify();
                        }
                    }
                    Err(message) => {
                        if this.mail_compose_mode == MailComposeMode::New
                            && this.mail_compose_draft_id.is_none()
                        {
                            this.mail_compose_error = Some(message.clone());
                        }
                        this.handle_mail_workspace_error(message, cx);
                    }
                }
            },
        );
    }
}

fn reconcile_mailbox_list_state(
    list_state: &ListState,
    previous_thread_ids: &[String],
    refreshed_thread_ids: &[String],
) {
    let prefix_len = previous_thread_ids
        .iter()
        .zip(refreshed_thread_ids)
        .take_while(|(previous, refreshed)| previous == refreshed)
        .count();
    let suffix_len = previous_thread_ids[prefix_len..]
        .iter()
        .rev()
        .zip(refreshed_thread_ids[prefix_len..].iter().rev())
        .take_while(|(previous, refreshed)| previous == refreshed)
        .count();
    let previous_changed_end = previous_thread_ids.len() - suffix_len;
    let refreshed_changed_count = refreshed_thread_ids.len() - prefix_len - suffix_len;
    if prefix_len != previous_changed_end || refreshed_changed_count != 0 {
        list_state.splice(prefix_len..previous_changed_end, refreshed_changed_count);
    }
}

fn mail_workspace_thread_ids(workspace: &MailWorkspace) -> Vec<String> {
    workspace
        .messages
        .iter()
        .map(|message| message.thread_id.clone())
        .collect()
}
