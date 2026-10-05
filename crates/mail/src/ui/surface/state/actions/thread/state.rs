use super::super::super::super::{
    MailReadState, MailStarState, MailThread, MailThreadTriageChange, MailWorkspace, SurfaceState,
};

impl SurfaceState {
    pub(crate) fn apply_mail_triage_change(
        &mut self,
        thread_id: &str,
        change: MailThreadTriageChange,
    ) {
        if let Some(workspace) = self.mail_workspace_mut() {
            apply_triage_change_to_messages(&mut workspace.messages, thread_id, change);
        }
        if let Some(session) = self.mail_search_state.session_mut() {
            apply_triage_change_to_messages(&mut session.messages, thread_id, change);
            if let Some(query) = session.submitted_query.as_ref() {
                session
                    .messages
                    .retain(|message| message.matches_search_query(query));
            }
        }
        if let Some(thread) = self.mail_thread_cache.get_mut(thread_id) {
            apply_triage_change_to_messages(&mut thread.messages, thread_id, change);
        }
        self.mail_thread_detail_cache.remove(thread_id);
        self.ensure_mail_active_thread_detail_cached();
        self.sync_mail_open_thread_body_list_state();
        self.sync_mail_list_rows();
    }

    pub(crate) fn apply_pending_mail_triage_to_workspace(&self, workspace: &mut MailWorkspace) {
        self.apply_pending_mail_triage_to_messages(&mut workspace.messages);
    }

    pub(crate) fn apply_pending_mail_triage_to_messages(
        &self,
        messages: &mut [crate::ui::MailMessage],
    ) {
        for (thread_id, (unread, starred)) in &self.mail_triage_summaries {
            apply_thread_summary_state(Some(&mut *messages), thread_id, *unread, *starred);
        }
        for (key, intent) in &self.mail_triage_intents {
            apply_triage_change_to_messages(messages, &key.thread_id, intent.change);
        }
    }

    pub(crate) fn apply_pending_mail_triage_to_thread(&self, thread: &mut MailThread) {
        for (key, intent) in &self.mail_triage_intents {
            if key.thread_id == thread.id {
                apply_triage_change_to_messages(
                    &mut thread.messages,
                    &key.thread_id,
                    intent.change,
                );
            }
        }
    }

    pub(crate) fn apply_mail_thread_summary_state(&mut self, thread: &MailThread) {
        let unread = thread.messages.iter().any(|message| message.is_unread);
        let starred = thread.messages.iter().any(|message| message.is_starred);
        apply_thread_summary_state(
            self.mail_workspace_mut()
                .map(|workspace| workspace.messages.as_mut_slice()),
            thread.id.as_str(),
            unread,
            starred,
        );
        apply_thread_summary_state(
            self.mail_search_state
                .session_mut()
                .map(|session| session.messages.as_mut_slice()),
            thread.id.as_str(),
            unread,
            starred,
        );
    }
}

fn apply_triage_change_to_messages(
    messages: &mut [crate::ui::MailMessage],
    thread_id: &str,
    change: MailThreadTriageChange,
) {
    for message in messages
        .iter_mut()
        .filter(|message| message.thread_id == thread_id)
    {
        match change {
            MailThreadTriageChange::Read(state) => {
                message.is_unread = state == MailReadState::Unread;
            }
            MailThreadTriageChange::Star(state) => {
                message.is_starred = state == MailStarState::Starred;
            }
        }
    }
}

fn apply_thread_summary_state(
    messages: Option<&mut [crate::ui::MailMessage]>,
    thread_id: &str,
    unread: bool,
    starred: bool,
) {
    let Some(messages) = messages else {
        return;
    };
    for message in messages
        .iter_mut()
        .filter(|message| message.thread_id == thread_id)
    {
        message.is_unread = unread;
        message.is_starred = starred;
    }
}
