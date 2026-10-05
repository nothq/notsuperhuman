use crate::model::{
    MailAttachmentDownloadRequest, MailAttachmentDownloadResult, MailDraftRequest, MailIdentity,
    MailMessage, MailMessagePage, MailSearchPage, MailSearchQuery, MailSendResult,
    MailSplitDefinition, MailSplitMutation, MailThread, MailThreadTriageRequest,
    MailThreadTriageResult, MailUploadFile, MailWorkspace,
};

use super::MailWorkspaceRuntime;

impl crate::model::MailWorkspaceApi for MailWorkspaceRuntime {
    fn load_mail_workspace(&self, mailbox_id: &str) -> Result<MailWorkspace, String> {
        self.client.load_mailbox_workspace(mailbox_id)
    }

    fn load_cached_mail_workspace(
        &self,
        mailbox_id: &str,
    ) -> Result<Option<MailWorkspace>, String> {
        self.cached_workspace(Some(mailbox_id))
    }

    fn refresh_mail_workspace(&self, mailbox_id: &str) -> Result<MailWorkspace, String> {
        self.refresh_workspace_summary(Some(mailbox_id))
    }

    fn supports_mail_split_preferences(&self) -> bool {
        true
    }

    fn load_mail_split_definitions(&self) -> Result<Vec<MailSplitDefinition>, String> {
        self.split_preferences.snapshot()
    }

    fn mutate_mail_split_definitions(
        &self,
        mutation: MailSplitMutation,
    ) -> Result<Vec<MailSplitDefinition>, String> {
        self.split_preferences.mutate(mutation)
    }

    fn archive_mail_thread(
        &self,
        mailbox_id: &str,
        thread_id: &str,
    ) -> Result<MailWorkspace, String> {
        self.client.archive_mail_thread(mailbox_id, thread_id)
    }

    fn move_mail_thread(
        &self,
        mailbox_id: &str,
        thread_id: &str,
        destination_mailbox_id: &str,
    ) -> Result<MailWorkspace, String> {
        self.client
            .move_mail_thread(mailbox_id, thread_id, destination_mailbox_id)
    }

    fn snooze_mail_thread(
        &self,
        mailbox_id: &str,
        thread_id: &str,
        remind_at: &str,
    ) -> Result<MailWorkspace, String> {
        self.client
            .snooze_mail_thread(mailbox_id, thread_id, remind_at)
    }

    fn supports_mail_thread_triage(&self) -> bool {
        true
    }

    fn set_mail_thread_triage(
        &self,
        request: MailThreadTriageRequest,
    ) -> Result<MailThreadTriageResult, String> {
        self.client.set_mail_thread_triage(request)
    }

    fn load_mail_message_detail(
        &self,
        mailbox_id: &str,
        message_id: &str,
    ) -> Result<MailMessage, String> {
        self.client.load_mail_message_detail(mailbox_id, message_id)
    }

    fn load_mail_message_page(
        &self,
        mailbox_id: &str,
        position: usize,
    ) -> Result<MailMessagePage, String> {
        self.client.load_mail_message_page(mailbox_id, position)
    }

    fn search_mail_messages(
        &self,
        query: &MailSearchQuery,
        position: usize,
    ) -> Result<MailSearchPage, String> {
        self.client.search_mail_messages(query, position)
    }

    fn load_mail_thread(&self, thread_id: &str) -> Result<MailThread, String> {
        self.client.load_mail_thread(thread_id)
    }

    fn load_mail_contact_history(
        &self,
        request: crate::model::MailContactHistoryRequest,
    ) -> Result<crate::model::MailContactHistoryPage, String> {
        self.client.load_mail_contact_history(request)
    }

    fn load_mail_identity(&self) -> Result<MailIdentity, String> {
        self.client.load_mail_identity()
    }

    fn create_mail_draft(&self, request: MailDraftRequest) -> Result<MailMessage, String> {
        self.client.create_mail_draft(request)
    }

    fn update_mail_draft(
        &self,
        draft_id: &str,
        request: MailDraftRequest,
    ) -> Result<MailMessage, String> {
        self.client.update_mail_draft(draft_id, request)
    }

    fn delete_mail_draft(&self, draft_id: &str) -> Result<(), String> {
        self.client.delete_mail_draft(draft_id)
    }

    fn upload_mail_files(
        &self,
        files: Vec<MailUploadFile>,
    ) -> Result<Vec<crate::model::MailAttachment>, String> {
        self.client.upload_mail_files(files)
    }

    fn download_mail_attachment(
        &self,
        request: MailAttachmentDownloadRequest,
    ) -> Result<MailAttachmentDownloadResult, String> {
        self.client.download_mail_attachment(request)
    }

    fn send_mail_draft(
        &self,
        draft_id: &str,
        identity_id: &str,
        to: Vec<crate::model::MailAddress>,
        cc: Vec<crate::model::MailAddress>,
    ) -> Result<MailSendResult, String> {
        self.client.send_mail_draft(draft_id, identity_id, to, cc)
    }

    fn load_remote_image(
        &self,
        url: &str,
    ) -> Result<Option<remote_image_model::RemoteImageData>, String> {
        self.client.load_remote_image(url)
    }
}
