use super::super::super::super::{
    Context, MailThread, MailTriageControl, MailTriageIntentKey, SurfaceState,
};

impl SurfaceState {
    pub(super) fn start_mail_thread_triage_preflight(
        &mut self,
        thread_id: String,
        control: MailTriageControl,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.mail_thread_triage_allowed(thread_id.as_str(), control) {
            return false;
        }
        let Some(workspace_api) = self.mail_workspace_api() else {
            return false;
        };
        let key = MailTriageIntentKey {
            thread_id: thread_id.clone(),
            control,
        };
        if !self.mail_triage_preflights.insert(key.clone()) {
            return false;
        }
        self.mail_error = None;
        cx.notify();
        self.spawn_background_task(
            thread_id,
            cx,
            move |thread_id| workspace_api.load_mail_thread(&thread_id),
            move |this, result, cx| {
                this.finish_mail_thread_triage_preflight(&key, result, cx);
            },
        );
        true
    }

    fn finish_mail_thread_triage_preflight(
        &mut self,
        key: &MailTriageIntentKey,
        result: Result<MailThread, String>,
        cx: &mut Context<Self>,
    ) {
        let thread_id = key.thread_id.as_str();
        let control = key.control;
        if !self.mail_triage_preflights.remove(key) {
            return;
        }
        match result {
            Ok(thread) if thread.id == thread_id => {
                self.apply_loaded_mail_triage_preflight(thread, control, cx);
            }
            Ok(_) => {
                self.report_mail_error("Mail thread response did not match the request");
                self.finish_failed_mail_triage_preflight(thread_id, control, cx);
                cx.notify();
            }
            Err(message) => {
                self.handle_mail_workspace_error(message, cx);
                self.finish_failed_mail_triage_preflight(thread_id, control, cx);
            }
        }
    }

    fn apply_loaded_mail_triage_preflight(
        &mut self,
        mut thread: MailThread,
        control: MailTriageControl,
        cx: &mut Context<Self>,
    ) {
        let thread_id = thread.id.clone();
        if self.mail_loading_thread_id.as_deref() == Some(thread_id.as_str()) {
            self.mail_loading_thread_id = None;
        }
        self.apply_pending_mail_triage_to_thread(&mut thread);
        self.apply_mail_thread_summary_state(&thread);
        self.mail_thread_cache.insert(thread_id.clone(), thread);
        self.mail_thread_detail_cache.remove(&thread_id);
        self.ensure_mail_active_thread_detail_cached();
        self.sync_mail_open_thread_body_list_state();
        self.sync_mail_list_rows();
        self.continue_mail_triage_after_preflight(&thread_id, control, cx);
    }

    fn continue_mail_triage_after_preflight(
        &mut self,
        thread_id: &str,
        control: MailTriageControl,
        cx: &mut Context<Self>,
    ) {
        if let Some(change) = self.mail_toggled_triage_change(thread_id, control) {
            if !self.start_mail_thread_triage(thread_id.to_string(), control, change, cx) {
                self.clear_automatic_mark_read_suppression(thread_id, control);
                self.retry_deferred_mail_thread_read(thread_id, cx);
            }
        } else {
            self.clear_automatic_mark_read_suppression(thread_id, control);
            self.retry_deferred_mail_thread_read(thread_id, cx);
        }
        if control == MailTriageControl::Star
            && self.mail_open_thread_id.as_deref() == Some(thread_id)
        {
            self.mark_mail_thread_read_on_open(thread_id, cx);
        }
        self.reconcile_mail_triage_workspace_in_background(cx);
        cx.notify();
    }

    fn finish_failed_mail_triage_preflight(
        &mut self,
        thread_id: &str,
        control: MailTriageControl,
        cx: &mut Context<Self>,
    ) {
        self.clear_automatic_mark_read_suppression(thread_id, control);
        self.retry_deferred_mail_thread_read(thread_id, cx);
        self.reconcile_mail_triage_workspace_in_background(cx);
    }
}
