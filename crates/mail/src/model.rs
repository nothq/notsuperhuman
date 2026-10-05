mod api;
mod contact_history;
mod download;
mod search;
mod splits;
mod workspace;

pub use api::{
    MailAccountLoadError, MailAccountLoaderApi, MailAccountWorkspace, MailBootstrapApi,
    MailLocalFileApi, MailSignInRequest, MailWorkspaceApi, MailWorkspaceBootstrap,
};
pub use contact_history::{
    MailContactAddress, MailContactAddressError, MailContactHistoryEntry, MailContactHistoryPage,
    MailContactHistoryRequest, MAIL_CONTACT_HISTORY_LIMIT,
};
pub use download::{
    MailAttachmentDownloadDestination, MailAttachmentDownloadRequest, MailAttachmentDownloadResult,
    MailBlobId, MailDownloadFileName,
};
pub use search::{
    MailSearchAge, MailSearchAgeUnit, MailSearchDate, MailSearchExpression,
    MailSearchMailboxSelector, MailSearchPredicate, MailSearchQuery, MailSearchQueryError,
    MailSearchQueryErrorKind, MailSearchSnippet, MailSearchSnippetSegment, MailSearchSnippetText,
    MailSearchText,
};
pub use splits::{
    MailSplitDefinition, MailSplitDraft, MailSplitEnabled, MailSplitId, MailSplitMove,
    MailSplitMutation, MailSplitOrder, MailSplitValidationError, MAIL_SPLIT_MAX_COUNT,
    MAIL_SPLIT_NAME_MAX_BYTES, MAIL_SPLIT_NAME_MAX_CHARS, MAIL_SPLIT_QUERY_MAX_BYTES,
};
pub use workspace::{
    MailAccountInfo, MailAddress, MailAttachment, MailBodySource, MailDisplayBody,
    MailDraftRequest, MailIdentity, MailMessage, MailMessagePage, MailReadState, MailSearchPage,
    MailSendResult, MailStarState, MailThread, MailThreadTriageChange, MailThreadTriageRequest,
    MailThreadTriageResult, MailUploadFile, MailWorkspace, Mailbox, MailboxRights,
};
