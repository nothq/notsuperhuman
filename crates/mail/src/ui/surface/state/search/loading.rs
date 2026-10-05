use super::super::super::MailSearchRequest;
use super::super::{mail_list_threads_for_messages, mail_search_list_threads, MailSearchOrigin};
use crate::ui::{Arc, Context, HashSet, MailSearchPage, MailSearchQuery, SurfaceState};

impl SurfaceState {
    pub(crate) fn maybe_load_more_mail_search_results(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.mail_search_session() else {
            return;
        };
        let Some(position) = session.next_position else {
            return;
        };
        if session.in_flight.is_some() {
            return;
        }
        let query = session
            .submitted_query
            .clone()
            .expect("search pagination requires a submitted query");
        self.load_mail_search_page_in_background(query, position, cx);
    }

    pub(crate) fn remove_mail_search_thread(&mut self, thread_id: &str) {
        if self.mail_search_session().is_none() {
            return;
        }
        let Some(row_index) = self
            .mail_list_threads
            .iter()
            .position(|thread| thread.id == thread_id)
        else {
            return;
        };
        let session = self
            .mail_search_state
            .session_mut()
            .expect("open mail search must have a session");
        let removed_message_ids = session
            .messages
            .iter()
            .filter(|message| message.thread_id == thread_id)
            .map(|message| message.id.clone())
            .collect::<Vec<_>>();
        session
            .messages
            .retain(|message| message.thread_id != thread_id);
        for message_id in removed_message_ids {
            session.snippets.remove(message_id.as_str());
        }
        session.thread_message_counts.remove(thread_id);
        self.rebuild_mail_search_list_threads();
        if self.mail_search_open() {
            self.mail_search_list_state
                .splice(row_index..row_index + 1, 0);
        } else {
            self.mail_list_state.splice(row_index..row_index + 1, 0);
        }
    }

    pub(super) fn load_mail_search_page_in_background(
        &mut self,
        query: MailSearchQuery,
        position: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace_api) = self.mail_workspace_api() else {
            self.report_mail_error("missing Mail workspace api");
            cx.notify();
            return;
        };
        let Some(account_id) = self
            .mail_workspace()
            .map(|workspace| workspace.account_id.clone())
        else {
            return;
        };
        let account_generation = self.mail_account_generation;
        let selected_source = self.mail_list_source.clone();
        let Some(session) = self.mail_search_state.session_mut() else {
            return;
        };
        let request = MailSearchRequest {
            account_generation,
            account_id,
            selected_source,
            generation: session.generation,
            query,
            position,
        };
        session.in_flight = Some(request.clone());
        cx.notify();
        self.spawn_background_task(
            request.clone(),
            cx,
            move |request| workspace_api.search_mail_messages(&request.query, request.position),
            move |this, result, cx| {
                if !this.mail_search_request_is_current(&request) {
                    return;
                }
                this.mail_search_state
                    .session_mut()
                    .expect("current mail search request requires an open session")
                    .in_flight = None;
                match result {
                    Ok(page) => this.apply_mail_search_page(request, page, cx),
                    Err(message) => {
                        this.handle_mail_workspace_error(message, cx);
                    }
                }
            },
        );
    }

    fn mail_search_request_is_current(&self, request: &MailSearchRequest) -> bool {
        self.mail_account_generation == request.account_generation
            && self.mail_list_source == request.selected_source
            && self
                .mail_workspace()
                .is_some_and(|workspace| workspace.account_id == request.account_id)
            && self
                .mail_search_session()
                .and_then(|session| session.in_flight.as_ref())
                == Some(request)
    }

    fn apply_mail_search_page(
        &mut self,
        request: MailSearchRequest,
        mut page: MailSearchPage,
        cx: &mut Context<Self>,
    ) {
        self.apply_pending_mail_triage_to_messages(&mut page.messages);
        let triage_thread_ids = self
            .mail_triage_summaries
            .keys()
            .chain(self.mail_triage_intents.keys().map(|key| &key.thread_id))
            .cloned()
            .collect::<HashSet<_>>();
        if !triage_thread_ids.is_empty() {
            page.messages.retain(|message| {
                !triage_thread_ids.contains(&message.thread_id)
                    || message.matches_search_query(&request.query)
            });
            let retained_message_ids = page
                .messages
                .iter()
                .map(|message| message.id.as_str())
                .collect::<HashSet<_>>();
            let retained_thread_ids = page
                .messages
                .iter()
                .map(|message| message.thread_id.as_str())
                .collect::<HashSet<_>>();
            page.snippets
                .retain(|message_id, _| retained_message_ids.contains(message_id.as_str()));
            page.thread_message_counts
                .retain(|thread_id, _| retained_thread_ids.contains(thread_id.as_str()));
        }
        let old_row_count = self.mail_list_threads.len();
        let page_position = page.position;
        if !self.merge_mail_search_page(&request, page) {
            return;
        }
        self.select_first_available_mail_search_thread();
        self.rebuild_mail_search_list_threads();
        let new_row_count = self.mail_list_threads.len();
        let list_state = if self.mail_search_open() {
            self.mail_search_list_state.clone()
        } else {
            self.mail_list_state.clone()
        };
        if page_position == 0 {
            list_state.reset(new_row_count);
        } else if new_row_count > old_row_count {
            list_state.splice(old_row_count..old_row_count, new_row_count - old_row_count);
        } else {
            list_state.remeasure();
        }
        self.mail_error = None;
        cx.notify();
    }

    fn merge_mail_search_page(
        &mut self,
        request: &MailSearchRequest,
        page: MailSearchPage,
    ) -> bool {
        let Some(session) = self.mail_search_state.session_mut() else {
            return false;
        };
        if session.generation != request.generation
            || session.submitted_query.as_ref() != Some(&request.query)
            || page.position != request.position
        {
            return false;
        }
        if page.position == 0 {
            session.messages = page.messages;
            session.snippets = page.snippets;
            session.thread_message_counts = page.thread_message_counts;
        } else {
            let mut seen_thread_ids = session
                .messages
                .iter()
                .map(|message| message.thread_id.clone())
                .collect::<HashSet<_>>();
            let mut seen_message_ids = session
                .messages
                .iter()
                .map(|message| message.id.clone())
                .collect::<HashSet<_>>();
            session
                .messages
                .extend(page.messages.into_iter().filter(|message| {
                    seen_thread_ids.insert(message.thread_id.clone())
                        && seen_message_ids.insert(message.id.clone())
                }));
            session.snippets.extend(page.snippets);
            session
                .thread_message_counts
                .extend(page.thread_message_counts);
        }
        session.next_position = page.next_position;
        true
    }

    fn select_first_available_mail_search_thread(&mut self) {
        let session = self
            .mail_search_session()
            .expect("applied mail search page requires an open session");
        self.mail_selected_thread_id = self
            .mail_selected_thread_id
            .clone()
            .filter(|thread_id| {
                session
                    .messages
                    .iter()
                    .any(|message| &message.thread_id == thread_id)
            })
            .or_else(|| {
                session
                    .messages
                    .first()
                    .map(|message| message.thread_id.clone())
            });
    }

    fn rebuild_mail_search_list_threads(&mut self) {
        let session = self
            .mail_search_session()
            .expect("mail search results require an open session");
        self.mail_list_threads = match session.origin {
            MailSearchOrigin::Interactive => {
                let workspace = self
                    .mail_workspace()
                    .expect("mail search results require a workspace");
                let query = session
                    .submitted_query
                    .as_ref()
                    .expect("mail search results require a submitted query");
                Arc::from(mail_search_list_threads(
                    workspace,
                    session.messages.as_slice(),
                    query,
                    &session.snippets,
                    &session.thread_message_counts,
                ))
            }
            MailSearchOrigin::StarredView
            | MailSearchOrigin::SplitView
            | MailSearchOrigin::OtherView => Arc::from(mail_list_threads_for_messages(
                session.messages.as_slice(),
                true,
            )),
        };
    }
}
