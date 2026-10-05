use super::super::super::super::{
    MailReadState, MailStarState, MailThread, MailThreadTriageChange, MailThreadTriageRequest,
    MailThreadTriageResult,
};

pub(super) struct MailTriageTaskOutcome {
    pub(super) error: Option<String>,
    pub(super) thread: Result<MailThread, String>,
}

pub(super) fn perform_mail_thread_triage(
    workspace_api: &dyn crate::model::MailWorkspaceApi,
    request: MailThreadTriageRequest,
) -> MailTriageTaskOutcome {
    match workspace_api.set_mail_thread_triage(request.clone()) {
        Ok(MailThreadTriageResult {
            workspace: _,
            thread,
        }) => MailTriageTaskOutcome {
            error: None,
            thread: Ok(thread),
        },
        Err(error) => MailTriageTaskOutcome {
            error: Some(error),
            thread: workspace_api.load_mail_thread(&request.thread_id),
        },
    }
}

pub(super) fn inverse_triage_change(change: MailThreadTriageChange) -> MailThreadTriageChange {
    match change {
        MailThreadTriageChange::Read(MailReadState::Read) => {
            MailThreadTriageChange::Read(MailReadState::Unread)
        }
        MailThreadTriageChange::Read(MailReadState::Unread) => {
            MailThreadTriageChange::Read(MailReadState::Read)
        }
        MailThreadTriageChange::Star(MailStarState::Starred) => {
            MailThreadTriageChange::Star(MailStarState::Unstarred)
        }
        MailThreadTriageChange::Star(MailStarState::Unstarred) => {
            MailThreadTriageChange::Star(MailStarState::Starred)
        }
    }
}
