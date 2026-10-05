use super::super::MailTab;
use super::{
    mail_list_threads, mail_list_threads_for_messages, mail_search_list_threads,
    mail_tabs_with_splits, ListState, MailSearchOrigin, SurfaceState,
};
use std::sync::Arc;

impl SurfaceState {
    pub(crate) fn sync_mail_list_rows(&mut self) {
        let list_threads = if let Some(session) = self.mail_search_session() {
            match session.origin {
                MailSearchOrigin::Interactive => {
                    match (self.mail_workspace(), session.submitted_query.as_ref()) {
                        (Some(workspace), Some(query)) => mail_search_list_threads(
                            workspace,
                            session.messages.as_slice(),
                            query,
                            &session.snippets,
                            &session.thread_message_counts,
                        ),
                        _ => Vec::new(),
                    }
                }
                MailSearchOrigin::StarredView
                | MailSearchOrigin::SplitView
                | MailSearchOrigin::OtherView => {
                    mail_list_threads_for_messages(session.messages.as_slice(), true)
                }
            }
        } else {
            self.mail_workspace()
                .map(mail_list_threads)
                .unwrap_or_default()
        };
        self.mail_list_threads = Arc::from(list_threads);
        let list_state = if self.mail_search_open() {
            &self.mail_search_list_state
        } else {
            &self.mail_list_state
        };
        sync_mail_list_state(list_state, self.mail_list_threads.len());
    }

    pub(crate) fn mail_tabs(&self) -> Vec<MailTab> {
        self.mail_workspace()
            .map(|workspace| {
                mail_tabs_with_splits(
                    workspace,
                    &self.mail_split_definitions,
                    self.mail_split_preferences_supported,
                )
            })
            .unwrap_or_default()
    }
}

fn sync_mail_list_state(list_state: &ListState, row_count: usize) {
    if list_state.item_count() != row_count {
        list_state.reset(row_count);
    }
}
