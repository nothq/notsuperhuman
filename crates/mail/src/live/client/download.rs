use reqwest::{
    blocking::Response,
    header::{ACCEPT, ACCEPT_ENCODING},
};

use crate::model::{MailAttachmentDownloadRequest, MailAttachmentDownloadResult};

use super::{stalwart_bearer_auth, MailLiveClient};

mod transfer;
mod url_template;

pub(crate) use transfer::{receive_download, validate_download_destination};
pub(crate) use url_template::MailDownloadUrlTemplate;

pub(crate) const MAIL_DOWNLOAD_MAX_BYTES: u64 = 512 * 1024 * 1024;

impl MailLiveClient {
    pub(crate) fn download_mail_attachment(
        &self,
        request: MailAttachmentDownloadRequest,
    ) -> Result<MailAttachmentDownloadResult, String> {
        if request.declared_size > MAIL_DOWNLOAD_MAX_BYTES {
            return Err(format!(
                "mail attachment {:?} declares {} bytes; downloads are limited to {MAIL_DOWNLOAD_MAX_BYTES} bytes",
                request.file_name.as_str(),
                request.declared_size
            ));
        }
        validate_download_destination(&request.destination)?;
        let context = self.context();
        let url = context
            .download_url
            .expand(context.account.id.as_str(), &request)?;
        let response = stalwart_bearer_auth(
            self.download_client
                .get(url)
                .header(ACCEPT, "application/octet-stream")
                .header(ACCEPT_ENCODING, "identity"),
            self.credentials.token.as_str(),
        )
        .send()
        .map_err(|error| format!("failed to download mail attachment: {error}"))?;
        validate_download_response(&response, request.declared_size)?;
        receive_download(response, request)
    }
}

fn validate_download_response(response: &Response, declared_size: u64) -> Result<(), String> {
    if !response.status().is_success() {
        return Err(format!(
            "mail attachment download failed with HTTP {}",
            response.status()
        ));
    }
    if response
        .content_length()
        .is_some_and(|content_length| content_length != declared_size)
    {
        return Err(format!(
            "mail attachment Content-Length does not match its declared size of {declared_size} bytes"
        ));
    }
    Ok(())
}
