use super::{
    mail_format_addresses, Context, Duration, MailComposeDraftSave, MailComposeField,
    MailComposeMode, MailDraftRequest, MailMessage, SurfaceState,
};

mod snapshot;

#[derive(Clone)]
pub(crate) struct MailComposeDraftTarget {
    pub(crate) mode: MailComposeMode,
    pub(crate) bound_thread_id: Option<String>,
}

const MAIL_COMPOSE_AUTOSAVE_DELAY: Duration = Duration::from_millis(900);

impl SurfaceState {
    pub(crate) fn apply_mail_compose_draft(
        &mut self,
        target: MailComposeDraftTarget,
        request: MailDraftRequest,
        draft: MailMessage,
        cx: &mut Context<Self>,
    ) {
        self.mail_compose_mode = target.mode;
        self.mail_compose_focused_field = if request.to.is_empty() {
            MailComposeField::To
        } else if request.subject.trim().is_empty() {
            MailComposeField::Subject
        } else {
            MailComposeField::Body
        };
        self.mail_shortcuts_focused = false;
        self.mail_compose_draft_id = Some(draft.id.clone());
        self.mail_compose_bound_thread_id = target.bound_thread_id;
        self.mail_compose_identity_id = Some(request.identity_id.clone());
        self.mail_compose_from = Some(request.from.clone());
        self.mail_compose_reply_to = request.reply_to.clone();
        self.mail_compose_to = mail_format_addresses(&request.to);
        self.mail_compose_cc = mail_format_addresses(&request.cc);
        self.mail_compose_subject = request.subject.clone();
        self.mail_compose_body = request.body_text.clone();
        self.mail_compose_attachments = draft.attachments;
        self.mail_compose_in_reply_to = request.in_reply_to;
        self.mail_compose_references = request.references;
        self.mail_compose_autocomplete_selected_index = 0;
        self.mail_compose_pending = false;
        self.mail_compose_error = None;
        self.mail_action_palette = None;
        self.clear_mail_compose_autosave_queue();
        self.mail_compose_autosave_last_saved = self.mail_compose_autosave_snapshot();
        self.sync_mail_open_thread_body_list_state();
        self.maybe_close_mail_compose_after_autosave(cx);
        cx.notify();
    }

    pub(crate) fn open_mail_compose_with_new_draft(
        &mut self,
        mode: MailComposeMode,
        bound_thread_id: Option<String>,
        request: MailDraftRequest,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace_api) = self.mail_workspace_api() else {
            self.mail_compose_error = Some("missing Mail workspace api".to_string());
            cx.notify();
            return;
        };
        self.mail_compose_mode = mode;
        self.mail_compose_bound_thread_id = bound_thread_id.clone();
        self.mail_compose_identity_id = Some(request.identity_id.clone());
        self.mail_compose_from = Some(request.from.clone());
        self.mail_shortcuts_focused = false;
        self.mail_compose_pending = true;
        self.mail_compose_error = None;
        self.mail_compose_close_after_autosave = false;
        self.mail_action_palette = None;
        self.clear_mail_compose_autosave_queue();
        self.sync_mail_open_thread_body_list_state();
        cx.notify();
        self.spawn_background_task(
            request.clone(),
            cx,
            move |request| workspace_api.create_mail_draft(request),
            move |this, result, cx| match result {
                Ok(draft) => {
                    this.apply_mail_compose_draft(
                        MailComposeDraftTarget {
                            mode,
                            bound_thread_id,
                        },
                        request,
                        draft,
                        cx,
                    );
                }
                Err(message) => {
                    this.mail_compose_pending = false;
                    this.mail_compose_error = Some(message);
                    cx.notify();
                }
            },
        );
    }

    pub(super) fn schedule_mail_draft_autosave(&mut self, cx: &mut Context<Self>) {
        self.queue_mail_draft_autosave(MAIL_COMPOSE_AUTOSAVE_DELAY, cx);
    }

    pub(crate) fn save_mail_draft_in_background(&mut self, cx: &mut Context<Self>) {
        self.queue_mail_draft_autosave(Duration::ZERO, cx);
    }

    pub(crate) fn close_mail_compose_view(&mut self, cx: &mut Context<Self>) -> bool {
        if self.mail_compose_mode == MailComposeMode::Closed {
            return false;
        }
        self.mail_compose_close_after_autosave = true;
        self.mail_compose_autosave_scheduled_generation = None;
        self.queue_mail_draft_autosave(Duration::ZERO, cx);
        self.maybe_close_mail_compose_after_autosave(cx);
        true
    }

    pub(super) fn cancel_mail_draft_autosave(&mut self) {
        self.clear_mail_compose_autosave_queue();
    }

    fn queue_mail_draft_autosave(&mut self, delay: Duration, cx: &mut Context<Self>) {
        let Some(snapshot) = self.mail_compose_autosave_snapshot() else {
            return;
        };

        if self
            .mail_compose_autosave_in_flight
            .as_ref()
            .is_some_and(|save| save.snapshot() == snapshot)
        {
            self.mail_compose_autosave_queued = None;
            return;
        }

        if self.mail_compose_autosave_in_flight.is_none()
            && self.mail_compose_autosave_last_saved.as_ref() == Some(&snapshot)
        {
            self.mail_compose_autosave_queued = None;
            self.maybe_close_mail_compose_after_autosave(cx);
            self.maybe_send_mail_draft_after_autosave(cx);
            return;
        }

        let save = self.next_mail_compose_autosave(snapshot);
        self.mail_compose_autosave_queued = Some(save.clone());

        if delay.is_zero() {
            self.mail_compose_autosave_scheduled_generation = None;
            self.start_next_mail_draft_autosave(cx);
            return;
        }

        self.mail_compose_autosave_scheduled_generation = Some(save.generation);
        self.spawn_timer_task(save.generation, delay, cx, |this, generation, cx| {
            if this.mail_compose_autosave_scheduled_generation != Some(generation) {
                return;
            }
            this.mail_compose_autosave_scheduled_generation = None;
            this.start_next_mail_draft_autosave(cx);
        });
    }

    fn start_next_mail_draft_autosave(&mut self, cx: &mut Context<Self>) {
        if self.mail_compose_autosave_in_flight.is_some() {
            return;
        }

        let Some(save) = self.mail_compose_autosave_queued.take() else {
            self.maybe_close_mail_compose_after_autosave(cx);
            self.maybe_send_mail_draft_after_autosave(cx);
            return;
        };

        if self.mail_compose_draft_id.as_deref() != Some(save.draft_id.as_str()) {
            self.maybe_send_mail_draft_after_autosave(cx);
            return;
        }

        if self.mail_compose_autosave_last_saved.as_ref() == Some(&save.snapshot()) {
            self.maybe_close_mail_compose_after_autosave(cx);
            self.maybe_send_mail_draft_after_autosave(cx);
            return;
        }

        let Some(workspace_api) = self.mail_workspace_api() else {
            if self.mail_compose_send_after_autosave.is_some() {
                self.mail_compose_send_after_autosave = None;
                self.mail_compose_pending = false;
                self.mail_compose_error = Some("missing Mail workspace api".to_string());
                cx.notify();
            }
            return;
        };

        let save_for_apply = save.clone();
        self.mail_compose_autosave_in_flight = Some(save_for_apply.clone());
        self.spawn_background_task(
            save,
            cx,
            move |save| {
                let draft_id = save.draft_id;
                workspace_api.update_mail_draft(&draft_id, save.request)
            },
            move |this, result, cx| {
                this.apply_mail_draft_autosave_result(save_for_apply, result, cx);
            },
        );
    }

    fn apply_mail_draft_autosave_result(
        &mut self,
        save: MailComposeDraftSave,
        result: Result<MailMessage, String>,
        cx: &mut Context<Self>,
    ) {
        if self
            .mail_compose_autosave_in_flight
            .as_ref()
            .map(|in_flight| in_flight.generation)
            != Some(save.generation)
        {
            return;
        }

        self.mail_compose_autosave_in_flight = None;
        let saved_snapshot = save.snapshot();

        match result {
            Ok(_) => {
                self.mail_compose_autosave_last_saved = Some(saved_snapshot);
                self.start_next_mail_draft_autosave(cx);
            }
            Err(message) => {
                if self.mail_compose_autosave_queued.is_some() {
                    self.start_next_mail_draft_autosave(cx);
                    return;
                }

                if self.mail_compose_autosave_snapshot().as_ref() == Some(&saved_snapshot) {
                    self.mail_compose_close_after_autosave = false;
                    self.mail_compose_send_after_autosave = None;
                    self.mail_compose_pending = false;
                    self.mail_compose_error = Some(message);
                    cx.notify();
                }
            }
        }
    }

    fn maybe_close_mail_compose_after_autosave(&mut self, cx: &mut Context<Self>) {
        if !self.mail_compose_close_after_autosave
            || self.mail_compose_autosave_scheduled_generation.is_some()
            || self.mail_compose_autosave_in_flight.is_some()
            || self.mail_compose_autosave_queued.is_some()
        {
            return;
        }
        let Some(snapshot) = self.mail_compose_autosave_snapshot() else {
            if self.mail_compose_pending {
                return;
            }
            self.clear_mail_compose_state();
            self.sync_mail_open_thread_body_list_state();
            cx.notify();
            return;
        };
        if self.mail_compose_autosave_last_saved.as_ref() != Some(&snapshot) {
            return;
        }
        self.clear_mail_compose_state();
        self.sync_mail_open_thread_body_list_state();
        cx.notify();
    }

    pub(super) fn flush_mail_draft_autosave_for_send(&mut self, cx: &mut Context<Self>) {
        self.mail_compose_autosave_scheduled_generation = None;
        let Some(snapshot) = self.mail_compose_autosave_snapshot() else {
            self.mail_compose_send_after_autosave = None;
            self.mail_compose_pending = false;
            self.mail_compose_error = Some("missing draft id".to_string());
            cx.notify();
            return;
        };

        if self.mail_compose_autosave_in_flight.is_none()
            && self.mail_compose_autosave_queued.is_none()
            && self.mail_compose_autosave_last_saved.as_ref() == Some(&snapshot)
        {
            self.maybe_send_mail_draft_after_autosave(cx);
            return;
        }

        self.queue_mail_draft_autosave(Duration::ZERO, cx);
        self.maybe_send_mail_draft_after_autosave(cx);
    }

    fn maybe_send_mail_draft_after_autosave(&mut self, cx: &mut Context<Self>) {
        if self.mail_compose_autosave_in_flight.is_some()
            || self.mail_compose_autosave_queued.is_some()
        {
            return;
        }

        let Some(intent) = self.mail_compose_send_after_autosave.clone() else {
            return;
        };
        let Some(snapshot) = self.mail_compose_autosave_snapshot() else {
            self.mail_compose_send_after_autosave = None;
            self.mail_compose_pending = false;
            self.mail_compose_error = Some("missing draft id".to_string());
            cx.notify();
            return;
        };

        if snapshot.draft_id != intent.draft_id {
            self.mail_compose_send_after_autosave = None;
            return;
        }

        if self.mail_compose_autosave_last_saved.as_ref() != Some(&snapshot) {
            self.queue_mail_draft_autosave(Duration::ZERO, cx);
            return;
        }

        let Some(intent) = self.mail_compose_send_after_autosave.take() else {
            return;
        };
        self.send_mail_draft_now(intent, cx);
    }

    pub(crate) fn clear_mail_compose_state(&mut self) {
        self.mail_compose_mode = MailComposeMode::Closed;
        self.mail_compose_focused_field = MailComposeField::To;
        self.mail_shortcuts_focused = true;
        self.mail_compose_draft_id = None;
        self.mail_compose_bound_thread_id = None;
        self.mail_compose_identity_id = None;
        self.mail_compose_from = None;
        self.mail_compose_reply_to.clear();
        self.mail_compose_to.clear();
        self.mail_compose_cc.clear();
        self.mail_compose_subject.clear();
        self.mail_compose_body.clear();
        self.mail_compose_attachments.clear();
        self.mail_compose_in_reply_to.clear();
        self.mail_compose_references.clear();
        self.mail_compose_autocomplete_selected_index = 0;
        self.mail_compose_pending = false;
        self.mail_compose_error = None;
        self.mail_compose_close_after_autosave = false;
        self.mail_action_palette = None;
        self.clear_mail_compose_autosave_queue();
    }
}
