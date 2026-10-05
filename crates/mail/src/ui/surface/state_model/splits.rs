use super::super::{MailListSource, MailSplitId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum MailSplitInputField {
    Name,
    Query,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MailSplitEditorTarget {
    New,
    Existing(MailSplitId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MailSplitEditorState {
    pub(crate) target: MailSplitEditorTarget,
    pub(crate) name: String,
    pub(crate) query: String,
    pub(crate) focused_field: MailSplitInputField,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MailSplitMutationRequest {
    pub(crate) account_generation: u64,
    pub(crate) account_id: String,
    pub(crate) selected_source: MailListSource,
    pub(crate) settings_generation: u64,
    pub(crate) mutation_generation: u64,
    pub(crate) close_editor_on_success: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MailSplitSettingsState {
    pub(crate) generation: u64,
    pub(crate) editor: Option<MailSplitEditorState>,
    pub(crate) error: Option<String>,
    pub(crate) mutation_generation: u64,
    pub(crate) in_flight: Option<MailSplitMutationRequest>,
}
