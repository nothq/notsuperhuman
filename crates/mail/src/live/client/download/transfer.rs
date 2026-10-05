use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

use crate::model::{
    MailAttachmentDownloadDestination, MailAttachmentDownloadRequest, MailAttachmentDownloadResult,
    MailDownloadFileName,
};

use super::MAIL_DOWNLOAD_MAX_BYTES;

const MAIL_DOWNLOAD_BUFFER_BYTES: usize = 64 * 1024;
const MAIL_DOWNLOAD_TEMP_ATTEMPTS: u16 = 1_000;
const MAIL_DOWNLOAD_COLLISION_ATTEMPTS: usize = 10_000;

static MAIL_DOWNLOAD_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) fn receive_download(
    mut response: impl Read,
    request: MailAttachmentDownloadRequest,
) -> Result<MailAttachmentDownloadResult, String> {
    let directory = download_directory(&request.destination)?;
    let mut temporary = TemporaryDownload::create(directory)?;
    let transfer = stream_response(&mut response, temporary.file_mut(), request.declared_size);
    let bytes_written = match transfer {
        Ok(bytes_written) => bytes_written,
        Err(error) => return temporary.fail(error),
    };
    if let Err(error) = temporary.sync() {
        return temporary.fail(error);
    }
    let destination = match &request.destination {
        MailAttachmentDownloadDestination::ExplicitlySelected(destination) => {
            temporary.publish_selected(destination.as_path())?
        }
        MailAttachmentDownloadDestination::NewFileInDirectory(directory) => {
            temporary.publish_new(directory.as_path(), &request.file_name)?
        }
    };
    Ok(MailAttachmentDownloadResult {
        destination,
        bytes_written,
    })
}

fn download_directory(destination: &MailAttachmentDownloadDestination) -> Result<&Path, String> {
    let directory = match destination {
        MailAttachmentDownloadDestination::ExplicitlySelected(destination) => destination
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new(".")),
        MailAttachmentDownloadDestination::NewFileInDirectory(directory) => directory.as_path(),
    };
    let metadata = fs::symlink_metadata(directory).map_err(|error| {
        format!(
            "failed to inspect mail attachment destination directory {}: {error}",
            directory.display()
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(format!(
            "mail attachment destination must be a physical directory: {}",
            directory.display()
        ));
    }
    Ok(directory)
}

pub(crate) fn validate_download_destination(
    destination: &MailAttachmentDownloadDestination,
) -> Result<(), String> {
    download_directory(destination)?;
    if let MailAttachmentDownloadDestination::ExplicitlySelected(destination) = destination {
        if destination.file_name().is_none() {
            return Err(format!(
                "mail attachment destination must include a file name: {}",
                destination.display()
            ));
        }
    }
    Ok(())
}

fn stream_response(
    response: &mut impl Read,
    file: &mut File,
    declared_size: u64,
) -> Result<u64, String> {
    let mut buffer = [0_u8; MAIL_DOWNLOAD_BUFFER_BYTES];
    let mut received = 0_u64;
    loop {
        let count = response
            .read(&mut buffer)
            .map_err(|error| format!("failed to read mail attachment response: {error}"))?;
        if count == 0 {
            break;
        }
        let count_u64 = u64::try_from(count)
            .map_err(|error| format!("invalid mail attachment response chunk length: {error}"))?;
        received = received
            .checked_add(count_u64)
            .ok_or_else(|| "mail attachment response byte count overflowed".to_string())?;
        if received > declared_size || received > MAIL_DOWNLOAD_MAX_BYTES {
            return Err(format!(
                "mail attachment response exceeded its declared size of {declared_size} bytes"
            ));
        }
        file.write_all(&buffer[..count])
            .map_err(|error| format!("failed to write mail attachment temporary file: {error}"))?;
    }
    if received != declared_size {
        return Err(format!(
            "mail attachment response ended after {received} bytes; expected {declared_size} bytes"
        ));
    }
    Ok(received)
}

struct TemporaryDownload {
    path: Option<PathBuf>,
    file: Option<File>,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

impl TemporaryDownload {
    fn create(directory: &Path) -> Result<Self, String> {
        for _ in 0..MAIL_DOWNLOAD_TEMP_ATTEMPTS {
            let sequence = MAIL_DOWNLOAD_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = directory.join(format!(
                ".notsuperhuman-mail-download-{}-{sequence}.tmp",
                std::process::id()
            ));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            options.mode(0o600);
            match options.open(&path) {
                Ok(file) => {
                    let metadata = file.metadata().map_err(|error| {
                        format!(
                            "failed to inspect mail attachment temporary file {}: {error}",
                            path.display()
                        )
                    })?;
                    if !metadata.is_file() {
                        return Err(format!(
                            "mail attachment temporary path is not a regular file: {}",
                            path.display()
                        ));
                    }
                    return Ok(Self {
                        path: Some(path),
                        file: Some(file),
                        #[cfg(unix)]
                        device: metadata.dev(),
                        #[cfg(unix)]
                        inode: metadata.ino(),
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(format!(
                        "failed to create mail attachment temporary file in {}: {error}",
                        directory.display()
                    ));
                }
            }
        }
        Err(format!(
            "failed to reserve a unique mail attachment temporary file in {}",
            directory.display()
        ))
    }

    fn file_mut(&mut self) -> &mut File {
        self.file
            .as_mut()
            .expect("unpublished mail download retains its temporary file")
    }

    fn sync(&mut self) -> Result<(), String> {
        let path = self.path().to_path_buf();
        let file = self
            .file
            .as_mut()
            .expect("unpublished mail download retains its temporary file");
        file.flush().map_err(|error| {
            format!(
                "failed to flush mail attachment temporary file {}: {error}",
                path.display()
            )
        })?;
        file.sync_all().map_err(|error| {
            format!(
                "failed to synchronize mail attachment temporary file {}: {error}",
                path.display()
            )
        })
    }

    fn publish_selected(mut self, destination: &Path) -> Result<PathBuf, String> {
        self.file.take();
        let temporary = self.path().to_path_buf();
        if let Err(error) = fs::rename(&temporary, destination) {
            return self.fail(format!(
                "failed to atomically save mail attachment to {}: {error}",
                destination.display()
            ));
        }
        self.path = None;
        sync_directory(destination.parent().unwrap_or_else(|| Path::new(".")))?;
        Ok(destination.to_path_buf())
    }

    fn publish_new(
        mut self,
        directory: &Path,
        file_name: &MailDownloadFileName,
    ) -> Result<PathBuf, String> {
        self.file.take();
        let temporary = self.path().to_path_buf();
        for index in 1..=MAIL_DOWNLOAD_COLLISION_ATTEMPTS {
            let destination_name = file_name.with_collision_index(index);
            let destination = directory.join(destination_name.as_str());
            match publish_without_replacing(&temporary, &destination) {
                Ok(()) => {
                    self.path = None;
                    sync_directory(directory)?;
                    return Ok(destination);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return self.fail(format!(
                        "failed to atomically save mail attachment to {}: {error}",
                        destination.display()
                    ));
                }
            }
        }
        self.fail(format!(
            "failed to choose a collision-free mail attachment name in {}",
            directory.display()
        ))
    }

    fn path(&self) -> &Path {
        self.path
            .as_deref()
            .expect("unpublished mail download retains its temporary path")
    }

    fn fail<T>(mut self, error: String) -> Result<T, String> {
        match self.remove_owned_temporary() {
            Ok(()) => Err(error),
            Err(cleanup) => Err(format!("{error}; temporary file cleanup failed: {cleanup}")),
        }
    }

    fn remove_owned_temporary(&mut self) -> Result<(), String> {
        let Some(path) = self.path.as_ref() else {
            return Ok(());
        };
        #[cfg(unix)]
        {
            let metadata = fs::symlink_metadata(path).map_err(|error| {
                format!(
                    "failed to inspect mail attachment temporary file {}: {error}",
                    path.display()
                )
            })?;
            if !metadata.is_file() || metadata.dev() != self.device || metadata.ino() != self.inode
            {
                return Err(format!(
                    "mail attachment temporary file changed before cleanup: {}",
                    path.display()
                ));
            }
        }
        fs::remove_file(path).map_err(|error| {
            format!(
                "failed to remove mail attachment temporary file {}: {error}",
                path.display()
            )
        })?;
        self.path = None;
        self.file = None;
        Ok(())
    }
}

impl Drop for TemporaryDownload {
    fn drop(&mut self) {
        if self.path.is_some() {
            let _ = self.remove_owned_temporary();
        }
    }
}

#[cfg(unix)]
fn publish_without_replacing(temporary: &Path, destination: &Path) -> std::io::Result<()> {
    let directory_path = destination.parent().unwrap_or_else(|| Path::new("."));
    let directory = File::open(directory_path)?;
    let temporary_name = temporary
        .file_name()
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let destination_name = destination
        .file_name()
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    rustix::fs::renameat_with(
        &directory,
        temporary_name,
        &directory,
        destination_name,
        rustix::fs::RenameFlags::NOREPLACE,
    )
    .map_err(std::io::Error::from)
}

#[cfg(windows)]
fn publish_without_replacing(temporary: &Path, destination: &Path) -> std::io::Result<()> {
    let mut temporary_path = tempfile::TempPath::try_from_path(temporary.to_path_buf())?;
    temporary_path.disable_cleanup(true);
    temporary_path
        .persist_noclobber(destination)
        .map_err(|error| error.error)
}

fn sync_directory(directory: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        File::open(directory)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| {
                format!(
                    "failed to synchronize mail attachment destination directory {}: {error}",
                    directory.display()
                )
            })
    }
    #[cfg(windows)]
    {
        let _ = directory;
        Ok(())
    }
}
