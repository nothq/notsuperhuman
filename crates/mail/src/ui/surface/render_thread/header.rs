use super::super::render_thread_helpers::{
    mail_subject_header_parts, mail_thread_shows_unsubscribe, mail_thread_tag,
};
use super::super::{
    alpha, div, if_light, img, mail_palette, px, rgb, AnyElement, Context, Div, FluentBuilder,
    FontWeight, InteractiveElement, IntoElement, MailPalette, MailRowAction, MailThreadDetail,
    MailTriageControl, MailTriageFlags, MouseButton, MouseDownEvent, ParentElement, Styled,
    SurfaceState, MAIL_FONT_FAMILY, MAIL_OPEN_HEADER_HEIGHT,
};
use gpui::font;

struct MailOpenThreadHeaderIcon {
    id: &'static str,
    view_box: &'static str,
    body: String,
    width: f32,
    height: f32,
    delta: isize,
}

impl SurfaceState {
    pub(crate) fn render_mail_open_thread_header(
        &self,
        thread: &MailThreadDetail,
        cx: &mut Context<Self>,
    ) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .relative()
            .h(px(MAIL_OPEN_HEADER_HEIGHT))
            .min_w(px(0.0))
            .flex_none()
            .bg(rgb(palette.viewer_bg))
            .flex()
            .justify_center()
            .child(
                div()
                    .relative()
                    .w_full()
                    .max_w(px(760.0))
                    .h_full()
                    .overflow_hidden()
                    .px(px(60.0))
                    .pt(px(24.0))
                    .pb(px(20.0))
                    .child(self.render_mail_open_thread_subject_row(thread, cx))
                    .child(self.render_mail_open_thread_summary_row(thread)),
            )
    }

    fn render_mail_open_thread_header_icon(
        &self,
        icon: MailOpenThreadHeaderIcon,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .id(icon.id.to_string())
            .w(px(24.0))
            .h(px(32.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .child(
                img(self.render_mail_svg_icon(icon.view_box, icon.body, cx))
                    .opacity(0.3)
                    .w(px(icon.width))
                    .h(px(icon.height)),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.select_adjacent_mail_thread(icon.delta, cx);
                    this.mail_shortcuts_focused = true;
                    cx.focus_self(window);
                }),
            )
            .into_any_element()
    }

    pub(crate) fn render_mail_open_thread_back_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let icon = mail_palette(self.appearance_mode).icon_rgb;
        (div()
                .id("mail-thread-back".to_string())
                .absolute()
                .left(px(30.0))
                .top(px(25.0))
                .w(px(32.0))
                .h(px(32.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .child(
                    img(self.render_mail_svg_icon(
                        "0 0 16 16",
                        format!(
                            r##"<path d="M8.75 1L1.75 8.00011M1.75 8.00011L8.75 15M1.75 8.00011L14.25 8" fill="none" stroke="#{icon:06X}" stroke-linecap="round" stroke-linejoin="round"></path>"##
                        ),
                        cx,
                    ))
                    .opacity(0.3)
                    .size(px(16.0)),
                )
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        this.close_mail_thread(cx);
                        this.mail_shortcuts_focused = true;
                        cx.focus_self(window);
                    }),
                )).into_any_element()
    }

    fn render_mail_open_thread_subject_row(
        &self,
        thread: &MailThreadDetail,
        cx: &mut Context<Self>,
    ) -> Div {
        let palette = mail_palette(self.appearance_mode);
        let (subject, ornament) = mail_subject_header_parts(thread.subject.as_str());
        div()
            .h(px(64.0))
            .w_full()
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(32.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .child(
                        self.render_mail_open_thread_subject_title(subject)
                            .whitespace_normal()
                            .line_clamp(2)
                            .flex_grow(1.0)
                            .max_w(px(406.0)),
                    )
                    .when_some(ornament, |this, ornament| {
                        this.child(
                            self.render_mail_open_thread_subject_ornament(ornament, &palette),
                        )
                    })
                    .when(mail_thread_shows_unsubscribe(thread), |this| {
                        this.child(self.render_mail_open_thread_unsubscribe(&palette))
                    })
                    .child(div().flex_grow(1.0).min_w(px(10.0)))
                    .child(self.render_mail_open_thread_actions(thread, cx)),
            )
            .when_some(thread.tag_label.as_deref(), |this, tag_label| {
                this.child(
                    div()
                        .h(px(28.0))
                        .flex()
                        .items_center()
                        .child(mail_thread_tag(tag_label, thread.tag_fill)),
                )
            })
    }

    pub(super) fn render_mail_open_thread_summary_row(&self, thread: &MailThreadDetail) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .mt(px(3.0))
            .h(px(28.0))
            .w_full()
            .overflow_hidden()
            .font(font(MAIL_FONT_FAMILY))
            .text_size(px(12.0))
            .line_height(px(20.0))
            .font_weight(FontWeight::BOLD)
            .text_color(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.35, 0.38),
            ))
            .flex()
            .items_center()
            .child(
                div()
                    .ml(px(-8.0))
                    .h(px(24.0))
                    .max_w(px(480.0))
                    .overflow_hidden()
                    .px(px(8.0))
                    .py(px(2.0))
                    .flex()
                    .items_center()
                    .text_ellipsis()
                    .child(thread.preview.clone()),
            )
    }

    fn render_mail_open_thread_subject_ornament(
        &self,
        ornament: String,
        palette: &MailPalette,
    ) -> Div {
        div()
            .text_size(px(14.0))
            .line_height(px(14.0))
            .text_color(alpha(palette.text_rgb, 0.9))
            .child(ornament)
    }

    pub(super) fn render_mail_open_thread_subject_title(&self, subject: String) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .min_w(px(0.0))
            .overflow_hidden()
            .font(font(MAIL_FONT_FAMILY))
            .text_size(px(21.0))
            .line_height(px(32.0))
            .font_weight(FontWeight::NORMAL)
            .text_color(alpha(palette.text_rgb, 0.9))
            .child(subject)
    }

    fn render_mail_open_thread_unsubscribe(&self, palette: &MailPalette) -> Div {
        div()
            .text_size(px(11.5))
            .line_height(px(14.0))
            .font_weight(FontWeight::BOLD)
            .text_color(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.35, 0.38),
            ))
            .child("Unsubscribe")
    }

    pub(crate) fn render_mail_open_thread_actions(
        &self,
        thread: &MailThreadDetail,
        cx: &mut Context<Self>,
    ) -> Div {
        let thread_id = thread.id.as_str();
        let triage = MailTriageFlags {
            unread: thread.unread,
            starred: thread.starred,
        };
        div()
            .w(px(227.0))
            .h(px(32.0))
            .flex_none()
            .flex()
            .items_center()
            .child(self.render_mail_open_thread_share_label())
            .child(div().w(px(7.0)).flex_none())
            .child(self.render_mail_triage_button(thread_id, MailTriageControl::Star, triage, cx))
            .child(self.render_mail_triage_button(thread_id, MailTriageControl::Read, triage, cx))
            .child(self.render_mail_thread_action_button(thread_id, MailRowAction::MarkDone, cx))
            .child(self.render_mail_thread_action_button(thread_id, MailRowAction::RemindMe, cx))
            .child(self.render_mail_thread_action_button(thread_id, MailRowAction::Move, cx))
            .child(self.render_mail_open_thread_navigation_icons(cx))
    }

    fn render_mail_open_thread_share_label(&self) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .ml(px(7.0))
            .h(px(22.0))
            .w(px(48.0))
            .flex()
            .items_center()
            .justify_center()
            .font(font(MAIL_FONT_FAMILY))
            .text_size(px(12.0))
            .line_height(px(16.0))
            .font_weight(FontWeight::SEMIBOLD)
            .opacity(0.4)
            .text_color(rgb(palette.text_rgb))
            .child("Share")
    }

    fn render_mail_open_thread_navigation_icons(&self, cx: &mut Context<Self>) -> Div {
        let icon = mail_palette(self.appearance_mode).icon_rgb;
        div()
            .w(px(48.0))
            .h(px(32.0))
            .flex_none()
            .flex()
            .items_center()
            .child(self.render_mail_open_thread_header_icon(
                MailOpenThreadHeaderIcon {
                    id: "mail-thread-prev",
                    view_box: "0 0 16 16",
                    body: format!(
                        r##"<path d="M1 11.5L7.63595 4.44931C7.83334 4.23958 8.16666 4.23958 8.36405 4.44931L15 11.5" fill="none" stroke="#{icon:06X}" stroke-linecap="round" stroke-linejoin="round"></path>"##
                    ),
                    width: 16.0,
                    height: 16.0,
                    delta: -1,
                },
                cx,
            ))
            .child(self.render_mail_open_thread_header_icon(
                MailOpenThreadHeaderIcon {
                    id: "mail-thread-next",
                    view_box: "0 0 16 16",
                    body: format!(
                        r##"<path d="M15 4.5L8.36405 11.5507C8.16666 11.7604 7.83334 11.7604 7.63595 11.5507L1 4.5" fill="none" stroke="#{icon:06X}" stroke-linecap="round" stroke-linejoin="round"></path>"##
                    ),
                    width: 16.0,
                    height: 16.0,
                    delta: 1,
                },
                cx,
            ))
    }
}
