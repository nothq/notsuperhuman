use super::super::{
    MailListSource, MailSplitEditorState, MailSplitEditorTarget, MailSplitInputField,
    MailSplitMutationRequest, MailSplitSettingsState,
};
use crate::ui::{
    Context, MailSplitDraft, MailSplitEnabled, MailSplitId, MailSplitMove, MailSplitMutation,
    SurfaceState,
};

mod input;

impl SurfaceState {
    pub(crate) fn open_mail_split_settings(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.mail_split_preferences_supported {
            return false;
        }
        let Some(workspace_api) = self.mail_workspace_api() else {
            return false;
        };
        let definitions = match workspace_api.load_mail_split_definitions() {
            Ok(definitions) => definitions,
            Err(error) => {
                self.report_mail_error(error);
                cx.notify();
                return true;
            }
        };
        self.mail_split_definitions = definitions;
        self.mail_split_settings_generation = self.mail_split_settings_generation.wrapping_add(1);
        self.mail_split_settings = Some(MailSplitSettingsState {
            generation: self.mail_split_settings_generation,
            editor: None,
            error: None,
            mutation_generation: 0,
            in_flight: None,
        });
        self.mail_split_text_inputs.borrow_mut().clear();
        self.mail_folder_drawer_open = false;
        self.mail_action_palette = None;
        self.mail_account_palette = None;
        self.mail_shortcuts_focused = false;
        cx.notify();
        true
    }

    pub(crate) fn close_mail_split_settings(&mut self, cx: &mut Context<Self>) -> bool {
        if self
            .mail_split_settings
            .as_ref()
            .is_some_and(|settings| settings.in_flight.is_some())
        {
            return false;
        }
        if self.mail_split_settings.take().is_none() {
            return false;
        }
        self.mail_split_text_inputs.borrow_mut().clear();
        self.mail_shortcuts_focused = true;
        cx.notify();
        true
    }

    pub(crate) fn begin_new_mail_split(&mut self, cx: &mut Context<Self>) {
        let Some(settings) = self.mail_split_settings.as_mut() else {
            return;
        };
        if settings.in_flight.is_some() {
            return;
        }
        settings.editor = Some(MailSplitEditorState {
            target: MailSplitEditorTarget::New,
            name: String::new(),
            query: String::new(),
            focused_field: MailSplitInputField::Name,
        });
        settings.error = None;
        self.mail_split_text_inputs.borrow_mut().clear();
        self.mail_shortcuts_focused = false;
        cx.notify();
    }

    pub(crate) fn begin_edit_mail_split(&mut self, id: MailSplitId, cx: &mut Context<Self>) {
        let Some(split) = self
            .mail_split_definitions
            .iter()
            .find(|split| split.id() == &id)
        else {
            return;
        };
        let editor = MailSplitEditorState {
            target: MailSplitEditorTarget::Existing(id),
            name: split.name().to_string(),
            query: split.query().as_str().to_string(),
            focused_field: MailSplitInputField::Name,
        };
        let Some(settings) = self.mail_split_settings.as_mut() else {
            return;
        };
        if settings.in_flight.is_some() {
            return;
        }
        settings.editor = Some(editor);
        settings.error = None;
        self.mail_split_text_inputs.borrow_mut().clear();
        self.mail_shortcuts_focused = false;
        cx.notify();
    }

    pub(crate) fn cancel_mail_split_editor(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(settings) = self.mail_split_settings.as_mut() else {
            return false;
        };
        if settings.in_flight.is_some() || settings.editor.take().is_none() {
            return false;
        }
        settings.error = None;
        self.mail_split_text_inputs.borrow_mut().clear();
        cx.notify();
        true
    }

    pub(crate) fn set_mail_split_editor_value(
        &mut self,
        field: MailSplitInputField,
        value: String,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self
            .mail_split_settings
            .as_mut()
            .and_then(|settings| settings.editor.as_mut())
        else {
            return;
        };
        match field {
            MailSplitInputField::Name => editor.name = value,
            MailSplitInputField::Query => editor.query = value,
        }
        if let Some(settings) = self.mail_split_settings.as_mut() {
            settings.error = None;
        }
        cx.notify();
    }

    pub(crate) fn focus_mail_split_editor_field(
        &mut self,
        field: MailSplitInputField,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self
            .mail_split_settings
            .as_mut()
            .and_then(|settings| settings.editor.as_mut())
        else {
            return;
        };
        editor.focused_field = field;
        self.mail_shortcuts_focused = false;
        cx.notify();
    }

    pub(crate) fn advance_mail_split_editor(&mut self, cx: &mut Context<Self>) {
        let Some(field) = self
            .mail_split_settings
            .as_ref()
            .and_then(|settings| settings.editor.as_ref())
            .map(|editor| editor.focused_field)
        else {
            return;
        };
        if field == MailSplitInputField::Name {
            self.focus_mail_split_editor_field(MailSplitInputField::Query, cx);
        } else {
            self.save_mail_split_editor(cx);
        }
    }

    pub(crate) fn save_mail_split_editor(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(editor) = self
            .mail_split_settings
            .as_ref()
            .and_then(|settings| settings.editor.clone())
        else {
            return false;
        };
        let draft = match MailSplitDraft::parse(&editor.name, &editor.query) {
            Ok(draft) => draft,
            Err(error) => {
                if let Some(settings) = self.mail_split_settings.as_mut() {
                    settings.error = Some(error.to_string());
                }
                cx.notify();
                return true;
            }
        };
        let mutation = match editor.target {
            MailSplitEditorTarget::New => MailSplitMutation::Create(draft),
            MailSplitEditorTarget::Existing(id) => MailSplitMutation::Edit { id, draft },
        };
        self.start_mail_split_mutation(mutation, true, cx)
    }

    pub(crate) fn toggle_mail_split_enabled(
        &mut self,
        id: MailSplitId,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(split) = self
            .mail_split_definitions
            .iter()
            .find(|split| split.id() == &id)
        else {
            return false;
        };
        let enabled = if split.is_enabled() {
            MailSplitEnabled::Disabled
        } else {
            MailSplitEnabled::Enabled
        };
        self.start_mail_split_mutation(MailSplitMutation::SetEnabled { id, enabled }, false, cx)
    }

    pub(crate) fn move_mail_split(
        &mut self,
        id: MailSplitId,
        direction: MailSplitMove,
        cx: &mut Context<Self>,
    ) -> bool {
        self.start_mail_split_mutation(MailSplitMutation::Move { id, direction }, false, cx)
    }

    fn start_mail_split_mutation(
        &mut self,
        mutation: MailSplitMutation,
        close_editor_on_success: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(workspace_api) = self.mail_workspace_api() else {
            return false;
        };
        let Some(account_id) = self
            .mail_workspace()
            .map(|workspace| workspace.account_id.clone())
        else {
            return false;
        };
        let account_generation = self.mail_account_generation;
        let selected_source = self.mail_list_source.clone();
        let Some(settings) = self.mail_split_settings.as_mut() else {
            return false;
        };
        if settings.in_flight.is_some() {
            return true;
        }
        settings.mutation_generation = settings.mutation_generation.wrapping_add(1);
        let request = MailSplitMutationRequest {
            account_generation,
            account_id,
            selected_source,
            settings_generation: settings.generation,
            mutation_generation: settings.mutation_generation,
            close_editor_on_success,
        };
        settings.error = None;
        settings.in_flight = Some(request.clone());
        cx.notify();
        self.spawn_background_task(
            (request.clone(), mutation),
            cx,
            move |(_, mutation)| workspace_api.mutate_mail_split_definitions(mutation),
            move |this, result, cx| {
                this.apply_mail_split_mutation_result(&request, result, cx);
            },
        );
        true
    }

    fn apply_mail_split_mutation_result(
        &mut self,
        request: &MailSplitMutationRequest,
        result: Result<Vec<crate::model::MailSplitDefinition>, String>,
        cx: &mut Context<Self>,
    ) {
        if !self.mail_split_mutation_is_current(request) {
            return;
        }
        if let Some(settings) = self.mail_split_settings.as_mut() {
            settings.in_flight = None;
        }
        match result {
            Ok(definitions) => {
                self.mail_split_definitions = definitions;
                if request.close_editor_on_success {
                    if let Some(settings) = self.mail_split_settings.as_mut() {
                        settings.editor = None;
                    }
                    self.mail_split_text_inputs.borrow_mut().clear();
                }
                self.reconcile_mail_split_view_after_mutation(cx);
            }
            Err(error) => {
                if let Some(settings) = self.mail_split_settings.as_mut() {
                    settings.error = Some(error);
                }
                cx.notify();
            }
        }
    }

    fn mail_split_mutation_is_current(&self, request: &MailSplitMutationRequest) -> bool {
        self.mail_account_generation == request.account_generation
            && self.mail_list_source == request.selected_source
            && self
                .mail_workspace()
                .is_some_and(|workspace| workspace.account_id == request.account_id)
            && self
                .mail_split_settings
                .as_ref()
                .and_then(|settings| settings.in_flight.as_ref())
                == Some(request)
    }

    fn reconcile_mail_split_view_after_mutation(&mut self, cx: &mut Context<Self>) {
        match self.mail_list_source.clone() {
            MailListSource::Split(id)
                if self
                    .mail_split_definitions
                    .iter()
                    .any(|split| split.id() == &id && split.is_enabled()) =>
            {
                self.open_mail_split_view(MailListSource::Split(id), cx);
            }
            MailListSource::Split(_) => {
                self.open_mail_split_view(MailListSource::Other, cx);
            }
            MailListSource::Other => {
                self.open_mail_split_view(MailListSource::Other, cx);
            }
            MailListSource::Mailbox(_) | MailListSource::Starred => {
                self.sync_mail_list_rows();
                cx.notify();
            }
        }
    }
}
