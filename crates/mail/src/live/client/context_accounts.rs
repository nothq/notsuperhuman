use reqwest::{blocking::Client, Url};

use crate::live::types::{JmapCoreLimits, MailResolveResponse, MailSessionResponse};
use crate::model::MailAccountInfo;

use super::{JmapCredentials, MAIL_CAPABILITY, SUBMISSION_CAPABILITY};

mod probe;

const MAX_MAIL_ACCOUNTS: usize = 256;

/// The authenticated JMAP API that account discovery probes.
pub(super) struct JmapEndpoint<'a> {
    pub(super) client: &'a Client,
    pub(super) api_session: &'a JmapCredentials,
    pub(super) api_url: &'a Url,
}

pub(super) fn discover(
    endpoint: JmapEndpoint<'_>,
    session: &MailSessionResponse,
    primary_identity: &MailResolveResponse,
    limits: JmapCoreLimits,
) -> Result<Vec<MailAccountInfo>, String> {
    let JmapEndpoint {
        client,
        api_session,
        api_url,
    } = endpoint;
    let candidates = select_mail_accounts(session, primary_identity)?;
    let accounts = probe::accessible_accounts(client, api_session, api_url, candidates, limits)?;
    finalize_mail_accounts(accounts, primary_identity)
}

fn select_mail_accounts(
    session: &MailSessionResponse,
    primary_identity: &MailResolveResponse,
) -> Result<Vec<MailAccountInfo>, String> {
    validate_primary_mail_account(session)?;
    let mut account_ids = mail_account_ids(session)?;
    validate_mail_account_count(account_ids.as_slice())?;
    let primary_account_id = session.primary_accounts.get(MAIL_CAPABILITY);
    account_ids.sort_by(|left, right| {
        (primary_account_id != Some(left))
            .cmp(&(primary_account_id != Some(right)))
            .then_with(|| left.cmp(right))
    });
    account_ids
        .into_iter()
        .map(|id| {
            mail_account_info(
                session,
                primary_identity,
                id,
                primary_account_id.map(String::as_str),
            )
        })
        .collect()
}

fn finalize_mail_accounts(
    mut accounts: Vec<MailAccountInfo>,
    primary_identity: &MailResolveResponse,
) -> Result<Vec<MailAccountInfo>, String> {
    if accounts.is_empty() {
        return Err("Stalwart JMAP session did not expose an accessible mail account".to_string());
    }
    if accounts.iter().any(|account| account.is_primary) {
        return Ok(accounts);
    }
    let personal_indices = accounts
        .iter()
        .enumerate()
        .filter_map(|(index, account)| account.is_personal.then_some(index))
        .collect::<Vec<_>>();
    if personal_indices.is_empty() {
        return Ok(accounts);
    }
    if personal_indices.len() != 1 {
        return Err(
            "Stalwart JMAP session omitted a primary among multiple personal mail accounts"
                .to_string(),
        );
    }
    let email = primary_identity.email.trim();
    if email.is_empty() {
        return Err("mail resolve returned an empty personal mailbox address".to_string());
    }
    let account = &mut accounts[personal_indices[0]];
    account.address = Some(email.to_string());
    let primary_name = primary_identity.name.trim();
    if !primary_name.is_empty() {
        account.name = primary_name.to_string();
    }
    Ok(accounts)
}

fn validate_primary_mail_account(session: &MailSessionResponse) -> Result<(), String> {
    let Some(account_id) = session.primary_accounts.get(MAIL_CAPABILITY) else {
        return Ok(());
    };
    let account = session.accounts.get(account_id).ok_or_else(|| {
        format!("Stalwart JMAP primary mail account {account_id} is absent from accounts")
    })?;
    if !account.account_capabilities.contains_key(MAIL_CAPABILITY) {
        return Err(format!(
            "Stalwart JMAP primary mail account {account_id} does not advertise {MAIL_CAPABILITY}"
        ));
    }
    Ok(())
}

fn mail_account_ids(session: &MailSessionResponse) -> Result<Vec<String>, String> {
    let ids = session
        .accounts
        .iter()
        .filter_map(|(id, account)| {
            account
                .account_capabilities
                .contains_key(MAIL_CAPABILITY)
                .then_some(id.clone())
        })
        .collect::<Vec<_>>();
    if let Some(id) = ids
        .iter()
        .find(|id| id.is_empty() || id.trim() != id.as_str())
    {
        return Err(format!(
            "Stalwart JMAP mail account ID {id:?} must be nonempty without surrounding whitespace"
        ));
    }
    Ok(ids)
}

fn validate_mail_account_count(account_ids: &[String]) -> Result<(), String> {
    if account_ids.is_empty() {
        return Err("Stalwart JMAP session did not expose a mail-capable account".to_string());
    }
    if account_ids.len() > MAX_MAIL_ACCOUNTS {
        return Err(format!(
            "Stalwart JMAP session exposed {} mail-capable accounts; notsuperhuman supports at most {MAX_MAIL_ACCOUNTS}",
            account_ids.len()
        ));
    }
    Ok(())
}

fn mail_account_info(
    session: &MailSessionResponse,
    primary_identity: &MailResolveResponse,
    id: String,
    primary_account_id: Option<&str>,
) -> Result<MailAccountInfo, String> {
    let account = session
        .accounts
        .get(id.as_str())
        .expect("mail-capable account came from session accounts");
    let is_primary = primary_account_id == Some(id.as_str());
    let session_name = account.name.trim();
    if session_name.is_empty() {
        return Err(format!("Stalwart JMAP mail account {id} has an empty name"));
    }
    let primary_name = primary_identity.name.trim();
    let name = if is_primary && !primary_name.is_empty() {
        primary_name
    } else {
        session_name
    }
    .to_string();
    let address = if is_primary {
        let email = primary_identity.email.trim();
        if email.is_empty() {
            return Err("mail resolve returned an empty primary mailbox address".to_string());
        }
        Some(email.to_string())
    } else {
        None
    };
    Ok(MailAccountInfo {
        id,
        name,
        address,
        is_primary,
        is_personal: is_primary || account.is_personal,
        is_read_only: account.is_read_only,
        can_submit: !account.is_read_only
            && account
                .account_capabilities
                .contains_key(SUBMISSION_CAPABILITY),
    })
}
