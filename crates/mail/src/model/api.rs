use std::{path::PathBuf, sync::Arc};

use remote_image_model::RemoteImageData;

use crate::model::{
    MailAccountInfo, MailAddress, MailAttachment, MailAttachmentDownloadRequest,
    MailAttachmentDownloadResult, MailContactHistoryPage, MailContactHistoryRequest,
    MailDraftRequest, MailIdentity, MailMessage, MailMessagePage, MailSearchPage, MailSearchQuery,
    MailSendResult, MailSplitDefinition, MailSplitMutation, MailThread, MailThreadTriageRequest,
    MailThreadTriageResult, MailUploadFile, MailWorkspace,
};

pub trait MailBootstrapApi: Send + Sync + 'static {
    fn bootstrap_workspace(&self) -> Result<MailWorkspaceBootstrap, String>;

    /// Whether any account is signed in; without one the app opens on the
    /// sign-in screen instead of loading mail.
    fn has_mail_accounts(&self) -> bool {
        true
    }

    /// Adds an account and makes it the one the next bootstrap opens. Blocks
    /// until the sign-in finishes.
    fn sign_in(&self, _request: MailSignInRequest) -> Result<(), String> {
        Err("signing in is not available here".to_string())
    }
}

/// Supported ways to add an account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MailSignInRequest {
    Superhuman,
    Jmap { server: String, token: String },
}

pub trait MailLocalFileApi: Send + Sync + 'static {
    fn load_upload_files(&self, paths: Vec<PathBuf>) -> Result<Vec<MailUploadFile>, String>;

    fn default_download_directory(&self) -> Result<PathBuf, String> {
        Err("Mail Downloads directory access is not available".to_string())
    }
}

pub trait MailAccountLoaderApi: Send + Sync + 'static {
    fn load_mail_account(
        &self,
        account_id: &str,
    ) -> Result<MailAccountWorkspace, MailAccountLoadError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MailAccountLoadError {
    AccessRemoved { account_id: String },
    Failed(String),
}

pub struct MailAccountWorkspace {
    pub accounts: Vec<MailAccountInfo>,
    pub workspace: MailWorkspace,
    pub workspace_api: Arc<dyn MailWorkspaceApi>,
    pub needs_initial_refresh: bool,
}

impl Clone for MailAccountWorkspace {
    fn clone(&self) -> Self {
        Self {
            accounts: self.accounts.clone(),
            workspace: self.workspace.clone(),
            workspace_api: self.workspace_api.clone(),
            needs_initial_refresh: self.needs_initial_refresh,
        }
    }
}

pub struct MailWorkspaceBootstrap {
    pub accounts: Vec<MailAccountInfo>,
    pub workspace: MailWorkspace,
    pub workspace_api: Arc<dyn MailWorkspaceApi>,
    pub needs_initial_refresh: bool,
    account_loader: Arc<dyn MailAccountLoaderApi>,
}

impl MailWorkspaceBootstrap {
    pub fn new(
        accounts: Vec<MailAccountInfo>,
        workspace: MailWorkspace,
        workspace_api: Arc<dyn MailWorkspaceApi>,
        needs_initial_refresh: bool,
        account_loader: Arc<dyn MailAccountLoaderApi>,
    ) -> Self {
        Self {
            accounts,
            workspace,
            workspace_api,
            needs_initial_refresh,
            account_loader,
        }
    }

    pub fn load_account(
        &self,
        account_id: &str,
    ) -> Result<MailAccountWorkspace, MailAccountLoadError> {
        self.account_loader.load_mail_account(account_id)
    }
}

impl Clone for MailWorkspaceBootstrap {
    fn clone(&self) -> Self {
        Self {
            accounts: self.accounts.clone(),
            workspace: self.workspace.clone(),
            workspace_api: self.workspace_api.clone(),
            needs_initial_refresh: self.needs_initial_refresh,
            account_loader: self.account_loader.clone(),
        }
    }
}

pub trait MailWorkspaceApi: Send + Sync + 'static {
    fn recover_mail_account_access(
        &self,
    ) -> Result<Option<MailAccountWorkspace>, MailAccountLoadError> {
        Ok(None)
    }
    fn load_mail_workspace(&self, mailbox_id: &str) -> Result<MailWorkspace, String>;
    fn load_cached_mail_workspace(
        &self,
        _mailbox_id: &str,
    ) -> Result<Option<MailWorkspace>, String> {
        Ok(None)
    }
    fn refresh_mail_workspace(&self, mailbox_id: &str) -> Result<MailWorkspace, String> {
        self.load_mail_workspace(mailbox_id)
    }
    fn supports_mail_split_preferences(&self) -> bool {
        false
    }
    fn load_mail_split_definitions(&self) -> Result<Vec<MailSplitDefinition>, String> {
        Ok(Vec::new())
    }
    fn mutate_mail_split_definitions(
        &self,
        _mutation: MailSplitMutation,
    ) -> Result<Vec<MailSplitDefinition>, String> {
        Err("Mail split preferences are not supported".to_string())
    }
    fn archive_mail_thread(
        &self,
        mailbox_id: &str,
        thread_id: &str,
    ) -> Result<MailWorkspace, String>;
    fn move_mail_thread(
        &self,
        mailbox_id: &str,
        thread_id: &str,
        destination_mailbox_id: &str,
    ) -> Result<MailWorkspace, String>;
    fn snooze_mail_thread(
        &self,
        mailbox_id: &str,
        thread_id: &str,
        remind_at: &str,
    ) -> Result<MailWorkspace, String>;
    fn supports_mail_thread_triage(&self) -> bool {
        false
    }
    fn set_mail_thread_triage(
        &self,
        _request: MailThreadTriageRequest,
    ) -> Result<MailThreadTriageResult, String> {
        Err("mail thread triage is not supported".to_string())
    }
    fn load_mail_message_detail(
        &self,
        mailbox_id: &str,
        message_id: &str,
    ) -> Result<MailMessage, String>;
    fn load_mail_message_page(
        &self,
        mailbox_id: &str,
        position: usize,
    ) -> Result<MailMessagePage, String>;
    fn search_mail_messages(
        &self,
        query: &MailSearchQuery,
        position: usize,
    ) -> Result<MailSearchPage, String>;
    fn load_mail_thread(&self, thread_id: &str) -> Result<MailThread, String>;
    fn load_mail_contact_history(
        &self,
        _request: MailContactHistoryRequest,
    ) -> Result<MailContactHistoryPage, String> {
        Err("mail contact history is not supported".to_string())
    }
    fn load_mail_identity(&self) -> Result<MailIdentity, String>;
    fn create_mail_draft(&self, request: MailDraftRequest) -> Result<MailMessage, String>;
    fn update_mail_draft(
        &self,
        draft_id: &str,
        request: MailDraftRequest,
    ) -> Result<MailMessage, String>;
    fn delete_mail_draft(&self, draft_id: &str) -> Result<(), String>;
    fn upload_mail_files(&self, files: Vec<MailUploadFile>) -> Result<Vec<MailAttachment>, String>;
    fn download_mail_attachment(
        &self,
        _request: MailAttachmentDownloadRequest,
    ) -> Result<MailAttachmentDownloadResult, String> {
        Err("mail attachment downloads are not supported".to_string())
    }
    fn send_mail_draft(
        &self,
        draft_id: &str,
        identity_id: &str,
        to: Vec<MailAddress>,
        cc: Vec<MailAddress>,
    ) -> Result<MailSendResult, String>;
    fn load_remote_image(&self, url: &str) -> Result<Option<RemoteImageData>, String> {
        Err(format!("remote image loading is not available for {url}"))
    }
}
