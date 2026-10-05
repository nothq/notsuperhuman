use std::collections::HashMap;

use crate::ui::{
    MailAddress, MailAttachment, MailDraftRequest, MailIdentity, MailMessage, MailMessagePage,
    MailSearchPage, MailSearchQuery, MailSendResult, MailThread, MailUploadFile, MailWorkspace,
};

use super::TestMailWorkspaceApi;

impl crate::model::MailWorkspaceApi for TestMailWorkspaceApi {
    fn load_mail_workspace(&self, mailbox_id: &str) -> Result<MailWorkspace, String> {
        self.workspaces_by_mailbox
            .get(mailbox_id)
            .cloned()
            .ok_or_else(|| format!("missing test mailbox {mailbox_id}"))
    }

    fn load_cached_mail_workspace(
        &self,
        mailbox_id: &str,
    ) -> Result<Option<MailWorkspace>, String> {
        Ok(self.cached_workspaces_by_mailbox.get(mailbox_id).cloned())
    }

    fn refresh_mail_workspace(&self, mailbox_id: &str) -> Result<MailWorkspace, String> {
        self.load_mail_workspace(mailbox_id)
    }

    fn archive_mail_thread(
        &self,
        mailbox_id: &str,
        thread_id: &str,
    ) -> Result<MailWorkspace, String> {
        let mut workspace = self.load_mail_workspace(mailbox_id)?;
        workspace
            .messages
            .retain(|message| message.thread_id != thread_id);
        Ok(workspace)
    }

    fn move_mail_thread(
        &self,
        mailbox_id: &str,
        thread_id: &str,
        _destination_mailbox_id: &str,
    ) -> Result<MailWorkspace, String> {
        let mut workspace = self.load_mail_workspace(mailbox_id)?;
        workspace
            .messages
            .retain(|message| message.thread_id != thread_id);
        Ok(workspace)
    }

    fn snooze_mail_thread(
        &self,
        mailbox_id: &str,
        thread_id: &str,
        _remind_at: &str,
    ) -> Result<MailWorkspace, String> {
        let mut workspace = self.load_mail_workspace(mailbox_id)?;
        workspace
            .messages
            .retain(|message| message.thread_id != thread_id);
        Ok(workspace)
    }

    fn load_mail_message_detail(
        &self,
        mailbox_id: &str,
        message_id: &str,
    ) -> Result<MailMessage, String> {
        self.load_mail_workspace(mailbox_id)?
            .messages
            .into_iter()
            .find(|message| message.id == message_id)
            .ok_or_else(|| format!("missing test message {message_id}"))
    }

    fn load_mail_message_page(
        &self,
        mailbox_id: &str,
        position: usize,
    ) -> Result<MailMessagePage, String> {
        Ok(MailMessagePage {
            mailbox_id: mailbox_id.to_string(),
            position,
            messages: Vec::new(),
            next_position: None,
        })
    }

    fn search_mail_messages(
        &self,
        query: &MailSearchQuery,
        position: usize,
    ) -> Result<MailSearchPage, String> {
        let mut messages = self
            .workspaces_by_mailbox
            .values()
            .flat_map(|workspace| workspace.messages.iter().cloned())
            .filter(|message| message.matches_search_query(query))
            .collect::<Vec<_>>();
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

    fn load_mail_thread(&self, thread_id: &str) -> Result<MailThread, String> {
        self.threads_by_id
            .get(thread_id)
            .cloned()
            .ok_or_else(|| format!("missing test thread {thread_id}"))
    }

    fn load_mail_identity(&self) -> Result<MailIdentity, String> {
        Ok(self.identity.clone())
    }

    fn create_mail_draft(&self, _request: MailDraftRequest) -> Result<MailMessage, String> {
        Err("draft creation is not supported in mail tests".to_string())
    }

    fn update_mail_draft(
        &self,
        _draft_id: &str,
        _request: MailDraftRequest,
    ) -> Result<MailMessage, String> {
        Err("draft updates are not supported in mail tests".to_string())
    }

    fn delete_mail_draft(&self, _draft_id: &str) -> Result<(), String> {
        Err("draft deletion is not supported in mail tests".to_string())
    }

    fn upload_mail_files(
        &self,
        _files: Vec<MailUploadFile>,
    ) -> Result<Vec<MailAttachment>, String> {
        Err("file upload is not supported in mail tests".to_string())
    }

    fn send_mail_draft(
        &self,
        _draft_id: &str,
        _identity_id: &str,
        _to: Vec<MailAddress>,
        _cc: Vec<MailAddress>,
    ) -> Result<MailSendResult, String> {
        Err("sending is not supported in mail tests".to_string())
    }

    fn load_remote_image(
        &self,
        _url: &str,
    ) -> Result<Option<remote_image_model::RemoteImageData>, String> {
        Ok(None)
    }
}
