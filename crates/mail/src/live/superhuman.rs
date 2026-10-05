//! Superhuman's session API supplies short-lived Gmail access tokens. Only
//! this service sees the session cookie; Gmail sees only the access token.

#[cfg(target_os = "macos")]
mod desktop;
#[cfg(test)]
mod tests;

use std::{sync::Arc, time::Duration};

use reqwest::{
    blocking::Client,
    cookie::{CookieStore, Jar},
    StatusCode, Url,
};
use serde::{Deserialize, Serialize};

use super::accounts::{self, SavedAccount};

const ACCOUNTS_ORIGIN: &str = "https://accounts.superhuman.com";
const RECONNECT: &str = "Your Superhuman session has expired. Sign in to Superhuman Desktop, then choose Continue with Superhuman again.";

// Deliberately no Debug: these values are credentials, including on errors.
#[derive(Serialize, Deserialize)]
struct Session {
    google_id: String,
    cookie: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CsrfResponse {
    csrf_token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenResponse {
    auth_data: AuthData,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthData {
    access_token: String,
    expires_in: u64,
    email_address: String,
    google_id: String,
    scope: String,
}

struct SessionClient {
    client: Client,
    jar: Arc<Jar>,
    origin: Url,
}

impl SessionClient {
    fn new(session: &Session) -> Result<Self, String> {
        Self::at_origin(session, Url::parse(ACCOUNTS_ORIGIN).expect("constant URL"))
    }

    fn at_origin(session: &Session, origin: Url) -> Result<Self, String> {
        validate_session(session)?;
        let jar = Arc::new(Jar::default());
        jar.add_cookie_str(
            &format!("{}={}; Path=/", session.google_id, session.cookie),
            &origin,
        );
        let client = Client::builder()
            .cookie_provider(jar.clone())
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| "Could not create the Superhuman connection".to_string())?;
        Ok(Self {
            client,
            jar,
            origin,
        })
    }

    fn request(
        &self,
        method: reqwest::Method,
        endpoint: &str,
    ) -> reqwest::blocking::RequestBuilder {
        self.client
            .request(
                method,
                self.origin
                    .join(&format!("/~backend/v3/{endpoint}"))
                    .expect("constant endpoint"),
            )
            .header("Origin", "https://mail.superhuman.com")
            .header("Referer", "https://mail.superhuman.com/")
            .header("Cache-Control", "no-store")
    }

    fn exchange(&self, session: &mut Session, email: Option<&str>) -> Result<AuthData, String> {
        for attempt in 0..2 {
            let csrf = self
                .request(reqwest::Method::GET, "sessions.getCsrfToken")
                .header("x-superhuman-user-email", email.unwrap_or("unknown"))
                .send()
                .map_err(|_| {
                    "Could not reach Superhuman. Check your connection and try again.".to_string()
                })?;
            let csrf = checked_response(csrf)?
                .json::<CsrfResponse>()
                .map_err(|_| "Superhuman returned an invalid sign-in response".to_string())?;
            let mut body = serde_json::json!({"googleId": session.google_id});
            if let Some(email) = email {
                body["emailAddress"] = email.into();
            }
            let response = self
                .request(reqwest::Method::POST, "sessions.getTokens")
                .header("x-superhuman-user-email", email.unwrap_or("unknown"))
                .header("X-CSRF-Token", csrf.csrf_token)
                .json(&body)
                .send()
                .map_err(|_| {
                    "Could not reach Superhuman. Check your connection and try again.".to_string()
                })?;
            if response.status() == StatusCode::FORBIDDEN && attempt == 0 {
                continue;
            }
            let auth = checked_response(response)?
                .json::<TokenResponse>()
                .map_err(|_| "Superhuman returned an invalid account response".to_string())?
                .auth_data;
            validate_auth(&auth, session, email)?;
            // Keep a rotated session cookie for the next launch/refresh.
            if let Some(cookies) = self.jar.cookies(&self.origin) {
                if let Ok(cookies) = cookies.to_str() {
                    let prefix = format!("{}=", session.google_id);
                    if let Some(cookie) = cookies
                        .split("; ")
                        .find_map(|pair| pair.strip_prefix(&prefix))
                    {
                        session.cookie = cookie.to_string();
                    }
                }
            }
            return Ok(auth);
        }
        Err(RECONNECT.into())
    }
}

fn checked_response(
    response: reqwest::blocking::Response,
) -> Result<reqwest::blocking::Response, String> {
    match response.status() {
        status if status.is_success() => Ok(response),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(RECONNECT.into()),
        StatusCode::PAYMENT_REQUIRED => {
            Err("Superhuman requires an active account to connect.".into())
        }
        StatusCode::TOO_MANY_REQUESTS => {
            Err("Superhuman is busy. Try signing in again in a moment.".into())
        }
        status => Err(format!(
            "Superhuman sign-in failed (HTTP {}). Try again.",
            status.as_u16()
        )),
    }
}

fn validate_session(session: &Session) -> Result<(), String> {
    if !(6..=64).contains(&session.google_id.len())
        || !session.google_id.bytes().all(|b| b.is_ascii_digit())
        || session.cookie.is_empty()
        || !session
            .cookie
            .bytes()
            .all(|b| matches!(b, 0x21 | 0x23..=0x2b | 0x2d..=0x3a | 0x3c..=0x5b | 0x5d..=0x7e))
    {
        return Err("Invalid Superhuman session. Connect Superhuman Desktop again.".into());
    }
    Ok(())
}

fn validate_auth(auth: &AuthData, session: &Session, email: Option<&str>) -> Result<(), String> {
    if auth.google_id != session.google_id
        || email.is_some_and(|email| !email.eq_ignore_ascii_case(&auth.email_address))
    {
        return Err(
            "Superhuman returned a different account. Connect the intended account again.".into(),
        );
    }
    if auth.access_token.is_empty() || auth.expires_in <= 60 || !auth.email_address.contains('@') {
        return Err("Superhuman returned an incomplete Gmail session".into());
    }
    if !auth.scope.split_whitespace().any(|scope| {
        matches!(
            scope,
            "https://mail.google.com/" | "https://www.googleapis.com/auth/gmail.modify"
        )
    }) {
        return Err("This Superhuman account has no Gmail access. Connect a Gmail account in Superhuman Desktop first.".into());
    }
    Ok(())
}

pub(super) fn refresh(account: &SavedAccount) -> Result<(String, u64), String> {
    let SavedAccount::Superhuman { email, google_id } = account else {
        return Err("This is not a Superhuman account".into());
    };
    let mut session: Session = serde_json::from_str(&accounts::account_secret(account)?)
        .map_err(|_| "The saved Superhuman session is invalid. Connect again.".to_string())?;
    if &session.google_id != google_id {
        return Err(
            "The saved Superhuman account does not match its session. Connect again.".into(),
        );
    }
    let auth = SessionClient::new(&session)?.exchange(&mut session, Some(email))?;
    let secret = serde_json::to_string(&session)
        .map_err(|_| "Could not save the Superhuman session".to_string())?;
    accounts::update_account_secret(account, &secret)?;
    Ok((auth.access_token, auth.expires_in))
}

fn save_session(account: &SavedAccount, session: &Session) -> Result<(), String> {
    let secret = serde_json::to_string(session)
        .map_err(|_| "Could not save the Superhuman session".to_string())?;
    accounts::save_account(account, &secret)
}

/// Import only Superhuman's own Gmail sessions, with macOS Keychain approval.
pub(super) fn sign_in() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let sessions = desktop::sessions()?;
        let mut first_email = None;
        let mut first_error = None;
        for mut session in sessions {
            let result = connect_session(&mut session);
            match result {
                Ok(email) => {
                    first_email.get_or_insert(email);
                }
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }
        first_email.ok_or_else(|| {
            first_error.unwrap_or_else(|| {
                "Sign in to a Gmail account in Superhuman Desktop, then try again.".into()
            })
        })
    }
    #[cfg(not(target_os = "macos"))]
    Err("Superhuman Desktop sign-in is currently available on macOS. You can connect a JMAP account here.".into())
}

#[cfg(target_os = "macos")]
fn connect_session(session: &mut Session) -> Result<String, String> {
    let client = SessionClient::new(session)?;
    let auth = client.exchange(session, None)?;
    // Confirm the token can actually open this Gmail inbox before saving it.
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Profile {
        email_address: String,
    }
    let profile = client.client.get("https://gmail.googleapis.com/gmail/v1/users/me/profile")
        .bearer_auth(&auth.access_token).send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|_| "Gmail did not accept this Superhuman session. Reconnect Gmail in Superhuman Desktop and try again.".to_string())?
        .json::<Profile>().map_err(|_| "Gmail returned an invalid profile".to_string())?;
    if !profile
        .email_address
        .eq_ignore_ascii_case(&auth.email_address)
    {
        return Err("Gmail returned a different account from Superhuman".into());
    }
    let email = auth.email_address.to_ascii_lowercase();
    save_session(
        &SavedAccount::Superhuman {
            email: email.clone(),
            google_id: session.google_id.clone(),
        },
        session,
    )?;
    Ok(email)
}
