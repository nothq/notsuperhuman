use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use reqwest::Url;

use crate::model::MailAttachmentDownloadRequest;

const DOWNLOAD_PLACEHOLDERS: [(&str, &str); 4] = [
    ("{accountId}", "NOTSUPERHUMANMAILACCOUNTIDPLACEHOLDER"),
    ("{blobId}", "NOTSUPERHUMANMAILBLOBIDPLACEHOLDER"),
    ("{name}", "NOTSUPERHUMANMAILNAMEPLACEHOLDER"),
    ("{type}", "NOTSUPERHUMANMAILTYPEPLACEHOLDER"),
];

#[derive(Clone)]
pub(crate) struct MailDownloadUrlTemplate {
    template: String,
    public_origin: Url,
}

impl MailDownloadUrlTemplate {
    pub(in crate::live::client) fn resolve(
        discovery_url: &Url,
        template: &str,
    ) -> Result<Self, String> {
        require_public_https_url(discovery_url, "JMAP discovery URL")?;
        if template.is_empty() || template.trim() != template {
            return Err(
                "JMAP downloadUrl must be nonempty without surrounding whitespace".to_string(),
            );
        }
        let mut resolvable = template.to_string();
        for (placeholder, sentinel) in DOWNLOAD_PLACEHOLDERS {
            if resolvable.matches(placeholder).count() != 1 {
                return Err(format!(
                    "JMAP downloadUrl must contain exactly one {placeholder}"
                ));
            }
            if resolvable.contains(sentinel) {
                return Err("JMAP downloadUrl conflicts with a reserved placeholder".to_string());
            }
            resolvable = resolvable.replace(placeholder, sentinel);
        }
        if resolvable.contains('{') || resolvable.contains('}') {
            return Err("JMAP downloadUrl contains an unsupported placeholder".to_string());
        }
        let resolved = super::super::rewrite_jmap_session_url(discovery_url, resolvable.as_str())?;
        require_public_https_url(&resolved, "JMAP downloadUrl")?;
        if !same_origin(&resolved, discovery_url) {
            return Err("JMAP downloadUrl must resolve on the public discovery origin".to_string());
        }
        let mut resolved_template = resolved.to_string();
        for (placeholder, sentinel) in DOWNLOAD_PLACEHOLDERS {
            if resolved_template.matches(sentinel).count() != 1 {
                return Err(format!(
                    "JMAP downloadUrl must retain exactly one {placeholder} after public-origin resolution"
                ));
            }
            resolved_template = resolved_template.replace(sentinel, placeholder);
        }
        Ok(Self {
            template: resolved_template,
            public_origin: discovery_url.clone(),
        })
    }

    pub(super) fn expand(
        &self,
        account_id: &str,
        request: &MailAttachmentDownloadRequest,
    ) -> Result<Url, String> {
        let content_type = if request.content_type.trim().is_empty() {
            "application/octet-stream"
        } else {
            request.content_type.as_str()
        };
        let replacements = [
            ("{accountId}", account_id),
            ("{blobId}", request.blob_id.as_str()),
            ("{name}", request.file_name.as_str()),
            ("{type}", content_type),
        ];
        let mut expanded = self.template.clone();
        for (placeholder, value) in replacements {
            let encoded = utf8_percent_encode(value, NON_ALPHANUMERIC).to_string();
            expanded = expanded.replace(placeholder, encoded.as_str());
        }
        if expanded.contains('{') || expanded.contains('}') {
            return Err("expanded JMAP downloadUrl contains an unexpanded placeholder".to_string());
        }
        let url = Url::parse(expanded.as_str())
            .map_err(|error| format!("invalid expanded JMAP downloadUrl: {error}"))?;
        require_public_https_url(&url, "expanded JMAP downloadUrl")?;
        if !same_origin(&url, &self.public_origin) {
            return Err(
                "expanded JMAP downloadUrl must remain on the public discovery origin".to_string(),
            );
        }
        Ok(url)
    }
}

fn require_public_https_url(url: &Url, label: &str) -> Result<(), String> {
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(format!(
            "{label} must use an absolute HTTPS origin without userinfo or a fragment"
        ));
    }
    Ok(())
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
}
