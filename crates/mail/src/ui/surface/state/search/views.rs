use super::super::super::{MailListSource, MailSearchOrigin, MailSearchSession, MailSearchState};
use crate::ui::{Context, MailSearchQuery, SurfaceState};

impl SurfaceState {
    pub(crate) fn mail_starred_view_open(&self) -> bool {
        self.mail_list_source == MailListSource::Starred
            && self
                .mail_search_state
                .session()
                .is_some_and(|session| session.origin == MailSearchOrigin::StarredView)
    }

    pub(crate) fn open_mail_starred_view(&mut self, cx: &mut Context<Self>) -> bool {
        if self.mail_compose_mode != crate::ui::MailComposeMode::Closed
        {
            return false;
        }
        if self.mail_starred_view_open() {
            self.refresh_current_mail_search_results(cx);
            return true;
        }
        let query = MailSearchQuery::parse("is:starred")
            .expect("the built-in Starred query must remain valid");
        let generation = self.mail_search_state.generation().wrapping_add(1);
        self.mail_list_source = MailListSource::Starred;
        self.mail_search_state = MailSearchState::Open(Box::new(MailSearchSession {
            origin: MailSearchOrigin::StarredView,
            generation,
            raw_query: String::new(),
            input_focused: false,
            submitted_query: Some(query.clone()),
            messages: Vec::new(),
            snippets: Default::default(),
            thread_message_counts: Default::default(),
            next_position: None,
            in_flight: None,
            previous_selected_thread_id: self.mail_selected_thread_id.take(),
        }));
        self.reset_mail_open_thread_read_session();
        self.mail_open_thread_id = None;
        self.mail_remote_image_discovery_thread_id = None;
        self.mail_hovered_thread_id = None;
        self.mail_action_palette = None;
        self.mail_error = None;
        self.mail_shortcuts_focused = true;
        self.mail_list_state.reset(0);
        self.sync_mail_list_rows();
        self.load_mail_search_page_in_background(query, 0, cx);
        true
    }

    pub(crate) fn open_mail_split_view(
        &mut self,
        source: MailListSource,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.mail_compose_mode != crate::ui::MailComposeMode::Closed
            || !self.mail_split_preferences_supported
            || !matches!(&source, MailListSource::Split(_) | MailListSource::Other)
        {
            return false;
        }
        let Some(inbox_mailbox_id) = self.prepare_mail_split_inbox(cx) else {
            return true;
        };
        let Some((origin, query)) = self.mail_split_view_query(&source, &inbox_mailbox_id, cx)
        else {
            return true;
        };
        self.open_mail_passive_query_view(source, origin, query, cx)
    }

    fn prepare_mail_split_inbox(&mut self, cx: &mut Context<Self>) -> Option<String> {
        let Some(inbox_mailbox_id) = self
            .mail_workspace()
            .and_then(|workspace| super::super::mail_mailbox_id_for_role(workspace, "inbox"))
        else {
            self.report_mail_error("Mail split views require a real Inbox mailbox");
            cx.notify();
            return None;
        };
        let backing_is_inbox = self
            .mail_workspace()
            .is_some_and(|workspace| workspace.selected_mailbox_id == inbox_mailbox_id);
        if !backing_is_inbox {
            self.set_mail_selected_tab(inbox_mailbox_id.clone(), cx);
            if self
                .mail_workspace()
                .is_none_or(|workspace| workspace.selected_mailbox_id != inbox_mailbox_id)
            {
                return None;
            }
        }
        Some(inbox_mailbox_id)
    }

    fn mail_split_view_query(
        &mut self,
        source: &MailListSource,
        inbox_mailbox_id: &str,
        cx: &mut Context<Self>,
    ) -> Option<(MailSearchOrigin, MailSearchQuery)> {
        match source {
            MailListSource::Split(id) => {
                let Some(split) = self
                    .mail_split_definitions
                    .iter()
                    .find(|split| split.id() == id && split.is_enabled())
                else {
                    self.report_mail_error("That Mail split is disabled or no longer exists");
                    cx.notify();
                    return None;
                };
                Some((
                    MailSearchOrigin::SplitView,
                    MailSearchQuery::inbox_scoped(inbox_mailbox_id, split.query()),
                ))
            }
            MailListSource::Other => Some((
                MailSearchOrigin::OtherView,
                MailSearchQuery::inbox_other(
                    inbox_mailbox_id,
                    self.mail_split_definitions
                        .iter()
                        .filter(|split| split.is_enabled())
                        .map(|split| split.query()),
                ),
            )),
            MailListSource::Mailbox(_) | MailListSource::Starred => None,
        }
    }

    fn open_mail_passive_query_view(
        &mut self,
        source: MailListSource,
        origin: MailSearchOrigin,
        query: MailSearchQuery,
        cx: &mut Context<Self>,
    ) -> bool {
        let previous_selected_thread_id = self
            .mail_search_state
            .session()
            .and_then(|session| session.previous_selected_thread_id.clone())
            .or_else(|| self.mail_selected_thread_id.take());
        let generation = self.mail_search_state.generation().wrapping_add(1);
        self.mail_list_source = source;
        self.mail_search_state = MailSearchState::Open(Box::new(MailSearchSession {
            origin,
            generation,
            raw_query: String::new(),
            input_focused: false,
            submitted_query: Some(query.clone()),
            messages: Vec::new(),
            snippets: Default::default(),
            thread_message_counts: Default::default(),
            next_position: None,
            in_flight: None,
            previous_selected_thread_id,
        }));
        self.mail_selected_thread_id = None;
        self.reset_mail_open_thread_read_session();
        self.mail_open_thread_id = None;
        self.mail_remote_image_discovery_thread_id = None;
        self.mail_hovered_thread_id = None;
        self.mail_action_palette = None;
        self.mail_error = None;
        self.mail_shortcuts_focused = true;
        self.mail_list_state.reset(0);
        self.sync_mail_list_rows();
        self.load_mail_search_page_in_background(query, 0, cx);
        true
    }
}
