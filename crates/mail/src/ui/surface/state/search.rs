use super::super::{
    mail_message_involves_contact, MailListSource, MailSearchOrigin, MailSearchSession,
    MailSearchState, MailThreadDetail,
};
use super::mail_search_thread_detail_from_summary;
use crate::ui::{Arc, Context, MailMessage, MailSearchQuery, SurfaceState};

mod input;
mod loading;
mod views;

impl SurfaceState {
    pub(crate) fn mail_search_open(&self) -> bool {
        self.mail_search_state
            .session()
            .is_some_and(|session| session.origin == MailSearchOrigin::Interactive)
    }

    pub(crate) fn mail_search_session(&self) -> Option<&MailSearchSession> {
        self.mail_search_state.session()
    }

    pub(crate) fn mail_search_submitted_query(&self) -> Option<&MailSearchQuery> {
        self.mail_search_session()?.submitted_query.as_ref()
    }

    pub(crate) fn mail_search_context_thread_detail(&self) -> Option<Arc<MailThreadDetail>> {
        let session = self.mail_search_session()?;
        if session.raw_query.is_empty() {
            return None;
        }
        let workspace = self.mail_workspace()?;
        if session.submitted_query.is_some() {
            let thread_id = self.mail_selected_thread_id.as_deref()?;
            return session
                .messages
                .iter()
                .find(|message| message.thread_id == thread_id)
                .map(|message| {
                    Arc::new(mail_search_thread_detail_from_summary(workspace, message))
                });
        }
        let contact = self.mail_search_contacts().into_iter().next()?;
        workspace
            .messages
            .iter()
            .find(|message| mail_message_involves_contact(message, contact.email.as_str()))
            .map(|message| Arc::new(mail_search_thread_detail_from_summary(workspace, message)))
    }

    pub(crate) fn mail_search_input_visible(&self) -> bool {
        self.mail_search_open()
            && self.mail_open_thread_id.is_none()
            && self.mail_compose_mode == crate::ui::MailComposeMode::Closed
    }

    pub(crate) fn mail_search_input_focused(&self) -> bool {
        self.mail_search_session()
            .is_some_and(|session| session.input_focused)
            && self.mail_search_input_visible()
    }

    pub(crate) fn mail_visible_messages(&self) -> &[MailMessage] {
        if let Some(session) = self.mail_search_session() {
            return session.messages.as_slice();
        }
        self.mail_workspace()
            .map(|workspace| workspace.messages.as_slice())
            .unwrap_or_default()
    }

    pub(crate) fn open_mail_search(&mut self, cx: &mut Context<Self>) -> bool {
        if self.mail_open_thread_id.is_some()
            || self.mail_compose_mode != crate::ui::MailComposeMode::Closed
        {
            return false;
        }
        if self.mail_search_open() {
            let session = self
                .mail_search_state
                .session_mut()
                .expect("open interactive search must have a session");
            session.input_focused = true;
            self.mail_shortcuts_focused = false;
            cx.notify();
            return true;
        }
        self.mail_list_source = MailListSource::Mailbox(self.mail_selected_tab_id.clone());
        let generation = self.mail_search_state.generation().wrapping_add(1);
        self.mail_search_state = MailSearchState::Open(Box::new(MailSearchSession {
            origin: MailSearchOrigin::Interactive,
            generation,
            raw_query: String::new(),
            input_focused: true,
            submitted_query: None,
            messages: Vec::new(),
            snippets: Default::default(),
            thread_message_counts: Default::default(),
            next_position: None,
            in_flight: None,
            previous_selected_thread_id: self.mail_selected_thread_id.take(),
        }));
        self.mail_shortcuts_focused = false;
        self.mail_hovered_thread_id = None;
        self.mail_action_palette = None;
        self.mail_error = None;
        self.mail_search_list_state.reset(0);
        self.sync_mail_list_rows();
        cx.notify();
        true
    }

    pub(crate) fn close_mail_search(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.mail_search_open() {
            return false;
        }
        let Some(session) = self.mail_search_state.session().cloned() else {
            return false;
        };
        self.mail_search_state = MailSearchState::Closed {
            generation: session.generation.wrapping_add(1),
        };
        self.mail_selected_thread_id = session
            .previous_selected_thread_id
            .filter(|thread_id| {
                self.mail_workspace().is_some_and(|workspace| {
                    workspace
                        .messages
                        .iter()
                        .any(|message| &message.thread_id == thread_id)
                })
            })
            .or_else(|| {
                self.mail_workspace()
                    .and_then(|workspace| workspace.messages.first())
                    .map(|message| message.thread_id.clone())
            });
        self.mail_hovered_thread_id = None;
        self.mail_action_palette = None;
        self.mail_error = None;
        self.mail_shortcuts_focused = true;
        self.sync_mail_list_rows();
        cx.notify();
        true
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn invalidate_mail_search(&mut self, restore_selection: bool) {
        let search_was_open = self.mail_search_state.session().is_some();
        let previous_selected_thread_id = self
            .mail_search_state
            .session()
            .and_then(|session| session.previous_selected_thread_id.clone());
        let generation = self.mail_search_state.generation().wrapping_add(1);
        self.mail_search_state = MailSearchState::Closed { generation };
        self.mail_search_list_state.reset(0);
        if search_was_open {
            self.mail_shortcuts_focused = true;
        }
        if search_was_open && restore_selection {
            self.mail_selected_thread_id = previous_selected_thread_id
                .filter(|thread_id| {
                    self.mail_workspace().is_some_and(|workspace| {
                        workspace
                            .messages
                            .iter()
                            .any(|message| &message.thread_id == thread_id)
                    })
                })
                .or_else(|| {
                    self.mail_workspace()
                        .and_then(|workspace| workspace.messages.first())
                        .map(|message| message.thread_id.clone())
                });
        }
    }

    pub(crate) fn set_mail_search_input(&mut self, value: String, cx: &mut Context<Self>) {
        let Some(session) = self.mail_search_state.session_mut() else {
            return;
        };
        if session.raw_query == value {
            return;
        }
        session.generation = session.generation.wrapping_add(1);
        session.raw_query = value;
        session.input_focused = true;
        session.submitted_query = None;
        session.messages.clear();
        session.snippets.clear();
        session.thread_message_counts.clear();
        session.next_position = None;
        session.in_flight = None;
        self.mail_selected_thread_id = None;
        self.mail_error = None;
        self.sync_mail_list_rows();
        cx.notify();
    }

    pub(crate) fn submit_mail_search(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(raw_query) = self
            .mail_search_session()
            .map(|session| session.raw_query.clone())
        else {
            return false;
        };
        let Ok(query) = MailSearchQuery::parse(raw_query) else {
            return false;
        };
        self.start_mail_search(query, cx);
        true
    }

    pub(crate) fn submit_mail_search_text(
        &mut self,
        query: String,
        cx: &mut Context<Self>,
    ) -> bool {
        let parsed = MailSearchQuery::parse(&query)
            .expect("mail search suggestion addresses must not be empty");
        self.start_mail_search(parsed, cx);
        if let Some(session) = self.mail_search_state.session_mut() {
            session.raw_query = query;
        }
        true
    }

    fn start_mail_search(&mut self, query: MailSearchQuery, cx: &mut Context<Self>) {
        let Some(session) = self.mail_search_state.session_mut() else {
            return;
        };
        session.generation = session.generation.wrapping_add(1);
        session.raw_query = query.as_str().to_string();
        session.input_focused = true;
        session.submitted_query = Some(query.clone());
        session.messages.clear();
        session.snippets.clear();
        session.thread_message_counts.clear();
        session.next_position = None;
        session.in_flight = None;
        self.mail_selected_thread_id = None;
        self.mail_hovered_thread_id = None;
        self.mail_error = None;
        self.sync_mail_list_rows();
        self.load_mail_search_page_in_background(query, 0, cx);
    }

    pub(crate) fn refresh_current_mail_search_results(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.mail_search_state.session_mut() else {
            return;
        };
        let Some(query) = session.submitted_query.clone() else {
            return;
        };
        session.generation = session.generation.wrapping_add(1);
        session.next_position = None;
        session.in_flight = None;
        self.load_mail_search_page_in_background(query, 0, cx);
    }
}
