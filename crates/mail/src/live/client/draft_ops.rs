mod send;

use crate::model::{MailAttachment, MailDraftRequest, MailMessage, MailUploadFile};
use reqwest::{
    header::{ACCEPT, CONTENT_TYPE},
    Url,
};
use serde_json::{json, Value};

use super::{mail_capabilities, MailLiveClient, MailLiveContext};
use crate::live::{
    helpers::decode_account_jmap_response,
    types::{JmapEmailCreateResult, JmapSetError, JmapSetResponse, JmapUploadResponse},
};

impl MailLiveClient {
    pub fn create_mail_draft(&self, request: MailDraftRequest) -> Result<MailMessage, String> {
        let context = self.context();
        let drafts_mailbox_id = self.mailbox_id_for_role(context, "drafts")?;
        let draft = draft_email_object(&request, drafts_mailbox_id.as_str())?;
        let response = self.jmap_request(
            &context.api_url,
            json!({
                "using": mail_capabilities(),
                "methodCalls": [[
                    "Email/set",
                    {
                        "accountId": context.account.id,
                        "create": { "draft": draft }
                    },
                    "draft"
                ]]
            }),
        )?;
        let created = take_created_item(
            decode_account_jmap_response::<JmapSetResponse<JmapEmailCreateResult>>(
                &response,
                "draft",
                "Email/set",
                context.account.id.as_str(),
            )?,
            "draft",
            "draft creation",
        )?;
        self.load_mail_message_detail_with_context(context, created.id.as_str())
    }

    pub fn update_mail_draft(
        &self,
        draft_id: &str,
        request: MailDraftRequest,
    ) -> Result<MailMessage, String> {
        let context = self.context();
        let drafts_mailbox_id = self.mailbox_id_for_role(context, "drafts")?;
        let draft = draft_email_object(&request, drafts_mailbox_id.as_str())?;
        let response = self.jmap_request(
            &context.api_url,
            json!({
                "using": mail_capabilities(),
                "methodCalls": [[
                    "Email/set",
                    {
                        "accountId": context.account.id,
                        "update": { draft_id: draft }
                    },
                    "draft"
                ]]
            }),
        )?;
        take_updated_item(
            decode_account_jmap_response::<JmapSetResponse<JmapEmailCreateResult>>(
                &response,
                "draft",
                "Email/set",
                context.account.id.as_str(),
            )?,
            draft_id,
            "draft update",
        )?;
        self.load_mail_message_detail_with_context(context, draft_id)
    }

    pub fn delete_mail_draft(&self, draft_id: &str) -> Result<(), String> {
        let context = self.context();
        let response = self.jmap_request(
            &context.api_url,
            json!({
                "using": mail_capabilities(),
                "methodCalls": [[
                    "Email/set",
                    { "accountId": context.account.id, "destroy": [draft_id] },
                    "draft"
                ]]
            }),
        )?;
        take_destroyed_item(
            decode_account_jmap_response::<JmapSetResponse<JmapEmailCreateResult>>(
                &response,
                "draft",
                "Email/set",
                context.account.id.as_str(),
            )?,
            draft_id,
            "draft deletion",
        )?;
        self.remove_message_details(&[draft_id.to_string()])
    }

    pub fn upload_mail_files(
        &self,
        files: Vec<MailUploadFile>,
    ) -> Result<Vec<MailAttachment>, String> {
        let context = self.context();
        let upload_url = resolved_mail_upload_url(context)?;
        files
            .into_iter()
            .map(|file| self.upload_mail_file(context, &upload_url, file))
            .collect()
    }

    fn upload_mail_file(
        &self,
        context: &MailLiveContext,
        upload_url: &Url,
        file: MailUploadFile,
    ) -> Result<MailAttachment, String> {
        let api_session = &self.credentials;
        let response = super::stalwart_bearer_auth(
            self.client
                .post(upload_url.clone())
                .header(ACCEPT, "application/json")
                .header(CONTENT_TYPE, file.content_type.clone())
                .body(file.bytes),
            api_session.token.as_str(),
        )
        .send()
        .map_err(|error| format!("failed to upload mail attachment {}: {error}", file.name))?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(format!(
                "mail attachment upload failed for {}: {status} {body}",
                file.name
            ));
        }
        let uploaded = response
            .json::<JmapUploadResponse>()
            .map_err(|error| format!("invalid mail attachment upload payload: {error}"))?;
        if uploaded.account_id != context.account.id {
            return Err(format!(
                "mail attachment upload returned unexpected account {}",
                uploaded.account_id
            ));
        }
        let blob_id = crate::model::MailBlobId::parse(uploaded.blob_id).map_err(|error| {
            format!("mail attachment upload returned an invalid blob ID: {error}")
        })?;
        Ok(MailAttachment {
            blob_id: Some(blob_id),
            name: file.name,
            content_type: uploaded.content_type,
            size: uploaded.size,
            disposition: Some("attachment".to_string()),
        })
    }
}

fn draft_email_object(
    request: &MailDraftRequest,
    drafts_mailbox_id: &str,
) -> Result<Value, String> {
    Ok(json!({
        "mailboxIds": { drafts_mailbox_id: true },
        "keywords": { "$draft": true, "$seen": true },
        "from": [request.from.clone()],
        "replyTo": request.reply_to.clone(),
        "to": request.to.clone(),
        "cc": request.cc.clone(),
        "subject": request.subject,
        "inReplyTo": request.in_reply_to,
        "references": request.references,
        "textBody": [{ "partId": "text-1", "type": "text/plain" }],
        "bodyValues": { "text-1": { "value": request.body_text } },
        "attachments": attachment_parts(&request.attachments)?
    }))
}

fn attachment_parts(attachments: &[MailAttachment]) -> Result<Vec<Value>, String> {
    attachments
        .iter()
        .map(|attachment| {
            let blob_id = attachment.blob_id.as_ref().ok_or_else(|| {
                format!(
                    "mail draft attachment {:?} does not have a downloadable blob ID",
                    attachment.name
                )
            })?;
            Ok(json!({
                "blobId": blob_id.as_str(),
                "name": attachment.name,
                "type": attachment.content_type,
                "disposition": attachment.disposition.clone().unwrap_or_else(|| "attachment".to_string())
            }))
        })
        .collect()
}

fn resolved_mail_upload_url(context: &MailLiveContext) -> Result<Url, String> {
    Url::parse(
        context
            .upload_url
            .as_str()
            .replace("{accountId}", context.account.id.as_str())
            .as_str(),
    )
    .map_err(|error| format!("invalid JMAP upload url: {error}"))
}

fn take_created_item<T>(
    mut response: JmapSetResponse<T>,
    key: &str,
    operation: &str,
) -> Result<T, String> {
    if let Some(created) = response.created.remove(key) {
        return Ok(created);
    }
    if let Some(error) = response.not_created.remove(key) {
        return Err(format_set_error(operation, error));
    }
    Err(format!(
        "{operation} did not return a created item for {key}"
    ))
}

fn take_updated_item<T>(
    mut response: JmapSetResponse<T>,
    id: &str,
    operation: &str,
) -> Result<(), String> {
    let error = response.not_updated.remove(id);
    if error.is_none() {
        return Ok(());
    }
    Err(format_set_error(
        operation,
        error.unwrap_or(JmapSetError {
            kind: "unknown".to_string(),
            description: String::new(),
        }),
    ))
}

fn take_destroyed_item<T>(
    mut response: JmapSetResponse<T>,
    id: &str,
    operation: &str,
) -> Result<(), String> {
    let error = response.not_destroyed.remove(id);
    if error.is_none() {
        return Ok(());
    }
    Err(format_set_error(
        operation,
        error.unwrap_or(JmapSetError {
            kind: "unknown".to_string(),
            description: String::new(),
        }),
    ))
}

fn format_set_error(operation: &str, error: JmapSetError) -> String {
    if error.description.is_empty() {
        format!("{operation} failed: {}", error.kind)
    } else {
        format!("{operation} failed: {}: {}", error.kind, error.description)
    }
}
