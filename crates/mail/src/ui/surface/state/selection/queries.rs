use super::super::{
    mail_latest_thread_message, mail_list_thread_by_id, Arc, MailListThread, MailMessage,
    MailThread, MailThreadDetail, SurfaceState,
};

impl SurfaceState {
    pub(crate) fn mail_selected_summary(&self) -> Option<&MailMessage> {
        let thread_id = self.mail_selected_thread_id.as_deref()?;
        self.mail_visible_messages()
            .iter()
            .find(|message| message.thread_id == thread_id)
    }

    pub(crate) fn mail_selected_list_thread(&self) -> Option<MailListThread> {
        let thread_id = self.mail_selected_thread_id.as_deref()?;
        self.mail_list_threads
            .iter()
            .find(|thread| thread.id == thread_id)
            .cloned()
            .or_else(|| {
                self.mail_workspace()
                    .and_then(|workspace| mail_list_thread_by_id(workspace, thread_id))
            })
    }

    pub(crate) fn mail_open_thread_detail(&self) -> Option<Arc<MailThreadDetail>> {
        let thread_id = self.mail_open_thread_id.as_deref()?;
        self.mail_thread_detail_cache.get(thread_id).cloned()
    }

    pub(crate) fn mail_active_thread(&self) -> Option<MailThread> {
        let thread_id = self
            .mail_open_thread_id
            .as_deref()
            .or(self.mail_selected_thread_id.as_deref())?;
        if let Some(thread) = self.mail_thread_cache.get(thread_id) {
            return Some(thread.clone());
        }
        self.mail_visible_messages()
            .iter()
            .find(|message| message.thread_id == thread_id)
            .map(|message| MailThread {
                id: message.thread_id.clone(),
                messages: vec![message.clone()],
            })
    }

    pub(crate) fn mail_active_message(&self) -> Option<MailMessage> {
        self.mail_active_thread()
            .and_then(|thread| mail_latest_thread_message(&thread).cloned())
            .or_else(|| self.mail_selected_summary().cloned())
    }
}
