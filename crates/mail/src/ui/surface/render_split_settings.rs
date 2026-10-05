use super::{
    alpha, div, mail_palette, px, rgb, AnyElement, Context, FluentBuilder, FontWeight,
    InteractiveElement, IntoElement, MailSplitId, MailSplitMove, MouseButton, MouseDownEvent,
    ParentElement, Styled, SurfaceState,
};
use gpui_components::backdrop::{dismissible_backdrop, BackdropDismissal};

mod content;

const MAIL_SPLIT_SETTINGS_WIDTH: f32 = 640.0;

#[derive(Clone)]
enum MailSplitSettingsAction {
    Close,
    New,
    Edit(MailSplitId),
    Toggle(MailSplitId),
    Move(MailSplitId, MailSplitMove),
    CancelEditor,
    SaveEditor,
}

impl SurfaceState {
    pub(crate) fn render_mail_split_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        let panel_left = ((self.preview_width - MAIL_SPLIT_SETTINGS_WIDTH) * 0.5).max(18.0);
        div()
            .id("mail-split-settings")
            .absolute()
            .top(px(0.0))
            .right(px(0.0))
            .bottom(px(0.0))
            .left(px(0.0))
            .occlude()
            .child(dismissible_backdrop(
                div()
                    .absolute()
                    .top(px(0.0))
                    .right(px(0.0))
                    .bottom(px(0.0))
                    .left(px(0.0))
                    .bg(alpha(0x090a0d, 0.42)),
                BackdropDismissal::new(|this: &mut SurfaceState, _, _, cx| {
                    this.close_mail_split_settings(cx);
                }),
                cx,
            ))
            .child(
                div()
                    .id("mail-split-settings-panel")
                    .absolute()
                    .top(px(32.0))
                    .bottom(px(32.0))
                    .left(px(panel_left))
                    .w(px(MAIL_SPLIT_SETTINGS_WIDTH))
                    .overflow_hidden()
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(alpha(palette.text_rgb, 0.12))
                    .bg(rgb(palette.shell_bg))
                    .flex()
                    .flex_col()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_, _: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                        }),
                    )
                    .child(self.render_mail_split_settings_header(cx))
                    .child(self.render_mail_split_settings_body(cx)),
            )
            .into_any_element()
    }

    fn render_mail_split_settings_header(&self, cx: &mut Context<Self>) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        let close_enabled = self
            .mail_split_settings
            .as_ref()
            .is_some_and(|settings| settings.in_flight.is_none());
        div()
            .h(px(68.0))
            .min_h(px(68.0))
            .px(px(22.0))
            .border_b_1()
            .border_color(alpha(palette.text_rgb, 0.09))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(3.0))
                    .child(
                        div()
                            .text_size(px(16.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(alpha(palette.text_rgb, 0.9))
                            .child("Split Inboxes"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(alpha(palette.text_rgb, 0.42))
                            .child("Named searches are always scoped to this account's Inbox"),
                    ),
            )
            .child(self.render_mail_split_settings_button(
                "Close",
                MailSplitSettingsAction::Close,
                close_enabled,
                cx,
            ))
            .into_any_element()
    }

    fn render_mail_split_settings_button(
        &self,
        label: &str,
        action: MailSplitSettingsAction,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        div()
            .px(px(8.0))
            .h(px(26.0))
            .rounded(px(3.0))
            .border_1()
            .border_color(alpha(palette.text_rgb, if enabled { 0.14 } else { 0.07 }))
            .flex()
            .items_center()
            .justify_center()
            .when(enabled, |this| {
                this.cursor_pointer()
                    .hover(|style| style.bg(alpha(palette.text_rgb, 0.06)))
            })
            .text_size(px(10.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(alpha(palette.text_rgb, if enabled { 0.62 } else { 0.22 }))
            .child(label.to_string())
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    if enabled {
                        this.activate_mail_split_settings_action(action.clone(), cx);
                    }
                }),
            )
            .into_any_element()
    }

    fn activate_mail_split_settings_action(
        &mut self,
        action: MailSplitSettingsAction,
        cx: &mut Context<Self>,
    ) {
        match action {
            MailSplitSettingsAction::Close => {
                self.close_mail_split_settings(cx);
            }
            MailSplitSettingsAction::New => self.begin_new_mail_split(cx),
            MailSplitSettingsAction::Edit(id) => self.begin_edit_mail_split(id, cx),
            MailSplitSettingsAction::Toggle(id) => {
                self.toggle_mail_split_enabled(id, cx);
            }
            MailSplitSettingsAction::Move(id, direction) => {
                self.move_mail_split(id, direction, cx);
            }
            MailSplitSettingsAction::CancelEditor => {
                self.cancel_mail_split_editor(cx);
            }
            MailSplitSettingsAction::SaveEditor => {
                self.save_mail_split_editor(cx);
            }
        }
    }
}
