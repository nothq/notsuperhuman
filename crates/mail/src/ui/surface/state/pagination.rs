use super::{mail_list_threads, Arc, Context, HashSet, MailMessagePage, SurfaceState};

const MAIL_LOAD_MORE_THRESHOLD: usize = 6;

impl SurfaceState {
    pub(crate) fn maybe_load_more_mail_messages(
        &mut self,
        visible_end: usize,
        row_count: usize,
        cx: &mut Context<Self>,
    ) {
        if row_count.saturating_sub(visible_end) > MAIL_LOAD_MORE_THRESHOLD {
            return;
        }
        if self.mail_search_submitted_query().is_some() {
            self.maybe_load_more_mail_search_results(cx);
            return;
        }
        if self.mail_search_open() {
            return;
        }
        let Some(workspace) = self.mail_workspace() else {
            return;
        };
        let Some(position) = workspace.message_next_position else {
            return;
        };
        let mailbox_id = workspace.selected_mailbox_id.clone();
        if self.mail_loading_message_page.is_some() {
            return;
        }
        self.load_mail_message_page_in_background(mailbox_id, position, cx);
    }

    pub(crate) fn load_mail_message_page_in_background(
        &mut self,
        mailbox_id: String,
        position: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace_api) = self.mail_workspace_api() else {
            self.report_mail_error("missing Mail workspace api");
            cx.notify();
            return;
        };
        let selected_source = self.mail_list_source.clone();
        self.mail_loading_message_page = Some((mailbox_id.clone(), position));
        cx.notify();
        self.spawn_background_task(
            (mailbox_id.clone(), position),
            cx,
            move |(mailbox_id, position)| {
                workspace_api.load_mail_message_page(&mailbox_id, position)
            },
            move |this, result, cx| {
                if this.mail_list_source != selected_source {
                    return;
                }
                let is_current_page = this.mail_loading_message_page.as_ref().is_some_and(
                    |(current_mailbox_id, current_position)| {
                        current_mailbox_id == &mailbox_id && *current_position == position
                    },
                );
                if !is_current_page {
                    return;
                }
                this.mail_loading_message_page = None;
                match result {
                    Ok(page) => this.apply_mail_message_page(page, cx),
                    Err(message) => {
                        this.handle_mail_workspace_error(message, cx);
                    }
                }
            },
        );
    }

    pub(crate) fn apply_mail_message_page(
        &mut self,
        mut page: MailMessagePage,
        cx: &mut Context<Self>,
    ) {
        self.apply_pending_mail_triage_to_messages(&mut page.messages);
        let search_open = self.mail_search_open();
        let current_page_matches = self.mail_workspace().is_some_and(|workspace| {
            workspace.selected_mailbox_id == page.mailbox_id
                && workspace.messages.len() == page.position
        }) && self.mail_selected_tab_id == page.mailbox_id;
        if !current_page_matches {
            return;
        }
        let Some(workspace) = self.mail_workspace_mut() else {
            return;
        };

        let old_len = workspace.messages.len();
        let mut seen_thread_ids = workspace
            .messages
            .iter()
            .map(|message| message.thread_id.clone())
            .collect::<HashSet<_>>();
        let mut seen_message_ids = workspace
            .messages
            .iter()
            .map(|message| message.id.clone())
            .collect::<HashSet<_>>();
        workspace
            .messages
            .extend(page.messages.into_iter().filter(|message| {
                seen_thread_ids.insert(message.thread_id.clone())
                    && seen_message_ids.insert(message.id.clone())
            }));
        let appended_len = workspace.messages.len() - old_len;
        workspace.message_next_position = if appended_len == 0 {
            None
        } else {
            page.next_position
        };
        let updated_list_threads = (!search_open).then(|| mail_list_threads(workspace));
        if let Some(list_threads) = updated_list_threads {
            self.mail_list_threads = Arc::from(list_threads);
        }
        if appended_len > 0 {
            self.mail_list_state.splice(old_len..old_len, appended_len);
        }
        self.mail_error = None;
        cx.notify();
    }
}
