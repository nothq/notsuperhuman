use super::super::MailListSource;
use crate::ui::{HashMap, MailMessage, MailSearchQuery, MailSearchSnippet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MailSearchOrigin {
    Interactive,
    StarredView,
    SplitView,
    OtherView,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MailSearchRequest {
    pub(crate) account_generation: u64,
    pub(crate) account_id: String,
    pub(crate) selected_source: MailListSource,
    pub(crate) generation: u64,
    pub(crate) query: MailSearchQuery,
    pub(crate) position: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct MailSearchSession {
    pub(crate) origin: MailSearchOrigin,
    pub(crate) generation: u64,
    pub(crate) raw_query: String,
    pub(crate) input_focused: bool,
    pub(crate) submitted_query: Option<MailSearchQuery>,
    pub(crate) messages: Vec<MailMessage>,
    pub(crate) snippets: HashMap<String, MailSearchSnippet>,
    pub(crate) thread_message_counts: HashMap<String, usize>,
    pub(crate) next_position: Option<usize>,
    pub(crate) in_flight: Option<MailSearchRequest>,
    pub(crate) previous_selected_thread_id: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) enum MailSearchState {
    Closed { generation: u64 },
    Open(Box<MailSearchSession>),
}

impl Default for MailSearchState {
    fn default() -> Self {
        Self::Closed { generation: 0 }
    }
}

impl MailSearchState {
    pub(crate) fn generation(&self) -> u64 {
        match self {
            Self::Closed { generation } => *generation,
            Self::Open(session) => session.generation,
        }
    }

    pub(crate) fn session(&self) -> Option<&MailSearchSession> {
        match self {
            Self::Closed { .. } => None,
            Self::Open(session) => Some(session.as_ref()),
        }
    }

    pub(crate) fn session_mut(&mut self) -> Option<&mut MailSearchSession> {
        match self {
            Self::Closed { .. } => None,
            Self::Open(session) => Some(session.as_mut()),
        }
    }
}
