use super::{
    alpha, div, if_light, mail_palette, px, rgb, AnyElement, Context, Div, FluentBuilder,
    FontWeight, InteractiveElement, IntoElement, MailComposeAutocompleteItem, MailComposeField,
    MailComposeRecipientRowRequest, MailPalette, MouseButton, MouseDownEvent, ParentElement,
    Styled, SurfaceState,
};
use crate::ui::surface::MAIL_FONT_FAMILY;
use crate::ui::{LongFormEditorStyle, MailComposeTextInputRequest, TextInputMode, TextInputStyle};

struct MailComposeRecipientRenderState<'a> {
    placeholder: &'static str,
    palette: &'a MailPalette,
    full_view: bool,
    focused: bool,
    value: String,
    suggestions: Vec<MailComposeAutocompleteItem>,
    selected_index: usize,
}

impl SurfaceState {
    pub(super) fn render_mail_compose_from_row(&self) -> Div {
        let palette = mail_palette(self.appearance_mode);
        let from = self.mail_compose_from.as_ref();
        div()
            .min_h(px(42.0))
            .py(px(10.0))
            .flex()
            .items_center()
            .gap(px(18.0))
            .child(self.render_mail_compose_field_label("From", false, false))
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .text_ellipsis()
                    .text_size(px(13.5))
                    .text_color(alpha(
                        palette.text_rgb,
                        if_light(self.appearance_mode, 0.88, 0.82),
                    ))
                    .when_some(from, |this, from| this.child(mail_compose_from_label(from)))
                    .when(from.is_none(), |this| {
                        this.child(
                            div()
                                .h(px(8.0))
                                .w(px(172.0))
                                .rounded(px(4.0))
                                .bg(alpha(palette.text_rgb, 0.07)),
                        )
                    }),
            )
    }

    pub(super) fn render_mail_compose_recipient_row(
        &self,
        request: MailComposeRecipientRowRequest,
        cx: &mut Context<Self>,
    ) -> Div {
        let MailComposeRecipientRowRequest {
            field,
            label,
            placeholder,
            full_view,
        } = request;
        let palette = mail_palette(self.appearance_mode);
        let focused = self.mail_compose_focused_field == field;
        let value = self.mail_compose_field_value(field).to_string();
        let suggestions = if focused {
            self.mail_compose_autocomplete_items()
        } else {
            Vec::new()
        };
        let state = MailComposeRecipientRenderState {
            placeholder,
            palette: &palette,
            full_view,
            focused,
            value,
            selected_index: self
                .mail_compose_autocomplete_selected_index
                .min(suggestions.len().saturating_sub(1)),
            suggestions,
        };
        div()
            .relative()
            .h(px(if full_view { 40.0 } else { 42.0 }))
            .min_h(px(if full_view { 40.0 } else { 42.0 }))
            .pt(px(if full_view { 4.0 } else { 10.0 }))
            .pb(px(if full_view { 8.0 } else { 10.0 }))
            .flex()
            .items_start()
            .gap(px(if full_view { 0.0 } else { 18.0 }))
            .child(self.render_mail_compose_field_label(label, true, full_view))
            .child(self.render_mail_compose_recipient_content(field, &state, cx))
    }

    fn render_mail_compose_recipient_content(
        &self,
        field: MailComposeField,
        state: &MailComposeRecipientRenderState<'_>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        (div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .pt(px(1.0))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                    this.focus_mail_compose_field(field, cx);
                }),
            )
            .child(self.render_mail_compose_recipient_value(field, state, cx))
            .when(!state.suggestions.is_empty(), |this| {
                this.child(self.render_mail_compose_autocomplete(
                    &state.suggestions,
                    state.selected_index,
                    cx,
                ))
            }))
        .into_any_element()
    }

    fn render_mail_compose_recipient_value(
        &self,
        field: MailComposeField,
        state: &MailComposeRecipientRenderState<'_>,
        cx: &mut Context<Self>,
    ) -> Div {
        div()
            .min_h(px(22.0))
            .flex()
            .items_center()
            .child(self.render_mail_compose_text_input(
                MailComposeTextInputRequest {
                    field,
                    value: state.value.clone(),
                    placeholder: state.placeholder.to_string(),
                    mode: TextInputMode::SingleLine,
                    style: self.mail_compose_line_input_style(
                        state.palette,
                        state.focused,
                        state.full_view,
                        if state.full_view { 27.0 } else { 22.0 },
                    ),
                },
                cx,
            ))
    }

    pub(super) fn render_mail_compose_subject_row(
        &self,
        full_view: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let palette = mail_palette(self.appearance_mode);
        let focused = self.mail_compose_focused_field == MailComposeField::Subject;
        let row = div()
            .h(px(if full_view { 34.0 } else { 42.0 }))
            .min_h(px(if full_view { 34.0 } else { 42.0 }))
            .when(!full_view, |this| this.pt(px(10.0)).pb(px(10.0)))
            .flex()
            .items_center()
            .gap(px(if full_view { 0.0 } else { 18.0 }));
        row.when(!full_view, |this| {
            this.child(self.render_mail_compose_field_label("Subject", false, false))
        })
        .child(self.render_mail_compose_subject_content(&palette, focused, full_view, cx))
    }

    fn render_mail_compose_subject_content(
        &self,
        palette: &MailPalette,
        focused: bool,
        full_view: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        (div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.focus_mail_compose_field(MailComposeField::Subject, cx);
                }),
            )
            .child(self.render_mail_compose_text_input(
                MailComposeTextInputRequest {
                    field: MailComposeField::Subject,
                    value: self.mail_compose_subject.clone(),
                    placeholder: "Subject".to_string(),
                    mode: TextInputMode::SingleLine,
                    style: self.mail_compose_line_input_style(
                        palette,
                        focused,
                        full_view,
                        if full_view { 34.0 } else { 22.0 },
                    ),
                },
                cx,
            )))
        .into_any_element()
    }

    pub(super) fn render_mail_compose_body_field(
        &self,
        full_view: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let palette = mail_palette(self.appearance_mode);
        let focused = self.mail_compose_focused_field == MailComposeField::Body;
        div()
            .when(full_view, |this| {
                this.flex_grow(1.0).min_h(px(0.0)).pt(px(20.0))
            })
            .when(!full_view, |this| this.min_h(px(132.0)).pt(px(16.0)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.focus_mail_compose_field(MailComposeField::Body, cx);
                }),
            )
            .child(self.render_mail_compose_body_content(full_view, &palette, focused, cx))
    }

    fn render_mail_compose_body_content(
        &self,
        full_view: bool,
        palette: &MailPalette,
        focused: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let min_height = if full_view { 196.0 } else { 116.0 };
        (div().flex().items_start().w_full().child(
            self.mail_compose_body_editor_entity(
                self.mail_compose_body.clone(),
                if full_view {
                    "Tip: Hit ⌘J for AI"
                } else {
                    "Write a reply..."
                },
                self.mail_compose_body_editor_style(palette, focused, min_height),
                cx,
            )
            .into_any_element(),
        ))
        .into_any_element()
    }

    fn render_mail_compose_field_label(
        &self,
        label: &str,
        top_padding: bool,
        full_view: bool,
    ) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .w(px(if full_view { 50.0 } else { 64.0 }))
            .when(top_padding, |this| {
                this.pt(px(if full_view { 1.0 } else { 3.0 }))
            })
            .flex_none()
            .text_size(px(if full_view { 14.0 } else { 11.5 }))
            .line_height(px(if full_view { 20.0 } else { 16.0 }))
            .font_weight(if full_view {
                FontWeight::BOLD
            } else {
                FontWeight::SEMIBOLD
            })
            .text_color(alpha(
                palette.text_rgb,
                if full_view {
                    if_light(self.appearance_mode, 0.76, 0.8)
                } else {
                    if_light(self.appearance_mode, 0.44, 0.4)
                },
            ))
            .child(label.to_string())
    }

    fn render_mail_compose_text_input(
        &self,
        request: MailComposeTextInputRequest,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.mail_compose_text_input_entity(request, cx)
            .into_any_element()
    }

    fn mail_compose_line_input_style(
        &self,
        palette: &MailPalette,
        focused: bool,
        full_view: bool,
        height: f32,
    ) -> TextInputStyle {
        TextInputStyle {
            height: px(height),
            min_height: px(height),
            padding_x: px(0.0),
            padding_y: px(0.0),
            radius: px(0.0),
            background: alpha(palette.text_rgb, 0.0),
            border: alpha(palette.text_rgb, 0.0),
            focused_border: alpha(palette.text_rgb, 0.0),
            text: alpha(palette.text_rgb, if_light(self.appearance_mode, 0.88, 0.82)),
            placeholder: alpha(
                palette.text_rgb,
                if focused {
                    if_light(self.appearance_mode, 0.34, 0.3)
                } else {
                    if_light(self.appearance_mode, 0.3, 0.26)
                },
            ),
            selection: alpha(palette.compose_caret, 0.28),
            caret: rgb(palette.compose_caret).into(),
            font_size: px(if full_view { 14.0 } else { 13.5 }),
            line_height: px(20.0),
            font_family: Some(MAIL_FONT_FAMILY.into()),
        }
    }

    fn mail_compose_body_editor_style(
        &self,
        palette: &MailPalette,
        focused: bool,
        min_height: f32,
    ) -> LongFormEditorStyle {
        LongFormEditorStyle {
            height: px(min_height),
            min_height: px(min_height),
            padding_x: px(0.0),
            padding_y: px(0.0),
            radius: px(0.0),
            background: alpha(palette.text_rgb, 0.0),
            border: alpha(palette.text_rgb, 0.0),
            focused_border: alpha(palette.text_rgb, 0.0),
            text: alpha(palette.text_rgb, if_light(self.appearance_mode, 0.86, 0.8)),
            placeholder: alpha(
                palette.text_rgb,
                if focused {
                    if_light(self.appearance_mode, 0.34, 0.3)
                } else {
                    if_light(self.appearance_mode, 0.3, 0.26)
                },
            ),
            selection: alpha(palette.compose_caret, 0.28),
            caret: rgb(palette.compose_caret).into(),
            font_size: px(13.0),
            line_height: px(20.0),
            font_family: Some(MAIL_FONT_FAMILY.into()),
            ..LongFormEditorStyle::default()
        }
    }
}

fn mail_compose_from_label(from: &crate::ui::MailAddress) -> String {
    if from.name.trim().is_empty() {
        from.email.clone()
    } else {
        format!("{} <{}>", from.name, from.email)
    }
}
