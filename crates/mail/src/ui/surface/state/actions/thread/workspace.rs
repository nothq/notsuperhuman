use super::super::super::super::{Context, MailSnoozePreset, SurfaceState};

impl SurfaceState {
    fn begin_mail_workspace_action(&mut self) {
        assert_eq!(
            self.mail_workspace_actions_in_flight, 0,
            "Mail workspace actions must be serialized"
        );
        self.mail_workspace_actions_in_flight = 1;
        self.mail_workspace_refresh_generation =
            self.mail_workspace_refresh_generation.wrapping_add(1);
    }

    pub(in super::super) fn archive_mail_thread(
        &mut self,
        thread_id: String,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.mail_thread_action_allowed(thread_id.as_str(), crate::ui::MailRowAction::MarkDone)
        {
            return true;
        }
        let Some(workspace_api) = self.mail_workspace_api() else {
            self.report_mail_error("missing Mail workspace api");
            cx.notify();
            return true;
        };
        let action_context = match self.mail_thread_action_context(thread_id.as_str()) {
            Ok(action_context) => action_context,
            Err(message) => {
                self.report_mail_error(message);
                cx.notify();
                return true;
            }
        };
        self.begin_mail_workspace_action();
        self.advance_mail_selection_after_thread_removal(thread_id.as_str(), cx);
        self.remove_mail_search_thread(thread_id.as_str());
        let request_refresh_mailbox_id = action_context.refresh_mailbox_id.clone();
        let request = (
            action_context.source_mailbox_id,
            action_context.refresh_mailbox_id,
            thread_id,
        );
        self.spawn_background_task(
            request,
            cx,
            move |(source_mailbox_id, refresh_mailbox_id, thread_id)| {
                let workspace =
                    workspace_api.archive_mail_thread(&source_mailbox_id, &thread_id)?;
                refresh_action_workspace(
                    workspace_api.as_ref(),
                    workspace,
                    &source_mailbox_id,
                    &refresh_mailbox_id,
                )
            },
            move |this, result, cx| {
                this.apply_mail_thread_action_result(&request_refresh_mailbox_id, result, cx);
            },
        );
        true
    }

    pub(in super::super) fn move_mail_thread(
        &mut self,
        thread_id: String,
        destination_mailbox_id: String,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.mail_thread_move_allowed(thread_id.as_str(), destination_mailbox_id.as_str()) {
            return true;
        }
        let Some(workspace_api) = self.mail_workspace_api() else {
            self.report_mail_error("missing Mail workspace api");
            cx.notify();
            return true;
        };
        let action_context = match self.mail_thread_action_context(thread_id.as_str()) {
            Ok(action_context) => action_context,
            Err(message) => {
                self.report_mail_error(message);
                cx.notify();
                return true;
            }
        };
        self.begin_mail_workspace_action();
        self.advance_mail_selection_after_thread_removal(thread_id.as_str(), cx);
        self.remove_mail_search_thread(thread_id.as_str());
        let request_refresh_mailbox_id = action_context.refresh_mailbox_id.clone();
        let request = (
            action_context.source_mailbox_id,
            action_context.refresh_mailbox_id,
            thread_id,
            destination_mailbox_id,
        );
        self.spawn_background_task(
            request,
            cx,
            move |(source_mailbox_id, refresh_mailbox_id, thread_id, destination_mailbox_id)| {
                let workspace = workspace_api.move_mail_thread(
                    &source_mailbox_id,
                    &thread_id,
                    &destination_mailbox_id,
                )?;
                refresh_action_workspace(
                    workspace_api.as_ref(),
                    workspace,
                    &source_mailbox_id,
                    &refresh_mailbox_id,
                )
            },
            move |this, result, cx| {
                this.apply_mail_thread_action_result(&request_refresh_mailbox_id, result, cx);
            },
        );
        true
    }

    pub(in super::super) fn snooze_mail_thread(
        &mut self,
        thread_id: String,
        preset: MailSnoozePreset,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.mail_thread_action_allowed(thread_id.as_str(), crate::ui::MailRowAction::RemindMe)
        {
            return true;
        }
        let Some(workspace_api) = self.mail_workspace_api() else {
            self.report_mail_error("missing Mail workspace api");
            cx.notify();
            return true;
        };
        let action_context = match self.mail_thread_action_context(thread_id.as_str()) {
            Ok(action_context) => action_context,
            Err(message) => {
                self.report_mail_error(message);
                cx.notify();
                return true;
            }
        };
        let remind_at = (time::OffsetDateTime::now_utc() + preset.duration())
            .format(&time::format_description::well_known::Rfc3339)
            .expect("Rfc3339 formatting should succeed");
        self.begin_mail_workspace_action();
        self.advance_mail_selection_after_thread_removal(thread_id.as_str(), cx);
        self.remove_mail_search_thread(thread_id.as_str());
        let request_refresh_mailbox_id = action_context.refresh_mailbox_id.clone();
        let request = (
            action_context.source_mailbox_id,
            action_context.refresh_mailbox_id,
            thread_id,
            remind_at,
        );
        self.spawn_background_task(
            request,
            cx,
            move |(source_mailbox_id, refresh_mailbox_id, thread_id, remind_at)| {
                let workspace =
                    workspace_api.snooze_mail_thread(&source_mailbox_id, &thread_id, &remind_at)?;
                refresh_action_workspace(
                    workspace_api.as_ref(),
                    workspace,
                    &source_mailbox_id,
                    &refresh_mailbox_id,
                )
            },
            move |this, result, cx| {
                this.apply_mail_thread_action_result(&request_refresh_mailbox_id, result, cx);
            },
        );
        true
    }

    fn apply_mail_thread_action_result(
        &mut self,
        refresh_mailbox_id: &str,
        result: Result<crate::ui::MailWorkspace, String>,
        cx: &mut Context<Self>,
    ) {
        self.mail_workspace_actions_in_flight = self
            .mail_workspace_actions_in_flight
            .checked_sub(1)
            .expect("Mail workspace action completed without an active request");
        self.mail_workspace_refresh_generation =
            self.mail_workspace_refresh_generation.wrapping_add(1);
        let selected_mailbox_matches = self.mail_selected_tab_id == refresh_mailbox_id;
        match result {
            Ok(workspace) if selected_mailbox_matches => {
                self.apply_mail_workspace_refresh(workspace, cx)
            }
            Ok(_) => {}
            Err(message) if selected_mailbox_matches => {
                self.handle_mail_workspace_error(message, cx);
            }
            Err(_) => {}
        }
        self.reconcile_mail_triage_workspace_in_background(cx);
    }

    fn advance_mail_selection_after_thread_removal(
        &mut self,
        thread_id: &str,
        cx: &mut Context<Self>,
    ) {
        let successor_thread_id = self.successor_mail_thread_id(thread_id);
        if self.mail_selected_thread_id.as_deref() == Some(thread_id) {
            self.mail_selected_thread_id = successor_thread_id.clone();
        }
        let should_load_successor = self.mail_open_thread_id.as_deref() == Some(thread_id);
        if should_load_successor {
            self.reset_mail_open_thread_read_session();
            self.mail_open_thread_id = successor_thread_id.clone();
            self.mail_remote_image_discovery_thread_id = None;
        }
        self.mail_hovered_thread_id = None;
        self.mail_hovered_message_card_id = None;
        self.mail_hovered_message_action = None;
        self.mail_hovered_history_message_id = None;
        self.mail_expanded_history_message_ids.clear();
        self.mail_expanded_message_header_ids.clear();
        if should_load_successor {
            self.sync_mail_open_thread_body_list_state();
        }
        self.mail_action_palette = None;
        self.mail_error = None;
        cx.notify();
        if let (true, Some(next_thread_id)) = (should_load_successor, successor_thread_id) {
            self.mark_mail_thread_read_on_open(next_thread_id.as_str(), cx);
            self.load_mail_thread_in_background(next_thread_id, cx);
        }
    }

    fn successor_mail_thread_id(&self, thread_id: &str) -> Option<String> {
        let current_index = self
            .mail_list_threads
            .iter()
            .position(|thread| thread.id == thread_id)?;
        if self.mail_list_threads.len() <= 1 {
            return None;
        }
        let successor_index = if current_index + 1 < self.mail_list_threads.len() {
            current_index + 1
        } else {
            current_index - 1
        };
        self.mail_list_threads
            .get(successor_index)
            .map(|thread| thread.id.clone())
    }
}

fn refresh_action_workspace(
    workspace_api: &dyn crate::model::MailWorkspaceApi,
    workspace: crate::ui::MailWorkspace,
    source_mailbox_id: &str,
    refresh_mailbox_id: &str,
) -> Result<crate::ui::MailWorkspace, String> {
    if source_mailbox_id == refresh_mailbox_id {
        Ok(workspace)
    } else {
        workspace_api.refresh_mail_workspace(refresh_mailbox_id)
    }
}
