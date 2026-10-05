use std::{
    collections::{HashMap, VecDeque},
    num::NonZeroUsize,
};

use crate::model::MailMessage;
use serde::Deserialize;
use serde_json::Value;

mod email;

pub(crate) use email::{
    JmapEmail, JmapEmailAddress, JmapEmailAttachment, JmapEmailBodyPart, JmapEmailBodyValue,
    JmapIdentity, JmapMailbox, JmapThread,
};

/// The signed-in person, as the JMAP session names them.
#[derive(Clone, Debug)]
pub(crate) struct MailResolveResponse {
    pub(crate) email: String,
    pub(crate) name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MailSessionResponse {
    pub username: String,
    #[serde(rename = "apiUrl")]
    pub api_url: String,
    #[serde(rename = "downloadUrl")]
    pub download_url: String,
    #[serde(rename = "uploadUrl")]
    pub upload_url: String,
    #[serde(default, rename = "primaryAccounts")]
    pub primary_accounts: HashMap<String, String>,
    #[serde(default)]
    pub accounts: HashMap<String, MailSessionAccount>,
    pub capabilities: HashMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MailSessionAccount {
    pub name: String,
    #[serde(rename = "isPersonal")]
    pub is_personal: bool,
    #[serde(rename = "isReadOnly")]
    pub is_read_only: bool,
    #[serde(rename = "accountCapabilities")]
    pub account_capabilities: HashMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapCoreCapabilities {
    #[serde(rename = "maxCallsInRequest")]
    pub(crate) max_calls_in_request: usize,
    #[serde(rename = "maxObjectsInGet")]
    pub(crate) max_objects_in_get: usize,
    #[serde(rename = "maxObjectsInSet")]
    pub(crate) max_objects_in_set: usize,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct JmapCoreLimits {
    pub(crate) max_calls_in_request: NonZeroUsize,
    pub(crate) max_objects_in_get: NonZeroUsize,
    pub(crate) max_objects_in_set: NonZeroUsize,
}

impl TryFrom<JmapCoreCapabilities> for JmapCoreLimits {
    type Error = String;

    fn try_from(capabilities: JmapCoreCapabilities) -> Result<Self, Self::Error> {
        Ok(Self {
            max_calls_in_request: NonZeroUsize::new(capabilities.max_calls_in_request).ok_or_else(
                || "Stalwart JMAP maxCallsInRequest must be greater than zero".to_string(),
            )?,
            max_objects_in_get: NonZeroUsize::new(capabilities.max_objects_in_get).ok_or_else(
                || "Stalwart JMAP maxObjectsInGet must be greater than zero".to_string(),
            )?,
            max_objects_in_set: NonZeroUsize::new(capabilities.max_objects_in_set).ok_or_else(
                || "Stalwart JMAP maxObjectsInSet must be greater than zero".to_string(),
            )?,
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapResponse {
    #[serde(rename = "methodResponses")]
    pub(crate) method_responses: Vec<(String, Value, String)>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapMailboxGetResponse {
    #[serde(rename = "accountId")]
    pub(crate) account_id: String,
    #[serde(default)]
    pub(crate) state: String,
    #[serde(default)]
    pub(crate) list: Vec<JmapMailbox>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapEmailQueryResponse {
    #[serde(rename = "accountId")]
    pub(crate) account_id: String,
    #[serde(default, rename = "queryState")]
    pub(crate) query_state: String,
    #[serde(default)]
    pub(crate) position: usize,
    #[serde(default)]
    pub(crate) ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapEmailGetResponse {
    #[serde(rename = "accountId")]
    pub(crate) account_id: String,
    #[serde(default)]
    pub(crate) list: Vec<JmapEmail>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapSearchSnippetGetResponse {
    #[serde(rename = "accountId")]
    pub(crate) account_id: String,
    #[serde(default)]
    pub(crate) list: Vec<JmapSearchSnippet>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapSearchSnippet {
    #[serde(rename = "emailId")]
    pub(crate) email_id: String,
    #[serde(default)]
    pub(crate) subject: Option<String>,
    #[serde(default)]
    pub(crate) preview: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapThreadGetResponse {
    #[serde(rename = "accountId")]
    pub(crate) account_id: String,
    #[serde(default)]
    pub(crate) list: Vec<JmapThread>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapIdentityGetResponse {
    #[serde(rename = "accountId")]
    pub(crate) account_id: String,
    #[serde(default)]
    pub(crate) list: Vec<JmapIdentity>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub(crate) struct JmapSetResponse<T> {
    #[serde(rename = "accountId")]
    pub(crate) account_id: String,
    #[serde(default)]
    pub(crate) created: HashMap<String, T>,
    #[serde(default)]
    pub(crate) updated: HashMap<String, Value>,
    #[serde(default, rename = "notCreated")]
    pub(crate) not_created: HashMap<String, JmapSetError>,
    #[serde(default, rename = "notUpdated")]
    pub(crate) not_updated: HashMap<String, JmapSetError>,
    #[serde(default, rename = "notDestroyed")]
    pub(crate) not_destroyed: HashMap<String, JmapSetError>,
}

pub(crate) trait JmapAccountResponse {
    fn account_id(&self) -> &str;
}

macro_rules! impl_jmap_account_response {
    ($($response:ty),+ $(,)?) => {
        $(
            impl JmapAccountResponse for $response {
                fn account_id(&self) -> &str {
                    self.account_id.as_str()
                }
            }
        )+
    };
}

impl_jmap_account_response!(
    JmapMailboxGetResponse,
    JmapEmailQueryResponse,
    JmapEmailGetResponse,
    JmapSearchSnippetGetResponse,
    JmapThreadGetResponse,
    JmapIdentityGetResponse,
);

impl<T> JmapAccountResponse for JmapSetResponse<T> {
    fn account_id(&self) -> &str {
        self.account_id.as_str()
    }
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapEmailCreateResult {
    pub(crate) id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapEmailSubmissionCreateResult {
    #[serde(default, rename = "threadId")]
    pub(crate) thread_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapUploadResponse {
    #[serde(rename = "accountId")]
    pub(crate) account_id: String,
    #[serde(rename = "blobId")]
    pub(crate) blob_id: String,
    #[serde(rename = "type")]
    pub(crate) content_type: String,
    pub(crate) size: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapSetError {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) description: String,
}

#[derive(Clone, Default)]
pub(crate) struct MailDetailCache {
    pub(crate) order: VecDeque<String>,
    pub(crate) entries: HashMap<String, MailMessage>,
}

pub struct MailboxMessagesLoad {
    pub mailbox_id: String,
    pub position: usize,
    pub query_state: String,
    pub messages: Vec<MailMessage>,
    pub next_position: Option<usize>,
}

pub struct MailWorkspaceSummaryLoad {
    pub workspace: crate::model::MailWorkspace,
    pub mailbox_state: String,
    pub query_state: String,
}
