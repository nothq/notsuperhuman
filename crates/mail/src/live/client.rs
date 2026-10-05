use std::sync::{Arc, Mutex};

use reqwest::{
    blocking::Client,
    header::{ACCEPT, CONTENT_TYPE},
    Url,
};
use serde_json::{json, Value};

use crate::live::{
    helpers::decode_account_jmap_response,
    types::{
        JmapCoreCapabilities, JmapCoreLimits, JmapMailboxGetResponse, JmapResponse,
        MailDetailCache, MailSessionResponse,
    },
};
use crate::model::MailAccountInfo;

mod contact_history;
mod context_accounts;
mod discovery;
pub(crate) mod download;
mod draft_ops;
mod search;
mod thread_ops;
mod workspace;

pub(crate) const CORE_CAPABILITY: &str = "urn:ietf:params:jmap:core";
pub(crate) const MAIL_CAPABILITY: &str = "urn:ietf:params:jmap:mail";
pub(crate) const SUBMISSION_CAPABILITY: &str = "urn:ietf:params:jmap:submission";
const MAIL_PAGE_SIZE: usize = 24;
const MAIL_BODY_MAX_BYTES: usize = 2 * 1024 * 1024;
const MAIL_DETAIL_CACHE_CAPACITY: usize = 64;

use download::MailDownloadUrlTemplate;

pub(crate) fn stalwart_bearer_auth(
    request: reqwest::blocking::RequestBuilder,
    session_token: &str,
) -> reqwest::blocking::RequestBuilder {
    request.bearer_auth(session_token)
}

/// A JMAP session URL and the API token that opens it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JmapCredentials {
    pub session_url: Url,
    pub token: String,
}

#[derive(Clone)]
pub struct MailLiveClient {
    client: Client,
    download_client: Client,
    credentials: JmapCredentials,
    context: Arc<MailLiveContext>,
    detail_cache: Arc<Mutex<MailDetailCache>>,
}

#[derive(Clone)]
pub(crate) struct MailLiveContext {
    pub(crate) api_url: Url,
    pub(crate) upload_url: Url,
    pub(crate) download_url: MailDownloadUrlTemplate,
    pub(crate) account: MailAccountInfo,
    pub(crate) owner_username: String,
    pub(crate) limits: JmapCoreLimits,
}

#[derive(Clone)]
pub(crate) struct MailLiveCatalog {
    client: Client,
    download_client: Client,
    credentials: JmapCredentials,
    api_url: Url,
    upload_url: Url,
    download_url: MailDownloadUrlTemplate,
    accounts: Arc<[MailAccountInfo]>,
    owner_username: String,
    limits: JmapCoreLimits,
}

pub(crate) struct MailboxMessagesRequest<'a> {
    pub(crate) api_url: &'a Url,
    pub(crate) account_id: &'a str,
    pub(crate) mailbox_id: &'a str,
    pub(crate) position: usize,
    pub(crate) cached_query_state: Option<&'a str>,
    pub(crate) cached_messages: Option<&'a [crate::model::MailMessage]>,
}

impl MailLiveClient {
    pub(crate) fn load_remote_image(
        &self,
        url: &str,
    ) -> Result<Option<remote_image_model::RemoteImageData>, String> {
        remote_image::load_public_remote_image_data(url).map(Some)
    }

    pub(crate) fn context(&self) -> &MailLiveContext {
        self.context.as_ref()
    }

    pub(crate) fn jmap_request(
        &self,
        api_url: &Url,
        payload: Value,
    ) -> Result<JmapResponse, String> {
        jmap_request(&self.client, &self.credentials, api_url, payload)
    }

    pub(crate) fn load_mailboxes(
        &self,
        api_url: &Url,
        account_id: &str,
    ) -> Result<JmapMailboxGetResponse, String> {
        let response = self.jmap_request(
            api_url,
            json!({
                "using": mail_capabilities(),
                "methodCalls": [["Mailbox/get", { "accountId": account_id }, "mailboxes"]]
            }),
        )?;
        let mut mailboxes = decode_account_jmap_response::<JmapMailboxGetResponse>(
            &response,
            "mailboxes",
            "Mailbox/get",
            account_id,
        )?;
        mailboxes
            .list
            .retain(|mailbox| mailbox.my_rights.may_read_items);
        if mailboxes.list.is_empty() {
            return Err(format!(
                "mail account {account_id} has no readable mailboxes"
            ));
        }
        Ok(mailboxes)
    }

    pub(crate) fn mailbox_id_for_role(
        &self,
        context: &MailLiveContext,
        role: &str,
    ) -> Result<String, String> {
        self.load_mailboxes(&context.api_url, context.account.id.as_str())?
            .list
            .into_iter()
            .find(|mailbox| mailbox.role.as_deref() == Some(role))
            .map(|mailbox| mailbox.id)
            .ok_or_else(|| format!("missing JMAP mailbox with role {role}"))
    }
}

impl MailLiveCatalog {
    pub(crate) fn accounts(&self) -> &[MailAccountInfo] {
        &self.accounts
    }

    pub(crate) fn session_url(&self) -> &Url {
        &self.credentials.session_url
    }

    pub(crate) fn owner_username(&self) -> &str {
        self.owner_username.as_str()
    }

    pub(crate) fn initial_account_id(&self) -> &str {
        self.accounts
            .iter()
            .find(|account| account.is_primary)
            .or_else(|| self.accounts.iter().find(|account| account.is_personal))
            .or_else(|| self.accounts.first())
            .expect("mail catalog always contains an accessible account")
            .id
            .as_str()
    }

    pub(crate) fn bind(&self, account_id: &str) -> Result<MailLiveClient, String> {
        let account = self
            .accounts
            .iter()
            .find(|account| account.id == account_id)
            .cloned()
            .ok_or_else(|| {
                format!("mail account {account_id} is absent from the current JMAP session")
            })?;
        Ok(MailLiveClient {
            client: self.client.clone(),
            download_client: self.download_client.clone(),
            credentials: self.credentials.clone(),
            context: Arc::new(MailLiveContext {
                api_url: self.api_url.clone(),
                upload_url: self.upload_url.clone(),
                download_url: self.download_url.clone(),
                account,
                owner_username: self.owner_username.clone(),
                limits: self.limits,
            }),
            detail_cache: Arc::new(Mutex::new(MailDetailCache::default())),
        })
    }
}

impl MailLiveClient {
    pub(crate) fn cached_message_detail(
        &self,
        message_id: &str,
    ) -> Result<Option<crate::model::MailMessage>, String> {
        let cache = self
            .detail_cache
            .lock()
            .map_err(|error| format!("mail detail cache lock poisoned: {error}"))?;
        Ok(cache.entries.get(message_id).cloned())
    }

    pub(crate) fn store_message_detail(
        &self,
        message: crate::model::MailMessage,
    ) -> Result<(), String> {
        let mut cache = self
            .detail_cache
            .lock()
            .map_err(|error| format!("mail detail cache lock poisoned: {error}"))?;
        let message_id = message.id.clone();
        cache.entries.insert(message_id.clone(), message);
        cache.order.retain(|cached_id| cached_id != &message_id);
        cache.order.push_back(message_id.clone());
        while cache.order.len() > MAIL_DETAIL_CACHE_CAPACITY {
            if let Some(evicted) = cache.order.pop_front() {
                cache.entries.remove(evicted.as_str());
            }
        }
        Ok(())
    }

    pub(crate) fn remove_message_details(&self, message_ids: &[String]) -> Result<(), String> {
        let mut cache = self
            .detail_cache
            .lock()
            .map_err(|error| format!("mail detail cache lock poisoned: {error}"))?;
        for message_id in message_ids {
            cache.entries.remove(message_id.as_str());
        }
        cache
            .order
            .retain(|cached_id| !message_ids.iter().any(|message_id| message_id == cached_id));
        Ok(())
    }
}

fn load_session(
    client: &Client,
    api_session: &JmapCredentials,
    discovery_url: &str,
) -> Result<MailSessionResponse, String> {
    stalwart_bearer_auth(
        client.get(discovery_url).header(ACCEPT, "application/json"),
        api_session.token.as_str(),
    )
    .send()
    .map_err(|error| format!("failed to probe mail discovery endpoint: {error}"))?
    .error_for_status()
    .map_err(|error| format!("mail discovery probe failed: {error}"))?
    .json::<MailSessionResponse>()
    .map_err(|error| format!("invalid JMAP session payload: {error}"))
}

fn require_jmap_core_limits(session: &MailSessionResponse) -> Result<JmapCoreLimits, String> {
    let raw = session.capabilities.get(CORE_CAPABILITY).ok_or_else(|| {
        format!("Stalwart JMAP session did not expose required {CORE_CAPABILITY}")
    })?;
    let capabilities = serde_json::from_value::<JmapCoreCapabilities>(raw.clone())
        .map_err(|error| format!("invalid Stalwart JMAP core capabilities: {error}"))?;
    capabilities.try_into()
}

fn jmap_request(
    client: &Client,
    api_session: &JmapCredentials,
    api_url: &Url,
    payload: Value,
) -> Result<JmapResponse, String> {
    let response = stalwart_bearer_auth(
        client
            .post(api_url.clone())
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .json(&payload),
        api_session.token.as_str(),
    )
    .send()
    .map_err(|error| format!("failed to call JMAP endpoint {api_url}: {error}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("JMAP request failed: {status} {body}"));
    }
    response
        .json::<JmapResponse>()
        .map_err(|error| format!("invalid JMAP response payload: {error}"))
}

pub(crate) fn mail_capabilities() -> [&'static str; 2] {
    [CORE_CAPABILITY, MAIL_CAPABILITY]
}

pub(crate) fn mail_submission_capabilities() -> [&'static str; 3] {
    [CORE_CAPABILITY, MAIL_CAPABILITY, SUBMISSION_CAPABILITY]
}

pub(crate) fn mail_snooze_capabilities() -> [&'static str; 3] {
    [
        "urn:ietf:params:jmap:core",
        "urn:ietf:params:jmap:mail",
        "urn:ietf:params:jmap:mail:snooze",
    ]
}

fn rewrite_jmap_session_url(session_url: &Url, raw_url: &str) -> Result<Url, String> {
    let parsed = Url::parse(raw_url)
        .or_else(|_| session_url.join(raw_url))
        .map_err(|error| format!("invalid JMAP session url {raw_url}: {error}"))?;
    let mut rewritten = session_url.clone();
    rewritten.set_path(parsed.path());
    rewritten.set_query(parsed.query());
    rewritten.set_fragment(parsed.fragment());
    Ok(rewritten)
}

#[cfg(test)]
mod tests {
    use reqwest::{header::AUTHORIZATION, Url};

    use super::{rewrite_jmap_session_url, stalwart_bearer_auth};

    #[test]
    fn stalwart_auth_uses_bearer_header() {
        let request = stalwart_bearer_auth(
            reqwest::blocking::Client::new().get("https://stalwart.test/jmap/session"),
            "session-token",
        )
        .build()
        .expect("request should build");

        assert_eq!(
            request
                .headers()
                .get(AUTHORIZATION)
                .and_then(|value| value.to_str().ok()),
            Some("Bearer session-token"),
        );
    }

    #[test]
    fn rewrite_jmap_session_url_reuses_public_discovery_origin() {
        let session_url = Url::parse("https://mail.example.com/jmap/session").expect("session url");
        let rewritten = rewrite_jmap_session_url(
            &session_url,
            "http://mail.example.com:8085/jmap/?u=1#ignored",
        )
        .expect("rewrite jmap url");
        assert_eq!(
            rewritten.as_str(),
            "https://mail.example.com/jmap/?u=1#ignored"
        );
    }
}
