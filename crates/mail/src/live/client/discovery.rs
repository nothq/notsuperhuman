use std::time::Duration;

use reqwest::{blocking::Client, redirect::Policy};

use crate::live::types::MailResolveResponse;

use super::{
    context_accounts, load_session, require_jmap_core_limits, rewrite_jmap_session_url,
    JmapCredentials, MailDownloadUrlTemplate, MailLiveCatalog,
};

const MAIL_DOWNLOAD_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const MAIL_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(10 * 60);

impl MailLiveCatalog {
    pub(crate) fn discover(credentials: JmapCredentials) -> Result<Self, String> {
        let (client, download_client) = build_http_clients()?;
        let session_url = credentials.session_url.clone();
        let session = load_session(&client, &credentials, session_url.as_str())?;
        let username = session.username.trim().to_string();
        if username.is_empty() {
            return Err("the JMAP session did not name the signed-in user".to_string());
        }
        let person = MailResolveResponse {
            email: username.clone(),
            name: String::new(),
        };
        let limits = require_jmap_core_limits(&session)?;
        let api_url = rewrite_jmap_session_url(&session_url, session.api_url.as_str())?;
        let download_url =
            MailDownloadUrlTemplate::resolve(&session_url, session.download_url.as_str())?;
        let accounts = context_accounts::discover(
            context_accounts::JmapEndpoint {
                client: &client,
                api_session: &credentials,
                api_url: &api_url,
            },
            &session,
            &person,
            limits,
        )?;
        Ok(Self {
            client,
            download_client,
            credentials,
            api_url,
            upload_url: rewrite_jmap_session_url(&session_url, session.upload_url.as_str())?,
            download_url,
            accounts: accounts.into(),
            owner_username: username,
            limits,
        })
    }
}

fn build_http_clients() -> Result<(Client, Client), String> {
    let client = Client::builder()
        .build()
        .map_err(|error| format!("failed to build mail HTTP client: {error}"))?;
    let download_client = Client::builder()
        .connect_timeout(MAIL_DOWNLOAD_CONNECT_TIMEOUT)
        .timeout(MAIL_DOWNLOAD_TIMEOUT)
        .redirect(Policy::none())
        .build()
        .map_err(|error| format!("failed to build mail download HTTP client: {error}"))?;
    Ok((client, download_client))
}
