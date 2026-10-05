use std::path::PathBuf;

use serde::Serialize;

use super::MailAttachment;

const MAIL_DOWNLOAD_FILE_NAME_MAX_BYTES: usize = 240;
const MAIL_DOWNLOAD_EXTENSION_MAX_BYTES: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct MailBlobId(String);

impl MailBlobId {
    pub fn parse(value: String) -> Result<Self, String> {
        if value.is_empty() || value.trim() != value {
            return Err(
                "mail attachment blob ID must be nonempty without surrounding whitespace"
                    .to_string(),
            );
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for MailBlobId {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailDownloadFileName(String);

impl MailDownloadFileName {
    pub fn for_attachment(name: &str, content_type: &str) -> Self {
        let component = name
            .split(['/', '\\'])
            .rfind(|component| !matches!(*component, "" | "." | ".."))
            .unwrap_or_default();
        let hidden_component = component.starts_with('.');
        let mut cleaned = component
            .chars()
            .filter(|character| {
                !character.is_control()
                    && !matches!(
                        *character,
                        '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*'
                    )
            })
            .collect::<String>();
        cleaned = cleaned
            .trim()
            .trim_start_matches('.')
            .trim_end_matches([' ', '.'])
            .to_string();
        if cleaned.is_empty() || matches!(cleaned.as_str(), "." | "..") {
            cleaned = "attachment".to_string();
        }
        if hidden_component && file_extension(cleaned.as_str()).is_some() {
            cleaned = format!("attachment-{cleaned}");
        } else if hidden_component {
            cleaned = content_type_extension(content_type)
                .map(|extension| format!("attachment.{extension}"))
                .unwrap_or_else(|| "attachment".to_string());
        } else if file_extension(cleaned.as_str()).is_none() {
            if let Some(extension) = content_type_extension(content_type) {
                cleaned.push('.');
                cleaned.push_str(extension);
            }
        }
        Self(bound_file_name(cleaned.as_str(), None))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub(crate) fn with_collision_index(&self, index: usize) -> String {
        if index <= 1 {
            return self.0.clone();
        }
        bound_file_name(self.0.as_str(), Some(index))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MailAttachmentDownloadDestination {
    ExplicitlySelected(PathBuf),
    NewFileInDirectory(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailAttachmentDownloadRequest {
    pub(crate) blob_id: MailBlobId,
    pub(crate) file_name: MailDownloadFileName,
    pub(crate) content_type: String,
    pub(crate) declared_size: u64,
    pub(crate) destination: MailAttachmentDownloadDestination,
}

impl MailAttachmentDownloadRequest {
    pub fn for_selected_path(attachment: &MailAttachment, destination: PathBuf) -> Option<Self> {
        Self::new(
            attachment,
            MailAttachmentDownloadDestination::ExplicitlySelected(destination),
        )
    }

    pub fn for_directory(attachment: &MailAttachment, directory: PathBuf) -> Option<Self> {
        Self::new(
            attachment,
            MailAttachmentDownloadDestination::NewFileInDirectory(directory),
        )
    }

    fn new(
        attachment: &MailAttachment,
        destination: MailAttachmentDownloadDestination,
    ) -> Option<Self> {
        Some(Self {
            blob_id: attachment.blob_id.clone()?,
            file_name: MailDownloadFileName::for_attachment(
                attachment.name.as_str(),
                attachment.content_type.as_str(),
            ),
            content_type: attachment.content_type.clone(),
            declared_size: attachment.size,
            destination,
        })
    }

    pub fn file_name(&self) -> &MailDownloadFileName {
        &self.file_name
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailAttachmentDownloadResult {
    pub destination: PathBuf,
    pub bytes_written: u64,
}

fn bound_file_name(value: &str, collision_index: Option<usize>) -> String {
    let (stem, extension) = split_file_name(value);
    let suffix = collision_index
        .map(|index| format!(" ({index})"))
        .unwrap_or_default();
    let reserved = suffix.len() + extension.len();
    let maximum_stem_bytes = MAIL_DOWNLOAD_FILE_NAME_MAX_BYTES.saturating_sub(reserved);
    let mut bounded_stem = truncate_utf8(stem, maximum_stem_bytes)
        .trim_end()
        .to_string();
    if bounded_stem.is_empty() {
        bounded_stem = truncate_utf8("attachment", maximum_stem_bytes).to_string();
    }
    format!("{bounded_stem}{suffix}{extension}")
}

fn split_file_name(value: &str) -> (&str, &str) {
    let Some((stem, extension)) = value.rsplit_once('.') else {
        return (value, "");
    };
    if stem.is_empty()
        || extension.is_empty()
        || extension.len() > MAIL_DOWNLOAD_EXTENSION_MAX_BYTES
    {
        return (value, "");
    }
    let extension_start = value.len() - extension.len() - 1;
    (&value[..extension_start], &value[extension_start..])
}

fn file_extension(value: &str) -> Option<&str> {
    let (stem, extension) = split_file_name(value);
    (!extension.is_empty() && stem != value).then_some(extension)
}

fn truncate_utf8(value: &str, maximum_bytes: usize) -> &str {
    if value.len() <= maximum_bytes {
        return value;
    }
    let mut boundary = maximum_bytes;
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    &value[..boundary]
}

fn content_type_extension(content_type: &str) -> Option<&'static str> {
    match content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "application/pdf" => Some("pdf"),
        "application/json" => Some("json"),
        "application/zip" => Some("zip"),
        "application/octet-stream" => Some("bin"),
        "message/rfc822" => Some("eml"),
        "text/plain" => Some("txt"),
        "text/csv" => Some("csv"),
        "text/html" => Some("html"),
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "image/svg+xml" => Some("svg"),
        "audio/mpeg" => Some("mp3"),
        "audio/mp4" => Some("m4a"),
        "audio/wav" => Some("wav"),
        "audio/ogg" => Some("ogg"),
        "video/mp4" => Some("mp4"),
        "video/quicktime" => Some("mov"),
        _ => None,
    }
}
