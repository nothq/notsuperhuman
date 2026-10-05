use super::super::{MailAccountViewState, MailSearchState, SurfaceState};
use crate::ui::ListOffset;

impl SurfaceState {
    pub(super) fn capture_current_mail_account_view(&mut self) {
        let Some(account_id) = self
            .mail_workspace()
            .map(|workspace| workspace.account_id.clone())
        else {
            return;
        };
        let view = self.capture_mail_account_view();
        self.mail_account_view_states.insert(account_id, view);
    }

    fn capture_mail_account_view(&self) -> MailAccountViewState {
        MailAccountViewState {
            selected_mailbox_id: self.mail_selected_tab_id.clone(),
            list_source: self.mail_list_source.clone(),
            selected_thread_id: self.mail_selected_thread_id.clone(),
            open_thread_id: self.mail_open_thread_id.clone(),
            list_scroll: self.mail_list_state.logical_scroll_top(),
            search_list_scroll: self.mail_search_list_state.logical_scroll_top(),
            search_state: sanitized_mail_search_state(self.mail_search_state.clone()),
        }
    }

    pub(super) fn invalidate_mail_account_async_state(&mut self) {
        self.clear_mail_contact_histories();
        self.mail_loading_message_page = None;
        self.mail_workspace_refresh_generation =
            self.mail_workspace_refresh_generation.wrapping_add(1);
        self.mail_workspace_actions_in_flight = 0;
        self.mail_loading_thread_id = None;
        self.mail_identity_loading = false;
        self.mail_mark_read_after_triage.clear();
        self.mail_mark_read_suppressed.clear();
        self.mail_triage_preflights.clear();
        self.mail_triage_intents.clear();
        self.mail_triage_summaries.clear();
        self.mail_triage_reconciled_generation = self.mail_triage_generation;
        if let Some(session) = self.mail_search_state.session_mut() {
            session.generation = session.generation.wrapping_add(1);
            session.in_flight = None;
            session.input_focused = false;
        }
        self.clear_mail_remote_images();
    }

    pub(super) fn restore_mail_account_view(&mut self, account_id: &str) {
        let (selected_mailbox_id, default_thread_id) = {
            let workspace = self
                .mail_workspace()
                .expect("loaded Mail account must expose a workspace");
            (
                workspace.selected_mailbox_id.clone(),
                workspace
                    .messages
                    .first()
                    .map(|message| message.thread_id.clone()),
            )
        };
        self.mail_selected_tab_id = selected_mailbox_id;
        self.mail_list_source =
            super::super::MailListSource::Mailbox(self.mail_selected_tab_id.clone());
        self.mail_search_state = MailSearchState::default();
        self.mail_selected_thread_id = default_thread_id;
        self.reset_mail_open_thread_read_session();
        self.mail_open_thread_id = None;
        let view = self.mail_account_view_states.get(account_id).cloned();
        if let Some(view) = view.as_ref() {
            self.restore_mail_account_selection(view);
        }
        self.sync_mail_list_rows();
        if let Some(view) = view {
            restore_list_scroll(&self.mail_list_state, view.list_scroll);
            restore_list_scroll(&self.mail_search_list_state, view.search_list_scroll);
        } else {
            restore_list_scroll(&self.mail_list_state, ListOffset::default());
            restore_list_scroll(&self.mail_search_list_state, ListOffset::default());
        }
    }

    fn restore_mail_account_selection(&mut self, view: &MailAccountViewState) {
        let restored_source = match &view.list_source {
            super::super::MailListSource::Split(id)
                if self.mail_split_preferences_supported
                    && self
                        .mail_split_definitions
                        .iter()
                        .any(|split| split.id() == id && split.is_enabled()) =>
            {
                view.list_source.clone()
            }
            super::super::MailListSource::Other if self.mail_split_preferences_supported => {
                view.list_source.clone()
            }
            super::super::MailListSource::Mailbox(_) | super::super::MailListSource::Starred => {
                view.list_source.clone()
            }
            super::super::MailListSource::Split(_) | super::super::MailListSource::Other => {
                super::super::MailListSource::Mailbox(self.mail_selected_tab_id.clone())
            }
        };
        self.mail_search_state = if restored_source == view.list_source {
            sanitized_mail_search_state(view.search_state.clone())
        } else {
            MailSearchState::default()
        };
        self.mail_list_source = restored_source;
        let selected_thread_id = {
            let visible_messages = self.mail_visible_messages();
            view.selected_thread_id
                .clone()
                .filter(|thread_id| {
                    visible_messages
                        .iter()
                        .any(|message| message.thread_id == *thread_id)
                })
                .or_else(|| {
                    visible_messages
                        .first()
                        .map(|message| message.thread_id.clone())
                })
        };
        self.mail_selected_thread_id = selected_thread_id;
        self.mail_open_thread_id = view.open_thread_id.clone().filter(|thread_id| {
            self.mail_selected_thread_id.as_deref() == Some(thread_id.as_str())
        });
    }
}

fn sanitized_mail_search_state(mut state: MailSearchState) -> MailSearchState {
    match &mut state {
        MailSearchState::Closed { generation } => *generation = generation.wrapping_add(1),
        MailSearchState::Open(session) => {
            session.generation = session.generation.wrapping_add(1);
            session.in_flight = None;
            session.input_focused = false;
        }
    }
    state
}

fn restore_list_scroll(list_state: &crate::ui::ListState, scroll: ListOffset) {
    if list_state.item_count() == 0 {
        return;
    }
    list_state.scroll_to(ListOffset {
        item_ix: scroll.item_ix.min(list_state.item_count() - 1),
        offset_in_item: scroll.offset_in_item,
    });
}
