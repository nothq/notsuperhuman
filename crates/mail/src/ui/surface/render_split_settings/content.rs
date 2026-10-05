use super::super::{
    alpha, div, mail_palette, px, rgb, AnyElement, Context, FluentBuilder, FontWeight,
    InteractiveElement, IntoElement, MailSplitEditorState, MailSplitEditorTarget,
    MailSplitInputField, MailSplitMove, ParentElement, StatefulInteractiveElement, Styled,
    SurfaceState,
};
use super::MailSplitSettingsAction;
use crate::model::{MailSplitDefinition, MAIL_SPLIT_MAX_COUNT};

impl SurfaceState {
    pub(super) fn render_mail_split_settings_body(&self, cx: &mut Context<Self>) -> AnyElement {
        let settings = self
            .mail_split_settings
            .as_ref()
            .expect("Mail split settings body requires open settings");
        let busy = settings.in_flight.is_some();
        div()
            .id("mail-split-settings-body")
            .flex_grow(1.0)
            .min_h(px(0.0))
            .overflow_y_scroll()
            .px(px(22.0))
            .py(px(18.0))
            .flex()
            .flex_col()
            .gap(px(12.0))
            .when(self.mail_split_definitions.is_empty(), |this| {
                this.child(self.render_mail_split_empty_state())
            })
            .children(
                self.mail_split_definitions
                    .iter()
                    .enumerate()
                    .map(|(index, split)| {
                        self.render_mail_split_settings_row(index, split, busy, cx)
                    }),
            )
            .when_some(settings.editor.as_ref(), |this, editor| {
                this.child(self.render_mail_split_editor(editor, busy, cx))
            })
            .when_some(settings.error.as_deref(), |this, error| {
                this.child(self.render_mail_split_settings_error(error))
            })
            .when(
                settings.editor.is_none()
                    && self.mail_split_definitions.len() < MAIL_SPLIT_MAX_COUNT,
                |this| this.child(self.render_mail_split_add_button(!busy, cx)),
            )
            .into_any_element()
    }

    fn render_mail_split_empty_state(&self) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        div()
            .py(px(12.0))
            .text_size(px(12.0))
            .text_color(alpha(palette.text_rgb, 0.42))
            .child("No named splits yet. Other currently contains the whole Inbox.")
            .into_any_element()
    }

    fn render_mail_split_settings_error(&self, error: &str) -> AnyElement {
        div()
            .rounded(px(3.0))
            .px(px(10.0))
            .py(px(8.0))
            .bg(alpha(0xc34a4a, 0.1))
            .text_size(px(11.0))
            .text_color(rgb(0xc34a4a))
            .child(error.to_string())
            .into_any_element()
    }

    fn render_mail_split_add_button(&self, enabled: bool, cx: &mut Context<Self>) -> AnyElement {
        self.render_mail_split_settings_button(
            "Add split",
            MailSplitSettingsAction::New,
            enabled,
            cx,
        )
    }

    fn render_mail_split_settings_row(
        &self,
        index: usize,
        split: &MailSplitDefinition,
        busy: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        div()
            .id(format!("mail-split-settings-row-{}", split.id()))
            .min_h(px(58.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(alpha(palette.text_rgb, 0.09))
            .px(px(12.0))
            .py(px(9.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .child(self.render_mail_split_settings_row_summary(split))
            .child(self.render_mail_split_settings_row_actions(index, split, busy, cx))
            .into_any_element()
    }

    fn render_mail_split_settings_row_summary(&self, split: &MailSplitDefinition) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        div()
            .min_w(px(0.0))
            .flex_grow(1.0)
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .max_w(px(220.0))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(alpha(palette.text_rgb, 0.82))
                            .child(split.name().to_string()),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(alpha(
                                palette.text_rgb,
                                if split.is_enabled() { 0.42 } else { 0.26 },
                            ))
                            .child(if split.is_enabled() {
                                "Enabled"
                            } else {
                                "Disabled"
                            }),
                    ),
            )
            .child(
                div()
                    .max_w(px(310.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(px(10.5))
                    .text_color(alpha(palette.text_rgb, 0.4))
                    .child(split.query().as_str().to_string()),
            )
            .into_any_element()
    }

    fn render_mail_split_settings_row_actions(
        &self,
        index: usize,
        split: &MailSplitDefinition,
        busy: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = split.id().clone();
        let toggle_label = if split.is_enabled() {
            "Disable"
        } else {
            "Enable"
        };
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(self.render_mail_split_settings_button(
                "↑",
                MailSplitSettingsAction::Move(id.clone(), MailSplitMove::Earlier),
                !busy && index > 0,
                cx,
            ))
            .child(self.render_mail_split_settings_button(
                "↓",
                MailSplitSettingsAction::Move(id.clone(), MailSplitMove::Later),
                !busy && index + 1 < self.mail_split_definitions.len(),
                cx,
            ))
            .child(self.render_mail_split_settings_button(
                toggle_label,
                MailSplitSettingsAction::Toggle(id.clone()),
                !busy,
                cx,
            ))
            .child(self.render_mail_split_settings_button(
                "Edit",
                MailSplitSettingsAction::Edit(id),
                !busy,
                cx,
            ))
            .into_any_element()
    }

    fn render_mail_split_editor(
        &self,
        editor: &MailSplitEditorState,
        busy: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        let title = match &editor.target {
            MailSplitEditorTarget::New => "New split",
            MailSplitEditorTarget::Existing(_) => "Edit split",
        };
        div()
            .rounded(px(4.0))
            .border_1()
            .border_color(alpha(palette.compose_caret, 0.32))
            .p(px(12.0))
            .flex()
            .flex_col()
            .gap(px(9.0))
            .child(
                div()
                    .text_size(px(12.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(alpha(palette.text_rgb, 0.8))
                    .child(title),
            )
            .child(
                self.mail_split_text_input_entity(MailSplitInputField::Name, cx)
                    .into_any_element(),
            )
            .child(
                self.mail_split_text_input_entity(MailSplitInputField::Query, cx)
                    .into_any_element(),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(7.0))
                    .child(self.render_mail_split_settings_button(
                        "Cancel",
                        MailSplitSettingsAction::CancelEditor,
                        !busy,
                        cx,
                    ))
                    .child(self.render_mail_split_settings_button(
                        "Save",
                        MailSplitSettingsAction::SaveEditor,
                        !busy,
                        cx,
                    )),
            )
            .into_any_element()
    }
}
