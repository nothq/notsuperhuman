use super::super::super::super::{
    Context, MailReadState, MailStarState, MailThread, MailThreadTriageChange,
    MailThreadTriageRequest, MailTriageControl, MailTriageIntent, MailTriageIntentKey,
    SurfaceState,
};
use super::task::{inverse_triage_change, perform_mail_thread_triage, MailTriageTaskOutcome};

impl SurfaceState {
    pub(crate) fn toggle_active_mail_thread_triage(
        &mut self,
        control: MailTriageControl,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(thread_id) = self
            .mail_open_thread_id
            .clone()
            .or_else(|| self.mail_selected_thread_id.clone())
        else {
            return false;
        };
        self.toggle_mail_thread_triage(thread_id, control, cx)
    }

    pub(crate) fn toggle_mail_thread_triage(
        &mut self,
        thread_id: String,
        control: MailTriageControl,
        cx: &mut Context<Self>,
    ) -> bool {
        let started = if self.mail_thread_cache.contains_key(&thread_id) {
            let Some(change) = self.mail_toggled_triage_change(thread_id.as_str(), control) else {
                return false;
            };
            self.start_mail_thread_triage(thread_id.clone(), control, change, cx)
        } else {
            self.start_mail_thread_triage_preflight(thread_id.clone(), control, cx)
        };
        if started {
            self.suppress_automatic_mark_read_after_explicit_toggle(thread_id.as_str(), control);
        }
        started
    }

    pub(crate) fn mark_mail_thread_read_on_open(
        &mut self,
        thread_id: &str,
        cx: &mut Context<Self>,
    ) {
        if self.mail_mark_read_suppressed.contains(thread_id) {
            self.mail_mark_read_after_triage
                .insert(thread_id.to_string());
            return;
        }
        if self.mail_thread_unread(thread_id) != Some(true) {
            self.mail_mark_read_after_triage.remove(thread_id);
            return;
        }
        if self.mail_thread_triage_in_flight(thread_id) {
            self.mail_mark_read_after_triage
                .insert(thread_id.to_string());
            return;
        }
        self.mail_mark_read_after_triage.remove(thread_id);
        self.start_mail_thread_triage(
            thread_id.to_string(),
            MailTriageControl::Read,
            MailThreadTriageChange::Read(MailReadState::Read),
            cx,
        );
    }

    pub(crate) fn mail_thread_triage_allowed(
        &self,
        thread_id: &str,
        control: MailTriageControl,
    ) -> bool {
        if self.mail_thread_triage_in_flight(thread_id) {
            return false;
        }
        if self.mail_active_account_is_read_only()
            || !self
                .mail_workspace_api()
                .is_some_and(|api| api.supports_mail_thread_triage())
        {
            return false;
        }
        let Some(workspace) = self.mail_workspace() else {
            return false;
        };
        let cached_messages = self
            .mail_thread_cache
            .get(thread_id)
            .map(|thread| thread.messages.as_slice());
        let visible_messages = self.mail_visible_messages();
        let messages = cached_messages.unwrap_or(visible_messages);
        let messages = messages
            .iter()
            .filter(|message| message.thread_id == thread_id)
            .collect::<Vec<_>>();
        !messages.is_empty()
            && messages.iter().all(|message| {
                !message.mailbox_ids.is_empty()
                    && message.mailbox_ids.iter().all(|mailbox_id| {
                        workspace
                            .mailboxes
                            .iter()
                            .find(|mailbox| mailbox.id == *mailbox_id)
                            .is_some_and(|mailbox| match control {
                                MailTriageControl::Read => mailbox.rights.may_set_seen,
                                MailTriageControl::Star => mailbox.rights.may_set_keywords,
                            })
                    })
            })
    }

    fn mail_thread_triage_in_flight(&self, thread_id: &str) -> bool {
        self.mail_triage_preflights
            .iter()
            .any(|key| key.thread_id == thread_id)
            || self
                .mail_triage_intents
                .keys()
                .any(|key| key.thread_id == thread_id)
    }

    pub(super) fn retry_deferred_mail_thread_read(
        &mut self,
        thread_id: &str,
        cx: &mut Context<Self>,
    ) {
        if !self.mail_mark_read_after_triage.remove(thread_id) {
            return;
        }
        if self.mail_open_thread_id.as_deref() == Some(thread_id) {
            self.mark_mail_thread_read_on_open(thread_id, cx);
        }
    }

    pub(crate) fn reset_mail_open_thread_read_session(&mut self) {
        self.mail_mark_read_after_triage.clear();
        self.mail_mark_read_suppressed.clear();
    }

    pub(super) fn reconcile_mail_triage_workspace_in_background(&mut self, cx: &mut Context<Self>) {
        if self.mail_triage_reconciled_generation == self.mail_triage_generation
            || self.mail_workspace_actions_in_flight != 0
            || !self.mail_triage_preflights.is_empty()
            || !self.mail_triage_intents.is_empty()
        {
            return;
        }
        let Some(mailbox_id) = self
            .mail_workspace()
            .map(|workspace| workspace.selected_mailbox_id.clone())
        else {
            return;
        };
        self.refresh_mail_mailbox_in_background(mailbox_id, cx);
    }

    fn suppress_automatic_mark_read_after_explicit_toggle(
        &mut self,
        thread_id: &str,
        control: MailTriageControl,
    ) {
        if control == MailTriageControl::Read
            && self.mail_open_thread_id.as_deref() == Some(thread_id)
        {
            self.mail_mark_read_after_triage.remove(thread_id);
            self.mail_mark_read_suppressed.insert(thread_id.to_string());
        }
    }

    pub(super) fn clear_automatic_mark_read_suppression(
        &mut self,
        thread_id: &str,
        control: MailTriageControl,
    ) {
        if control == MailTriageControl::Read {
            self.mail_mark_read_suppressed.remove(thread_id);
        }
    }

    pub(super) fn mail_toggled_triage_change(
        &self,
        thread_id: &str,
        control: MailTriageControl,
    ) -> Option<MailThreadTriageChange> {
        match control {
            MailTriageControl::Read => self.mail_thread_unread(thread_id).map(|unread| {
                MailThreadTriageChange::Read(if unread {
                    MailReadState::Read
                } else {
                    MailReadState::Unread
                })
            }),
            MailTriageControl::Star => self.mail_thread_starred(thread_id).map(|starred| {
                MailThreadTriageChange::Star(if starred {
                    MailStarState::Unstarred
                } else {
                    MailStarState::Starred
                })
            }),
        }
    }

    fn mail_thread_unread(&self, thread_id: &str) -> Option<bool> {
        self.mail_thread_cache
            .get(thread_id)
            .map(|thread| thread.messages.iter().any(|message| message.is_unread))
            .or_else(|| {
                self.mail_visible_messages()
                    .iter()
                    .find(|message| message.thread_id == thread_id)
                    .map(|message| message.is_unread)
            })
    }

    fn mail_thread_starred(&self, thread_id: &str) -> Option<bool> {
        self.mail_thread_cache
            .get(thread_id)
            .map(|thread| thread.messages.iter().any(|message| message.is_starred))
            .or_else(|| {
                self.mail_visible_messages()
                    .iter()
                    .find(|message| message.thread_id == thread_id)
                    .map(|message| message.is_starred)
            })
    }

    pub(super) fn start_mail_thread_triage(
        &mut self,
        thread_id: String,
        control: MailTriageControl,
        change: MailThreadTriageChange,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.mail_thread_triage_allowed(thread_id.as_str(), control) {
            return false;
        }
        let Some(workspace_api) = self.mail_workspace_api() else {
            return false;
        };
        let Some(mailbox_id) = self
            .mail_workspace()
            .map(|workspace| workspace.selected_mailbox_id.clone())
        else {
            return false;
        };
        let (key, intent) = self.begin_mail_thread_triage_intent(&thread_id, control, change);
        self.mail_error = None;
        cx.notify();
        let request = MailThreadTriageRequest {
            mailbox_id,
            thread_id: thread_id.clone(),
            change,
        };
        self.spawn_background_task(
            request,
            cx,
            move |request| perform_mail_thread_triage(workspace_api.as_ref(), request),
            move |this, outcome, cx| {
                if this.mail_triage_intents.get(&key) != Some(&intent) {
                    return;
                }
                this.mail_triage_intents.remove(&key);
                this.apply_mail_triage_task_outcome(thread_id.as_str(), change, outcome, cx);
                this.retry_deferred_mail_thread_read(thread_id.as_str(), cx);
                this.reconcile_mail_triage_workspace_in_background(cx);
            },
        );
        true
    }

    fn begin_mail_thread_triage_intent(
        &mut self,
        thread_id: &str,
        control: MailTriageControl,
        change: MailThreadTriageChange,
    ) -> (MailTriageIntentKey, MailTriageIntent) {
        self.mail_triage_generation = self.mail_triage_generation.wrapping_add(1);
        let intent = MailTriageIntent {
            generation: self.mail_triage_generation,
            change,
        };
        let key = MailTriageIntentKey {
            thread_id: thread_id.to_string(),
            control,
        };
        self.mail_triage_intents.insert(key.clone(), intent);
        self.apply_mail_triage_change(thread_id, change);
        (key, intent)
    }

    fn apply_mail_triage_task_outcome(
        &mut self,
        thread_id: &str,
        change: MailThreadTriageChange,
        outcome: MailTriageTaskOutcome,
        cx: &mut Context<Self>,
    ) {
        let MailTriageTaskOutcome { error, thread } = outcome;
        let reconciled = self.reconcile_mail_triage_task_thread(thread_id, thread, error.is_none());
        if !reconciled {
            self.rollback_mail_triage_change(thread_id, change);
        }
        self.ensure_mail_active_thread_detail_cached();
        self.sync_mail_open_thread_body_list_state();
        if self.mail_search_session().is_some() {
            self.refresh_current_mail_search_results(cx);
        } else {
            self.sync_mail_list_rows();
        }
        match error {
            Some(error) => self.report_mail_error(error),
            None => self.mail_error = None,
        }
        cx.notify();
    }

    fn reconcile_mail_triage_task_thread(
        &mut self,
        thread_id: &str,
        thread: Result<MailThread, String>,
        mut reconciled: bool,
    ) -> bool {
        let Ok(mut thread) = thread else {
            return reconciled;
        };
        if thread.id != thread_id {
            return reconciled;
        }
        if self.mail_loading_thread_id.as_deref() == Some(thread_id) {
            self.mail_loading_thread_id = None;
        }
        self.apply_pending_mail_triage_to_thread(&mut thread);
        let summary = (
            thread.messages.iter().any(|message| message.is_unread),
            thread.messages.iter().any(|message| message.is_starred),
        );
        self.mail_triage_summaries
            .insert(thread_id.to_string(), summary);
        self.apply_mail_thread_summary_state(&thread);
        self.mail_thread_cache.insert(thread_id.to_string(), thread);
        self.mail_thread_detail_cache.remove(thread_id);
        reconciled = true;
        reconciled
    }

    fn rollback_mail_triage_change(&mut self, thread_id: &str, change: MailThreadTriageChange) {
        self.apply_mail_triage_change(thread_id, inverse_triage_change(change));
        if let (Some(unread), Some(starred)) = (
            self.mail_thread_unread(thread_id),
            self.mail_thread_starred(thread_id),
        ) {
            self.mail_triage_summaries
                .insert(thread_id.to_string(), (unread, starred));
        }
        if self.mail_loading_thread_id.as_deref() == Some(thread_id) {
            self.mail_loading_thread_id = None;
        }
        self.mail_thread_cache.remove(thread_id);
        self.mail_thread_detail_cache.remove(thread_id);
    }
}
