use super::{
    alpha, div, if_light, img, mail_palette, mail_sender_initials, px, rgb, AnyElement, Context,
    Div, FluentBuilder, FontWeight, InteractiveElement, IntoElement, MailContactHistoryRow,
    MailContactHistoryStatus, MailFooterAction, MailListThread, MailPalette, MailThreadDetail,
    MouseButton, MouseDownEvent, ParentElement, Styled, SurfaceState,
};
use crate::ui::format_attachment_size;

mod contact;
mod footer;

impl SurfaceState {
    pub(crate) fn render_mail_activity_backdrop(&self, width: f32) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .absolute()
            .top(px(0.0))
            .right(px(0.0))
            .bottom(px(0.0))
            .w(px(width))
            .border_l_1()
            .border_color(rgb(palette.activity_border))
            .bg(rgb(palette.activity_bg))
    }

    pub(crate) fn render_mail_compose_sidebar(&self, width: f32, cx: &mut Context<Self>) -> Div {
        let palette = mail_palette(self.appearance_mode);
        let Some(from) = self.mail_compose_from.as_ref() else {
            assert!(
                self.mail_identity_loading,
                "an open Mail composer requires a From address"
            );
            return self.render_mail_compose_sidebar_loading(width, cx);
        };
        let display_name = if from.name.trim().is_empty() {
            from.email
                .split('@')
                .next()
                .expect("validated From address requires a local part")
                .to_string()
        } else {
            from.name.clone()
        };
        let header = self.render_mail_compose_sidebar_header(&display_name, &palette);
        let identity =
            self.render_mail_compose_sidebar_identity(&display_name, &from.email, &palette);
        mail_sidebar_shell(width, palette)
            .child(header)
            .child(identity)
            .child(div().flex_grow(1.0))
            .child(self.render_mail_activity_footer(cx))
    }

    fn render_mail_compose_sidebar_header(&self, display_name: &str, palette: &MailPalette) -> Div {
        div()
            .h(px(80.0))
            .px(px(30.0))
            .pt(px(32.0))
            .pb(px(24.0))
            .text_size(px(16.0))
            .line_height(px(24.0))
            .font_weight(FontWeight::NORMAL)
            .text_color(alpha(palette.text_rgb, 0.85))
            .child(display_name.to_string())
    }

    fn render_mail_compose_sidebar_identity(
        &self,
        display_name: &str,
        email: &str,
        palette: &MailPalette,
    ) -> Div {
        div()
            .px(px(30.0))
            .pt(px(8.0))
            .flex()
            .items_center()
            .gap(px(14.0))
            .child(
                div()
                    .size(px(58.0))
                    .rounded_full()
                    .bg(rgb(palette.contact_avatar_bg))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(17.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(alpha(palette.text_rgb, 0.55))
                    .child(mail_sender_initials(display_name)),
            )
            .child(
                div()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_size(px(12.0))
                            .line_height(px(16.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(alpha(palette.text_rgb, 0.8))
                            .child(email.to_string()),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .line_height(px(16.0))
                            .text_color(alpha(palette.text_rgb, 0.42))
                            .child("Sending from this account"),
                    ),
            )
    }

    fn render_mail_compose_sidebar_loading(&self, width: f32, cx: &mut Context<Self>) -> Div {
        let palette = mail_palette(self.appearance_mode);
        let skeleton = alpha(palette.text_rgb, 0.08);
        mail_sidebar_shell(width, palette)
            .child(
                div()
                    .h(px(80.0))
                    .px(px(30.0))
                    .pt(px(32.0))
                    .pb(px(24.0))
                    .child(div().w(px(116.0)).h(px(16.0)).rounded(px(2.0)).bg(skeleton)),
            )
            .child(
                div()
                    .px(px(30.0))
                    .pt(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(14.0))
                    .child(div().size(px(58.0)).rounded_full().bg(skeleton))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .child(div().w(px(152.0)).h(px(12.0)).rounded(px(2.0)).bg(skeleton))
                            .child(div().w(px(120.0)).h(px(10.0)).rounded(px(2.0)).bg(skeleton)),
                    ),
            )
            .child(div().flex_grow(1.0))
            .child(self.render_mail_activity_footer(cx))
    }

    pub(crate) fn render_mail_activity_sidebar(&self, width: f32, cx: &mut Context<Self>) -> Div {
        let palette = mail_palette(self.appearance_mode);
        if let Some(thread) = self.mail_open_thread_detail() {
            return self.render_mail_contact_sidebar(width, &thread, cx);
        }
        let Some(selected_thread) = self.mail_selected_list_thread() else {
            return mail_sidebar_shell(width, palette)
                .child(div().flex_grow(1.0))
                .child(self.render_mail_activity_footer(cx));
        };
        mail_sidebar_shell(width, palette)
            .child(self.render_mail_activity_sidebar_header("Mailbox Summary"))
            .child(self.render_mail_activity_sidebar_summary(&selected_thread))
            .child(div().flex_grow(1.0))
            .child(self.render_mail_activity_footer(cx))
    }

    pub(crate) fn render_mail_contact_sidebar(
        &self,
        width: f32,
        thread: &MailThreadDetail,
        cx: &mut Context<Self>,
    ) -> Div {
        let palette = mail_palette(self.appearance_mode);
        mail_sidebar_shell(width, palette)
            .child(self.render_mail_activity_sidebar_header(&thread.sender))
            .child(self.render_mail_contact_identity(thread))
            .child(self.render_mail_contact_context(thread, cx))
            .when(
                !self.mail_search_open() && !thread.attachments.is_empty(),
                |this| this.child(self.render_mail_attachment_sidebar(thread)),
            )
            .child(div().flex_grow(1.0))
            .child(self.render_mail_activity_footer(cx))
    }

    pub(crate) fn render_mail_activity_sidebar_header(&self, label: &str) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .px(px(28.0))
            .pt(px(24.0))
            .pb(px(8.0))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .text_size(px(16.0))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(alpha(palette.text_rgb, 0.9))
                    .text_ellipsis()
                    .child(label.to_string()),
            )
            .child(div().size(px(8.0)).rounded_full().bg(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.16, 0.24),
            )))
    }

    pub(crate) fn render_mail_activity_sidebar_summary(&self, thread: &MailListThread) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .px(px(28.0))
            .pt(px(18.0))
            .pb(px(14.0))
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(
                div()
                    .text_size(px(13.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(alpha(
                        palette.text_rgb,
                        if_light(self.appearance_mode, 0.9, 0.82),
                    ))
                    .child(thread.sender.text.clone()),
            )
            .child(
                div()
                    .text_size(px(11.5))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(alpha(
                        palette.text_rgb,
                        if_light(self.appearance_mode, 0.5, 0.38),
                    ))
                    .child(format!("{}  {}", thread.date_label, thread.subject.text)),
            )
            .child(
                div()
                    .text_size(px(11.5))
                    .line_height(px(15.0))
                    .text_color(alpha(
                        palette.text_rgb,
                        if_light(self.appearance_mode, 0.5, 0.52),
                    ))
                    .child(thread.preview.text.clone()),
            )
    }

    pub(crate) fn render_mail_contact_identity(&self, thread: &MailThreadDetail) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .px(px(28.0))
            .pt(px(10.0))
            .pb(px(14.0))
            .flex()
            .items_center()
            .gap(px(14.0))
            .child(
                div()
                    .size(px(58.0))
                    .rounded_full()
                    .bg(rgb(palette.contact_avatar_bg))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(17.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(alpha(
                        palette.text_rgb,
                        if_light(self.appearance_mode, 0.5, 0.68),
                    ))
                    .child(mail_sender_initials(&thread.sender)),
            )
            .child(
                div().min_w(px(0.0)).flex().flex_col().gap(px(4.0)).child(
                    div()
                        .overflow_hidden()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(alpha(
                            palette.text_rgb,
                            if_light(self.appearance_mode, 0.78, 0.72),
                        ))
                        .text_ellipsis()
                        .child(thread.sender_email.clone()),
                ),
            )
    }

    pub(crate) fn render_mail_attachment_sidebar(&self, thread: &MailThreadDetail) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .px(px(28.0))
            .pt(px(18.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .text_size(px(10.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(alpha(palette.text_rgb, 0.3))
                    .child("Attachments"),
            )
            .children(thread.attachments.iter().map(|attachment| {
                div()
                    .text_size(px(11.5))
                    .line_height(px(15.0))
                    .text_color(alpha(
                        palette.text_rgb,
                        if_light(self.appearance_mode, 0.52, 0.46),
                    ))
                    .child(format!(
                        "{}  {}",
                        attachment.name,
                        format_attachment_size(attachment.size)
                    ))
            }))
    }
}

pub(crate) fn mail_sidebar_shell(width: f32, palette: MailPalette) -> Div {
    div()
        .w(px(width))
        .min_w(px(width))
        .max_w(px(width))
        .h_full()
        .flex_none()
        .border_l_1()
        .border_color(rgb(palette.activity_border))
        .bg(rgb(palette.activity_bg))
        .flex()
        .flex_col()
}
