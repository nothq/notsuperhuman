use super::{
    alpha, div, if_light, mail_palette, point, px, rgb, BoxShadow, Context, Div, FluentBuilder,
    FontWeight, InteractiveElement, ParentElement, Styled, SurfaceState, MAIL_FONT_FAMILY,
};
use crate::ui::{
    AppearanceMode, MailImageResolver, MailRenderOptions, MailTextMeasure,
    MAIL_RICH_BODY_SIDE_PADDING, MAIL_RICH_BODY_WIDTH,
};
use gpui::Stateful;

mod attachments;

pub(crate) struct MailMessageCardContentFrameState {
    pub(crate) thread_id: String,
    pub(crate) message_id: String,
    pub(crate) card_index: usize,
    pub(crate) rich_layout: bool,
    pub(crate) first: bool,
    pub(crate) bottom_padding: f32,
}

impl SurfaceState {
    pub(crate) fn render_mail_open_message_card_border(&self) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .absolute()
            .left(px(0.0))
            .top(px(0.0))
            .w(px(4.0))
            .h_full()
            .bg(rgb(palette.message_card_border))
    }

    pub(crate) fn mail_open_message_render_options(&self, rich_layout: bool) -> MailRenderOptions {
        mail_message_render_options(
            self.appearance_mode,
            rich_layout,
            Some(self.mail_image_resolver()),
            self.text_measure.clone(),
        )
    }

    pub(crate) fn render_mail_open_message_card_row_shell(
        &self,
        _thread_id: &str,
        first: bool,
        last: bool,
        _cx: &mut Context<Self>,
    ) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .relative()
            .border_l_1()
            .border_r_1()
            .border_color(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.05, 0.08),
            ))
            .bg(rgb(palette.message_card_bg))
            .when(first, |this| {
                this.border_t_1()
                    .rounded_t(px(4.0))
                    .shadow(vec![BoxShadow {
                        color: alpha(0x000000, if_light(self.appearance_mode, 0.12, 0.28)),
                        offset: point(px(0.0), px(8.0)),
                        blur_radius: px(24.0),
                        spread_radius: px(-10.0),
                        inset: false,
                    }])
                    .overflow_hidden()
            })
            .when(last, |this| {
                this.border_b_1().rounded_b(px(4.0)).overflow_hidden()
            })
            .child(self.render_mail_open_message_card_border())
    }

    pub(crate) fn render_open_message_card_content_frame(
        &self,
        frame: MailMessageCardContentFrameState,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let hover_message_card_id =
            mail_message_card_id(frame.thread_id.as_str(), frame.message_id.as_str());
        let selector_thread_id = frame.thread_id;
        let selector_message_id = frame.message_id;
        let mut content = div()
            .id(format!(
                "mail-message-card-{selector_thread_id}-{selector_message_id}-{}",
                frame.card_index,
            ))
            .debug_selector(move || {
                format!(
                    "mail-message-card-{selector_thread_id}-{selector_message_id}-{}",
                    frame.card_index,
                )
            })
            .relative()
            .pl(px(if frame.rich_layout {
                MAIL_RICH_BODY_SIDE_PADDING
            } else {
                30.0
            }))
            .pr(px(if frame.rich_layout {
                MAIL_RICH_BODY_SIDE_PADDING
            } else {
                28.0
            }))
            .pt(px(if frame.first { 18.0 } else { 0.0 }))
            .pb(px(frame.bottom_padding));
        content
            .interactivity()
            .on_hover(cx.listener(move |this, is_hovered, _, cx| {
                this.set_mail_message_card_hover(hover_message_card_id.clone(), *is_hovered, cx);
            }));
        content
    }

    pub(crate) fn render_mail_open_message_card_auto_responses(&self) -> Div {
        div()
            .pt(px(2.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(self.render_mail_auto_response_chip("Thanks for the gift!", "↑"))
            .child(self.render_mail_auto_response_chip("Will share it!", "↑"))
            .child(self.render_mail_auto_response_chip("Not interested, thanks", "↓"))
    }

    pub(crate) fn render_mail_auto_response_chip(&self, label: &str, indicator: &str) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .h(px(24.0))
            .max_w(px(146.0))
            .px(px(8.0))
            .rounded(px(3.0))
            .border_1()
            .border_color(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.08, 0.12),
            ))
            .bg(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.02, 0.05),
            ))
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(
                div()
                    .flex_none()
                    .text_size(px(11.0))
                    .text_color(alpha(
                        palette.text_rgb,
                        if_light(self.appearance_mode, 0.38, 0.34),
                    ))
                    .child(indicator.to_string()),
            )
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .text_size(px(12.0))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(alpha(
                        palette.text_rgb,
                        if_light(self.appearance_mode, 0.58, 0.52),
                    ))
                    .text_ellipsis()
                    .child(label.to_string()),
            )
    }

    pub(crate) fn render_mail_open_message_card_signature(
        &self,
        signature_lines: Vec<String>,
    ) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .children(
                signature_lines
                    .into_iter()
                    .enumerate()
                    .map(|(index, line)| {
                        div()
                            .text_size(px(if index == 0 { 13.0 } else { 12.0 }))
                            .font_weight(if index == 0 {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(alpha(
                                palette.text_rgb,
                                if index < 2 {
                                    0.68
                                } else {
                                    if_light(self.appearance_mode, 0.5, 0.44)
                                },
                            ))
                            .child(line)
                    }),
            )
    }
}

pub(crate) fn mail_message_render_options(
    appearance_mode: AppearanceMode,
    rich_layout: bool,
    image_resolver: Option<MailImageResolver>,
    measure_text: MailTextMeasure,
) -> MailRenderOptions {
    let palette = mail_palette(appearance_mode);
    MailRenderOptions {
        // Styled HTML is drawn the way a browser draws it, whatever the app's
        // theme: black text and blue links on the white canvas that
        // `render_mail_document` paints. An email that sets its own dark text
        // and no background was otherwise unreadable in dark mode.
        text_color: if rich_layout {
            rgb(0x000000).into()
        } else {
            alpha(palette.text_rgb, 0.72)
        },
        muted_text_color: alpha(palette.text_rgb, if_light(appearance_mode, 0.48, 0.42)),
        link_color: if rich_layout {
            rgb(0x0000ee).into()
        } else {
            alpha(0x2f79ff, if_light(appearance_mode, 0.92, 0.9))
        },
        border_color: alpha(palette.text_rgb, if_light(appearance_mode, 0.1, 0.14)),
        placeholder_bg: alpha(palette.text_rgb, if_light(appearance_mode, 0.04, 0.08)),
        font_family: if rich_layout {
            "Times".into()
        } else {
            MAIL_FONT_FAMILY.into()
        },
        // A browser's defaults for HTML; the plain-text card keeps its own.
        base_font_size: if rich_layout { 16.0 } else { 14.0 },
        line_height: if rich_layout { 18.4 } else { 19.0 },
        content_width: mail_body_content_width(rich_layout),
        image_resolver,
        measure_text,
    }
}

/// The body width notsuperhuman renders an email at.
pub(crate) fn mail_body_content_width(rich_layout: bool) -> f32 {
    if rich_layout {
        MAIL_RICH_BODY_WIDTH
    } else {
        500.0
    }
}

/// Measures one line with the window's text system. Automatic table layout needs
/// real font metrics; without them column widths cannot be resolved.
pub(crate) fn mail_text_measure(
    text_system: std::sync::Arc<gpui::WindowTextSystem>,
) -> MailTextMeasure {
    std::sync::Arc::new(move |text: &str, font: &crate::ui::MailTextFont| {
        if text.is_empty() {
            return 0.0;
        }
        let run = gpui::TextRun {
            len: text.len(),
            font: mail_gpui_font(font),
            color: gpui::Hsla::default(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        f32::from(
            text_system
                .shape_line(text.to_string().into(), px(font.size), &[run], None)
                .width,
        )
    })
}

fn mail_gpui_font(font: &crate::ui::MailTextFont) -> gpui::Font {
    gpui::Font {
        family: font.family.clone(),
        features: Default::default(),
        weight: crate::ui::render::font_weight(font.weight),
        style: if font.italic {
            gpui::FontStyle::Italic
        } else {
            gpui::FontStyle::Normal
        },
        fallbacks: None,
    }
}

pub(super) fn mail_message_card_id(thread_id: &str, message_id: &str) -> String {
    format!("{thread_id}:{message_id}")
}
