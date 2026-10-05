use crate::model::{MailAddress, MailSendResult};
use serde_json::{json, Value};

use super::super::{
    mail_capabilities, mail_submission_capabilities, MailLiveClient, MailLiveContext,
};
use super::take_created_item;
use crate::live::{
    helpers::decode_account_jmap_response,
    types::{JmapEmail, JmapEmailGetResponse, JmapEmailSubmissionCreateResult, JmapSetResponse},
};

impl MailLiveClient {
    pub fn send_mail_draft(
        &self,
        draft_id: &str,
        identity_id: &str,
        to: Vec<MailAddress>,
        cc: Vec<MailAddress>,
    ) -> Result<MailSendResult, String> {
        let context = self.context();
        let mailbox_ids = SendMailboxIds {
            drafts: self.mailbox_id_for_role(context, "drafts")?,
            sent: self.mailbox_id_for_role(context, "sent")?,
        };
        let sendable_draft =
            self.load_sendable_mail_draft(context, draft_id, mailbox_ids.drafts.as_str())?;
        let created = self.create_mail_draft_submission(
            context,
            MailDraftSubmissionRequest {
                draft_id,
                identity_id,
                mail_from: sendable_draft.mail_from.as_str(),
                to,
                cc,
                mailbox_ids: &mailbox_ids,
            },
        )?;
        Ok(MailSendResult {
            draft_id: draft_id.to_string(),
            thread_id: submission_thread_id(created, sendable_draft.thread_id),
            sent_mailbox_id: mailbox_ids.sent,
        })
    }

    fn create_mail_draft_submission(
        &self,
        context: &MailLiveContext,
        request: MailDraftSubmissionRequest<'_>,
    ) -> Result<JmapEmailSubmissionCreateResult, String> {
        let response = self.jmap_request(
            &context.api_url,
            json!({
                "using": mail_submission_capabilities(),
                "methodCalls": [[
                    "EmailSubmission/set",
                    {
                        "accountId": context.account.id,
                        "create": {
                            "send": {
                                "identityId": request.identity_id,
                                "emailId": request.draft_id,
                                "envelope": submission_envelope(request.mail_from, request.to, request.cc)?
                            }
                        },
                        "onSuccessUpdateEmail": {
                            "#send": send_success_email_update(
                                request.mailbox_ids.drafts.as_str(),
                                request.mailbox_ids.sent.as_str()
                            )
                        }
                    },
                    "send"
                ]]
            }),
        )?;
        take_created_item(
            decode_account_jmap_response::<JmapSetResponse<JmapEmailSubmissionCreateResult>>(
                &response,
                "send",
                "EmailSubmission/set",
                context.account.id.as_str(),
            )?,
            "send",
            "draft send",
        )
    }

    fn load_sendable_mail_draft(
        &self,
        context: &MailLiveContext,
        draft_id: &str,
        drafts_mailbox_id: &str,
    ) -> Result<SendableMailDraft, String> {
        let draft = self.get_sendable_mail_draft(context, draft_id)?;
        validate_sendable_mail_draft(&draft, draft_id, drafts_mailbox_id)?;
        Ok(SendableMailDraft {
            thread_id: sendable_thread_id(draft.thread_id.as_str(), draft_id)?,
            mail_from: sendable_mail_from(&draft, draft_id)?,
        })
    }

    fn get_sendable_mail_draft(
        &self,
        context: &MailLiveContext,
        draft_id: &str,
    ) -> Result<JmapEmail, String> {
        let response = self.jmap_request(
            &context.api_url,
            json!({
                "using": mail_capabilities(),
                "methodCalls": [[
                    "Email/get",
                    {
                        "accountId": context.account.id,
                        "ids": [draft_id],
                        "properties": [
                            "id", "threadId", "from", "mailboxIds", "keywords"
                        ]
                    },
                    "draft"
                ]]
            }),
        )?;
        decode_account_jmap_response::<JmapEmailGetResponse>(
            &response,
            "draft",
            "Email/get",
            context.account.id.as_str(),
        )?
        .list
        .into_iter()
        .next()
        .ok_or_else(|| format!("missing JMAP draft {draft_id}"))
    }
}

struct SendMailboxIds {
    drafts: String,
    sent: String,
}

struct MailDraftSubmissionRequest<'a> {
    draft_id: &'a str,
    identity_id: &'a str,
    mail_from: &'a str,
    to: Vec<MailAddress>,
    cc: Vec<MailAddress>,
    mailbox_ids: &'a SendMailboxIds,
}

struct SendableMailDraft {
    thread_id: String,
    mail_from: String,
}

fn validate_sendable_mail_draft(
    draft: &JmapEmail,
    draft_id: &str,
    drafts_mailbox_id: &str,
) -> Result<(), String> {
    if !draft
        .mailbox_ids
        .get(drafts_mailbox_id)
        .copied()
        .unwrap_or(false)
    {
        return Err(format!("mail draft {draft_id} is not in Drafts"));
    }
    if !draft.keywords.get("$draft").copied().unwrap_or(false) {
        return Err(format!("mail draft {draft_id} is not marked as a draft"));
    }
    Ok(())
}

fn sendable_thread_id(raw_thread_id: &str, draft_id: &str) -> Result<String, String> {
    let thread_id = raw_thread_id.trim().to_string();
    if thread_id.is_empty() {
        return Err(format!("mail draft {draft_id} is missing a thread id"));
    }
    Ok(thread_id)
}

fn sendable_mail_from(draft: &JmapEmail, draft_id: &str) -> Result<String, String> {
    let from_emails = draft
        .from
        .iter()
        .map(|address| address.email.trim())
        .filter(|email| !email.is_empty())
        .map(|email| email.to_string())
        .collect::<Vec<_>>();
    match from_emails.as_slice() {
        [email] => Ok(email.clone()),
        _ => Err(format!(
            "mail draft {draft_id} must have exactly one From address"
        )),
    }
}

fn submission_thread_id(created: JmapEmailSubmissionCreateResult, fallback: String) -> String {
    created
        .thread_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
}

fn submission_envelope(
    mail_from: &str,
    to: Vec<MailAddress>,
    cc: Vec<MailAddress>,
) -> Result<Value, String> {
    let mail_from = mail_from.trim();
    if mail_from.is_empty() {
        return Err("mail send requires a From address".to_string());
    }
    let recipients = to
        .into_iter()
        .chain(cc)
        .filter_map(|address| {
            let email = address.email.trim().to_string();
            (!email.is_empty()).then_some(email)
        })
        .collect::<Vec<_>>();
    if recipients.is_empty() {
        return Err("mail send requires at least one recipient".to_string());
    }
    Ok(json!({
        "mailFrom": { "email": mail_from },
        "rcptTo": recipients.into_iter().map(|email| {
            json!({ "email": email })
        }).collect::<Vec<_>>()
    }))
}

fn send_success_email_update(drafts_mailbox_id: &str, sent_mailbox_id: &str) -> Value {
    let mut patch = serde_json::Map::new();
    patch.insert("keywords/$draft".to_string(), Value::Null);
    patch.insert(format!("mailboxIds/{drafts_mailbox_id}"), Value::Null);
    patch.insert(format!("mailboxIds/{sent_mailbox_id}"), Value::Bool(true));
    Value::Object(patch)
}
