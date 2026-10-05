use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::ui::{
    MailIdentity, MailThread, MailUploadFile, MailWorkspace,
};

mod workspace_api;

#[derive(Clone)]
pub struct TestMailWorkspaceApi {
    pub workspaces_by_mailbox: HashMap<String, MailWorkspace>,
    pub cached_workspaces_by_mailbox: HashMap<String, MailWorkspace>,
    pub threads_by_id: HashMap<String, MailThread>,
    pub identity: MailIdentity,
}

#[derive(Clone, Default)]
pub struct TestMailLocalFileApi {
    pub files_by_path: HashMap<PathBuf, MailUploadFile>,
}

impl crate::model::MailLocalFileApi for TestMailLocalFileApi {
    fn load_upload_files(&self, paths: Vec<PathBuf>) -> Result<Vec<MailUploadFile>, String> {
        paths
            .into_iter()
            .map(|path| {
                self.files_by_path
                    .get(&path)
                    .cloned()
                    .ok_or_else(|| format!("missing test mail upload file {}", path.display()))
            })
            .collect()
    }
}

pub fn mail_test_local_file_api() -> Arc<dyn crate::model::MailLocalFileApi> {
    Arc::new(TestMailLocalFileApi::default())
}
