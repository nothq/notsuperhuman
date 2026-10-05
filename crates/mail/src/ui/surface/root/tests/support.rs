use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::model::MailboxRights;
use crate::ui::{
    MailAddress, MailAttachment, MailDraftRequest, MailIdentity, MailMessage, MailMessagePage,
    MailSearchPage, MailSearchQuery, MailSendResult, MailThread, MailUploadFile, MailWorkspace,
    Mailbox,
};

struct TestMailWorkspaceApi {
    mode: TestMailWorkspaceApiMode,
    workspaces_by_mailbox: HashMap<String, MailWorkspace>,
    message_pages_by_position: HashMap<(String, usize), MailMessagePage>,
    page_requests: PageRequests,
}

struct TestMailAccountLoaderApi;

impl crate::model::MailAccountLoaderApi for TestMailAccountLoaderApi {
    fn load_mail_account(
        &self,
        _account_id: &str,
    ) -> Result<crate::model::MailAccountWorkspace, crate::model::MailAccountLoadError> {
        Err(crate::model::MailAccountLoadError::Failed(
            "unused test API method".to_string(),
        ))
    }
}

pub(crate) type PageRequests = Arc<Mutex<Vec<(String, usize)>>>;

#[derive(Clone, Copy)]
enum TestMailWorkspaceApiMode {
    Cached,
    StaleCached,
    Uncached,
}

impl crate::model::MailWorkspaceApi for TestMailWorkspaceApi {
    fn load_mail_workspace(&self, mailbox_id: &str) -> Result<MailWorkspace, String> {
        Ok(self
            .workspaces_by_mailbox
            .get(mailbox_id)
            .cloned()
            .unwrap_or_else(|| test_workspace(mailbox_id)))
    }

    fn load_cached_mail_workspace(
        &self,
        mailbox_id: &str,
    ) -> Result<Option<MailWorkspace>, String> {
        Ok(match self.mode {
            TestMailWorkspaceApiMode::Cached => Some(test_workspace(mailbox_id)),
            TestMailWorkspaceApiMode::StaleCached => Some(stale_test_workspace(mailbox_id)),
            TestMailWorkspaceApiMode::Uncached => None,
        })
    }

    fn refresh_mail_workspace(&self, mailbox_id: &str) -> Result<MailWorkspace, String> {
        Ok(test_workspace(mailbox_id))
    }

    fn archive_mail_thread(
        &self,
        _mailbox_id: &str,
        _thread_id: &str,
    ) -> Result<MailWorkspace, String> {
        unused_test_api_method()
    }

    fn move_mail_thread(
        &self,
        _mailbox_id: &str,
        _thread_id: &str,
        _destination_mailbox_id: &str,
    ) -> Result<MailWorkspace, String> {
        unused_test_api_method()
    }

    fn snooze_mail_thread(
        &self,
        _mailbox_id: &str,
        _thread_id: &str,
        _remind_at: &str,
    ) -> Result<MailWorkspace, String> {
        unused_test_api_method()
    }

    fn load_mail_message_detail(
        &self,
        _mailbox_id: &str,
        _message_id: &str,
    ) -> Result<MailMessage, String> {
        unused_test_api_method()
    }

    fn load_mail_message_page(
        &self,
        mailbox_id: &str,
        position: usize,
    ) -> Result<MailMessagePage, String> {
        self.page_requests
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push((mailbox_id.to_string(), position));
        let page = self
            .message_pages_by_position
            .get(&(mailbox_id.to_string(), position));
        Ok(MailMessagePage {
            mailbox_id: mailbox_id.to_string(),
            position,
            messages: page.map(|page| page.messages.clone()).unwrap_or_default(),
            next_position: page.and_then(|page| page.next_position),
        })
    }

    fn search_mail_messages(
        &self,
        query: &MailSearchQuery,
        position: usize,
    ) -> Result<MailSearchPage, String> {
        let mut messages = if self.workspaces_by_mailbox.is_empty() {
            test_workspace("inbox").messages
        } else {
            self.workspaces_by_mailbox
                .values()
                .flat_map(|workspace| workspace.messages.iter().cloned())
                .collect()
        };
        messages.retain(|message| message.matches_search_query(query));
        messages.sort_by(|left, right| {
            right
                .received_at
                .cmp(&left.received_at)
                .then_with(|| left.id.cmp(&right.id))
        });
        messages.dedup_by(|left, right| left.id == right.id);
        Ok(MailSearchPage {
            position,
            messages: messages.into_iter().skip(position).collect(),
            next_position: None,
            snippets: HashMap::new(),
            thread_message_counts: HashMap::new(),
        })
    }

    fn load_mail_thread(&self, _thread_id: &str) -> Result<MailThread, String> {
        unused_test_api_method()
    }

    fn load_mail_identity(&self) -> Result<MailIdentity, String> {
        Ok(MailIdentity::default())
    }

    fn create_mail_draft(&self, request: MailDraftRequest) -> Result<MailMessage, String> {
        Ok(test_draft_message(request))
    }

    fn update_mail_draft(
        &self,
        _draft_id: &str,
        _request: MailDraftRequest,
    ) -> Result<MailMessage, String> {
        unused_test_api_method()
    }

    fn delete_mail_draft(&self, _draft_id: &str) -> Result<(), String> {
        unused_test_api_method()
    }

    fn upload_mail_files(
        &self,
        _files: Vec<MailUploadFile>,
    ) -> Result<Vec<MailAttachment>, String> {
        unused_test_api_method()
    }

    fn send_mail_draft(
        &self,
        _draft_id: &str,
        _identity_id: &str,
        _to: Vec<MailAddress>,
        _cc: Vec<MailAddress>,
    ) -> Result<MailSendResult, String> {
        unused_test_api_method()
    }
}

pub(crate) fn test_api() -> Arc<dyn crate::model::MailWorkspaceApi> {
    Arc::new(TestMailWorkspaceApi {
        mode: TestMailWorkspaceApiMode::Cached,
        workspaces_by_mailbox: HashMap::new(),
        message_pages_by_position: HashMap::new(),
        page_requests: Arc::new(Mutex::new(Vec::new())),
    })
}

pub(crate) fn test_account_loader() -> Arc<dyn crate::model::MailAccountLoaderApi> {
    Arc::new(TestMailAccountLoaderApi)
}
pub(crate) fn stale_cached_test_api() -> Arc<dyn crate::model::MailWorkspaceApi> {
    Arc::new(TestMailWorkspaceApi {
        mode: TestMailWorkspaceApiMode::StaleCached,
        workspaces_by_mailbox: HashMap::new(),
        message_pages_by_position: HashMap::new(),
        page_requests: Arc::new(Mutex::new(Vec::new())),
    })
}

pub(crate) fn uncached_test_api() -> Arc<dyn crate::model::MailWorkspaceApi> {
    Arc::new(TestMailWorkspaceApi {
        mode: TestMailWorkspaceApiMode::Uncached,
        workspaces_by_mailbox: HashMap::new(),
        message_pages_by_position: HashMap::new(),
        page_requests: Arc::new(Mutex::new(Vec::new())),
    })
}

pub(crate) fn paged_test_api(
    workspace: MailWorkspace,
    pages: Vec<MailMessagePage>,
) -> (Arc<dyn crate::model::MailWorkspaceApi>, PageRequests) {
    let page_requests = Arc::new(Mutex::new(Vec::new()));
    let api = TestMailWorkspaceApi {
        mode: TestMailWorkspaceApiMode::Cached,
        workspaces_by_mailbox: HashMap::from([(workspace.selected_mailbox_id.clone(), workspace)]),
        message_pages_by_position: pages
            .into_iter()
            .map(|page| ((page.mailbox_id.clone(), page.position), page))
            .collect(),
        page_requests: page_requests.clone(),
    };
    (Arc::new(api), page_requests)
}

fn unused_test_api_method<T>() -> Result<T, String> {
    Err("unused test API method".to_string())
}

pub(crate) fn test_workspace(mailbox_id: &str) -> MailWorkspace {
    MailWorkspace {
        account_id: "account".to_string(),
        mailbox_email: Some("notsuperhuman@example.test".to_string()),
        display_name: "notsuperhuman".to_string(),
        owner_username: "notsuperhuman".to_string(),
        selected_mailbox_id: mailbox_id.to_string(),
        mailboxes: vec![
            test_mailbox("inbox", "Inbox"),
            test_mailbox("drafts", "Drafts"),
            test_mailbox("sent", "Sent"),
        ],
        messages: vec![test_message(mailbox_id)],
        message_next_position: None,
    }
}

pub(crate) fn stale_test_workspace(mailbox_id: &str) -> MailWorkspace {
    let mut workspace = test_workspace("inbox");
    workspace.selected_mailbox_id = mailbox_id.to_string();
    workspace
}

fn test_mailbox(id: &str, name: &str) -> Mailbox {
    Mailbox {
        id: id.to_string(),
        name: name.to_string(),
        role: Some(id.to_string()),
        unread_emails: 0,
        total_emails: 1,
        rights: full_mailbox_rights(),
    }
}

fn full_mailbox_rights() -> MailboxRights {
    MailboxRights {
        may_read_items: true,
        may_add_items: true,
        may_remove_items: true,
        may_set_seen: true,
        may_set_keywords: true,
        may_create_child: true,
        may_rename: true,
        may_delete: true,
        may_submit: true,
    }
}

fn test_message(mailbox_id: &str) -> MailMessage {
    MailMessage {
        id: format!("{mailbox_id}-message"),
        thread_id: format!("{mailbox_id}-thread"),
        subject: format!("{mailbox_id} subject"),
        preview: String::new(),
        received_at: "2026-05-19T00:00:00Z".to_string(),
        from: Vec::new(),
        to: Vec::new(),
        mailbox_ids: vec![mailbox_id.to_string()],
        has_attachment: false,
        is_unread: false,
        is_starred: false,
        body_text: String::new(),
        attachments: Vec::new(),
        ..Default::default()
    }
}

pub(crate) fn test_message_at(mailbox_id: &str, index: usize) -> MailMessage {
    let mut message = test_message(mailbox_id);
    message.id = format!("{mailbox_id}-message-{index}");
    message.thread_id = format!("{mailbox_id}-thread-{index}");
    message.subject = format!("{mailbox_id} subject {index}");
    message.received_at = format!("2026-05-{:02}T00:00:00Z", 19usize.saturating_sub(index));
    message
}

fn test_draft_message(request: MailDraftRequest) -> MailMessage {
    MailMessage {
        id: "draft-new".to_string(),
        thread_id: "draft-new-thread".to_string(),
        subject: request.subject,
        preview: String::new(),
        received_at: String::new(),
        from: vec![request.from],
        to: request.to,
        cc: request.cc,
        mailbox_ids: vec!["drafts".to_string()],
        has_attachment: false,
        is_unread: false,
        is_starred: false,
        is_draft: true,
        body_text: request.body_text,
        attachments: request.attachments,
        in_reply_to: request.in_reply_to,
        references: request.references,
        ..Default::default()
    }
}
