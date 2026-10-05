use std::{path::PathBuf, sync::Arc};

use crate::model::{MailLocalFileApi, MailUploadFile};

struct ProductionMailLocalFileApi;

pub fn production_mail_local_file_api() -> Arc<dyn MailLocalFileApi> {
    Arc::new(ProductionMailLocalFileApi)
}

impl MailLocalFileApi for ProductionMailLocalFileApi {
    fn load_upload_files(&self, paths: Vec<PathBuf>) -> Result<Vec<MailUploadFile>, String> {
        paths
            .into_iter()
            .map(load_mail_upload_file_from_path)
            .collect()
    }

    fn default_download_directory(&self) -> Result<PathBuf, String> {
        dirs::download_dir().ok_or_else(|| {
            "The operating system did not provide a Downloads directory.".to_string()
        })
    }
}

fn load_mail_upload_file_from_path(path: PathBuf) -> Result<MailUploadFile, String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            format!(
                "failed to determine attachment filename for {}",
                path.display()
            )
        })?
        .to_string();
    let bytes = std::fs::read(&path)
        .map_err(|error| format!("failed to read attachment {}: {error}", path.display()))?;
    Ok(MailUploadFile {
        name,
        bytes,
        content_type: mail_upload_content_type(&path),
    })
}

fn mail_upload_content_type(path: &std::path::Path) -> String {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase());
    match extension.as_deref() {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("pdf") => "application/pdf",
        Some("txt") | Some("md") | Some("log") => "text/plain",
        Some("json") => "application/json",
        Some("csv") => "text/csv",
        Some("zip") => "application/zip",
        Some("mp4") => "video/mp4",
        Some("mov") => "video/quicktime",
        Some("m4a") => "audio/mp4",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("ogg") => "audio/ogg",
        _ => "application/octet-stream",
    }
    .to_string()
}
