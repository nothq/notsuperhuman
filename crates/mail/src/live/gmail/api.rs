use std::{
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use reqwest::{blocking::Client, Method, StatusCode};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;

use crate::live::accounts::SavedAccount;

const GMAIL_API: &str = "https://gmail.googleapis.com/gmail/v1/users/me";
const PARALLEL_REQUESTS: usize = 8;

/// A signed-in Gmail account with credentials in the private secret store
/// and a short-lived token refreshed through its selected sign-in provider.
#[derive(Clone)]
pub(crate) struct GmailApi {
    pub(crate) email: String,
    client: Client,
    account: SavedAccount,
    access_token: Arc<Mutex<Option<(String, u64)>>>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GmailLabel {
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(default, rename = "type")]
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) label_list_visibility: Option<String>,
    #[serde(default)]
    pub(crate) threads_total: u64,
    #[serde(default)]
    pub(crate) threads_unread: u64,
}

#[derive(Deserialize)]
pub(crate) struct GmailLabelList {
    #[serde(default)]
    pub(crate) labels: Vec<GmailLabel>,
}

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GmailMessage {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) thread_id: String,
    #[serde(default)]
    pub(crate) label_ids: Vec<String>,
    #[serde(default)]
    pub(crate) snippet: String,
    #[serde(default)]
    pub(crate) internal_date: String,
    #[serde(default)]
    pub(crate) payload: GmailPart,
}

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GmailPart {
    #[serde(default)]
    pub(crate) mime_type: String,
    #[serde(default)]
    pub(crate) filename: String,
    #[serde(default)]
    pub(crate) headers: Vec<GmailHeader>,
    #[serde(default)]
    pub(crate) body: GmailBody,
    #[serde(default)]
    pub(crate) parts: Vec<GmailPart>,
}

#[derive(Clone, Default, Deserialize)]
pub(crate) struct GmailHeader {
    pub(crate) name: String,
    pub(crate) value: String,
}

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GmailBody {
    #[serde(default)]
    pub(crate) attachment_id: Option<String>,
    #[serde(default)]
    pub(crate) size: u64,
    #[serde(default)]
    pub(crate) data: Option<String>,
}

#[derive(Clone, Deserialize)]
pub(crate) struct GmailThread {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) messages: Vec<GmailMessage>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GmailThreadPage {
    #[serde(default)]
    pub(crate) threads: Vec<GmailIds>,
    #[serde(default)]
    pub(crate) next_page_token: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GmailMessagePage {
    #[serde(default)]
    pub(crate) messages: Vec<GmailIds>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GmailIds {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) thread_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GmailDraft {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) message: GmailMessage,
}

#[derive(Deserialize)]
pub(crate) struct GmailDraftList {
    #[serde(default)]
    pub(crate) drafts: Vec<GmailDraft>,
}

#[derive(Deserialize)]
pub(crate) struct GmailAttachmentData {
    #[serde(default)]
    pub(crate) data: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GmailSendAs {
    pub(crate) send_as_email: String,
    #[serde(default)]
    pub(crate) display_name: String,
    #[serde(default)]
    pub(crate) reply_to_address: String,
    #[serde(default)]
    pub(crate) signature: String,
    #[serde(default)]
    pub(crate) is_primary: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GmailSendAsList {
    #[serde(default)]
    pub(crate) send_as: Vec<GmailSendAs>,
}

impl GmailApi {
    pub(crate) fn new(account: SavedAccount) -> Result<Self, String> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(60))
            // Inbox pages fan out briefly. Retain only a small idle pool
            // after each batch instead of keeping every TLS connection alive.
            .pool_max_idle_per_host(2)
            .pool_idle_timeout(Duration::from_secs(15))
            .build()
            .map_err(|error| format!("failed to build the Gmail HTTP client: {error}"))?;
        Ok(Self {
            email: account.label().to_string(),
            client,
            account,
            access_token: Arc::new(Mutex::new(None)),
        })
    }

    pub(crate) fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, String> {
        self.call(Method::GET, path, query, None)
    }

    pub(crate) fn post<T: DeserializeOwned>(&self, path: &str, body: Value) -> Result<T, String> {
        self.call(Method::POST, path, &[], Some(body))
    }

    pub(crate) fn put<T: DeserializeOwned>(&self, path: &str, body: Value) -> Result<T, String> {
        self.call(Method::PUT, path, &[], Some(body))
    }

    pub(crate) fn delete(&self, path: &str) -> Result<(), String> {
        self.send(Method::DELETE, path, &[], None).map(drop)
    }

    /// Runs one request per item, a few at a time, keeping the input order.
    pub(crate) fn parallel<T, R>(
        &self,
        items: &[T],
        request: impl Fn(&Self, &T) -> Result<R, String> + Sync,
    ) -> Result<Vec<R>, String>
    where
        T: Sync,
        R: Send,
    {
        let mut results = Vec::with_capacity(items.len());
        for chunk in items.chunks(PARALLEL_REQUESTS) {
            let chunk_results = std::thread::scope(|scope| {
                let handles = chunk
                    .iter()
                    .map(|item| scope.spawn(|| request(self, item)))
                    .collect::<Vec<_>>();
                handles
                    .into_iter()
                    .map(|handle| {
                        handle
                            .join()
                            .unwrap_or_else(|_| Err("a Gmail request panicked".to_string()))
                    })
                    .collect::<Vec<_>>()
            });
            for result in chunk_results {
                results.push(result?);
            }
        }
        Ok(results)
    }

    fn call<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, &str)],
        body: Option<Value>,
    ) -> Result<T, String> {
        self.send(method, path, query, body)?
            .json::<T>()
            .map_err(|error| format!("invalid Gmail response for {path}: {error}"))
    }

    fn send(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, &str)],
        body: Option<Value>,
    ) -> Result<reqwest::blocking::Response, String> {
        let mut attempt = 0;
        let mut force_refresh = false;
        loop {
            let mut request = self
                .client
                .request(method.clone(), format!("{GMAIL_API}{path}"))
                .bearer_auth(self.token(force_refresh)?)
                .query(query);
            if let Some(body) = &body {
                request = request.json(body);
            }
            let response = request
                .send()
                .map_err(|error| format!("Gmail request {path} failed: {error}"))?;
            let status = response.status();
            let retry = match status {
                StatusCode::UNAUTHORIZED => attempt == 0,
                StatusCode::TOO_MANY_REQUESTS => attempt < 3,
                StatusCode::SERVICE_UNAVAILABLE => method == Method::GET && attempt < 3,
                _ => false,
            };
            if retry {
                force_refresh = status == StatusCode::UNAUTHORIZED;
                if status != StatusCode::UNAUTHORIZED {
                    std::thread::sleep(Duration::from_millis(400 << attempt));
                }
                attempt += 1;
                continue;
            }
            if !status.is_success() {
                let text = response.text().unwrap_or_default();
                return Err(format!(
                    "Gmail request {path} failed: {status} {}",
                    gmail_error_message(&text)
                ));
            }
            return Ok(response);
        }
    }

    fn token(&self, force_refresh: bool) -> Result<String, String> {
        let mut cached = self
            .access_token
            .lock()
            .map_err(|error| format!("Gmail token lock poisoned: {error}"))?;
        let now = now_seconds();
        if let Some((token, expires_at)) = cached.as_ref() {
            if !force_refresh && *expires_at > now + 60 {
                return Ok(token.clone());
            }
        }
        let (token, expires_in) = match &self.account {
            SavedAccount::Superhuman { .. } => crate::live::superhuman::refresh(&self.account)?,
            SavedAccount::Jmap { .. } => return Err("A JMAP account cannot use Gmail".into()),
        };
        *cached = Some((token.clone(), now + expires_in));
        Ok(token)
    }
}

impl GmailPart {
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case(name))
            .map(|header| header.value.as_str())
    }
}

fn gmail_error_message(body: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| {
            value
                .pointer("/error/message")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| body.chars().take(200).collect())
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
