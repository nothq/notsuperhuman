use super::super::super::{Context, MailListSource, MailSearchState, MailWorkspace, SurfaceState};

fn cached_workspace_matches_mailbox(workspace: &MailWorkspace, mailbox_id: &str) -> bool {
    workspace.messages.iter().all(|message| {
        message.mailbox_ids.is_empty() || message.mailbox_ids.iter().any(|id| id == mailbox_id)
    })
}

impl SurfaceState {
    pub(crate) fn set_mail_list_source(&mut self, source: MailListSource, cx: &mut Context<Self>) {
        match source {
            MailListSource::Mailbox(mailbox_id) => self.set_mail_selected_tab(mailbox_id, cx),
            MailListSource::Starred => {
                self.mail_folder_drawer_open = false;
                self.open_mail_starred_view(cx);
            }
            source @ (MailListSource::Split(_) | MailListSource::Other) => {
                self.mail_folder_drawer_open = false;
                self.open_mail_split_view(source, cx);
            }
        }
    }

    pub(crate) fn set_mail_selected_tab(
        &mut self,
        tab_id: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        let tab_id = tab_id.into();
        self.mail_folder_drawer_open = false;
        let source = MailListSource::Mailbox(tab_id.clone());
        if self.mail_selected_tab_id == tab_id && self.mail_list_source == source {
            cx.notify();
            return;
        }
        self.reset_mail_selected_tab_state(tab_id.as_str(), source);
        if let Some(workspace) = self.mail_workspace() {
            if workspace.selected_mailbox_id == tab_id {
                self.mail_selected_thread_id = workspace
                    .messages
                    .first()
                    .map(|message| message.thread_id.clone());
                cx.notify();
                return;
            }
        }
        self.load_mail_selected_tab_workspace(tab_id, cx);
    }

    fn reset_mail_selected_tab_state(&mut self, tab_id: &str, source: MailListSource) {
        self.mail_list_source = source;
        if self.mail_search_state.session().is_some() {
            let generation = self.mail_search_state.generation().wrapping_add(1);
            self.mail_search_state = MailSearchState::Closed { generation };
            self.mail_search_list_state.reset(0);
        }
        self.mail_selected_tab_id = tab_id.to_string();
        self.mail_selected_thread_id = None;
        self.reset_mail_open_thread_read_session();
        self.mail_open_thread_id = None;
        self.mail_remote_image_discovery_thread_id = None;
        self.mail_hovered_thread_id = None;
        self.mail_hovered_message_card_id = None;
        self.mail_hovered_message_action = None;
        self.mail_hovered_history_message_id = None;
        self.mail_expanded_history_message_ids.clear();
        self.mail_expanded_message_header_ids.clear();
        self.mail_action_palette = None;
        self.mail_loading_message_page = None;
        self.mail_error = None;
    }

    fn load_mail_selected_tab_workspace(&mut self, tab_id: String, cx: &mut Context<Self>) {
        let Some(workspace_api) = self.mail_workspace_api() else {
            self.report_mail_error("missing Mail workspace api");
            cx.notify();
            return;
        };
        match workspace_api.load_cached_mail_workspace(&tab_id) {
            Ok(Some(workspace)) if cached_workspace_matches_mailbox(&workspace, &tab_id) => {
                self.apply_mail_workspace_refresh(workspace, cx);
                self.refresh_mail_mailbox_in_background(tab_id, cx);
            }
            Ok(Some(_)) | Ok(None) | Err(_) => match workspace_api.load_mail_workspace(&tab_id) {
                Ok(workspace) => self.apply_mail_workspace_refresh(workspace, cx),
                Err(message) => self.handle_mail_workspace_error(message, cx),
            },
        }
    }
}
