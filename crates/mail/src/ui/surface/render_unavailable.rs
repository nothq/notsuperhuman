use super::{
    alpha, div, img, mail_palette, px, rgb, AnyElement, Arc, Context, FluentBuilder, FontWeight,
    Image, IntoElement, MailComposeMode, MailStartup, ParentElement, Styled,
    SurfaceState, MAIL_ACTIVITY_MAX_WIDTH, MAIL_ACTIVITY_MIN_WIDTH, MAIL_FONT_FAMILY,
    MAIL_PREVIEW_MIN_WIDTH, MAIL_ROW_HEIGHT, MAIL_SECTION_LABEL_LEFT, MAIL_TABBAR_HEIGHT,
};
use gpui::{
    font, linear_color_stop, linear_gradient, relative, ImageFormat, InteractiveElement, ObjectFit,
    StatefulInteractiveElement, StyledImage,
};
use std::sync::OnceLock;

const MAIL_ACTIVITY_WIDTH_RATIO: f32 = 0.2583;

impl SurfaceState {
    pub(crate) fn render_mail_unavailable(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.mail_sign_in.is_some() {
            return self.render_mail_sign_in_screen(cx);
        }
        match &self.mail_startup {
            MailStartup::Loading { .. } => self.render_mail_loading_surface(),
            MailStartup::SignedOut => self.render_mail_sign_in_screen(cx),
            MailStartup::Error { error, .. } => self.render_mail_startup_error(error, cx),
            #[cfg(any(test, feature = "test-support"))]
            MailStartup::Fixture => self.render_mail_startup_error("Mail is unavailable", cx),
            MailStartup::Ready(_) => self.render_mail_startup_error("Mail is unavailable", cx),
        }
    }

    pub(crate) fn mail_inbox_zero_visible(&self) -> bool {
        self.mail_list_threads.is_empty()
            && self.mail_search_session().is_none()
            && self.mail_switching_account_id.is_none()
            && self.mail_open_thread_id.is_none()
            && self.mail_compose_mode == MailComposeMode::Closed
    }

    pub(crate) fn render_mail_inbox_zero_surface(
        &self,
        activity_width: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .min_h(px(0.0))
            .relative()
            .overflow_hidden()
            .child(render_mail_inbox_zero_backdrop())
            .child(
                div()
                    .relative()
                    .h(px(MAIL_TABBAR_HEIGHT))
                    .child(self.render_mail_tabbar(cx)),
            )
            .child(
                div()
                    .absolute()
                    .left(px(58.5))
                    .bottom(px(16.5))
                    .font(font(MAIL_FONT_FAMILY))
                    .text_size(px(12.0))
                    .line_height(px(16.0))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(alpha(0xffffff, 0.8))
                    .child("You've hit Inbox Zero!"),
            )
            .child(
                div()
                    .absolute()
                    .right_0()
                    .bottom_0()
                    .w(px(activity_width))
                    .child(self.render_mail_activity_footer(cx)),
            )
            .into_any_element()
    }

    pub(crate) fn render_mail_error_toast(&self, error: &str) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        div()
            .absolute()
            .left(px(28.0))
            .bottom(px(24.0))
            .max_w(px(520.0))
            .rounded(px(4.0))
            .px(px(12.0))
            .py(px(8.0))
            .bg(rgb(palette.tooltip_bg))
            .font(font(MAIL_FONT_FAMILY))
            .text_size(px(12.0))
            .line_height(px(16.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(palette.tooltip_label))
            .child(error.to_string())
            .into_any_element()
    }

    fn render_mail_loading_surface(&self) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        let preview_width = self.preview_width.max(MAIL_PREVIEW_MIN_WIDTH);
        let activity_width = (preview_width * MAIL_ACTIVITY_WIDTH_RATIO)
            .clamp(MAIL_ACTIVITY_MIN_WIDTH, MAIL_ACTIVITY_MAX_WIDTH);
        let skeleton = alpha(palette.text_rgb, 0.08);
        let top_inset = self.mail_surface_top_inset();
        div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .min_h(px(0.0))
            .overflow_hidden()
            .flex()
            .relative()
            .font(font(MAIL_FONT_FAMILY))
            .bg(rgb(palette.surface_bg))
            .pt(px(top_inset))
            .when(top_inset > 0.0, |this| {
                this.child(self.render_mail_activity_backdrop(activity_width))
            })
            .child(
                div()
                    .flex_grow(1.0)
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .border_l_1()
                    .border_color(alpha(palette.text_rgb, 0.09))
                    .bg(rgb(palette.shell_bg))
                    .child(render_mail_loading_header(skeleton))
                    .child(render_mail_loading_rows(skeleton)),
            )
            .child(
                div()
                    .w(px(activity_width))
                    .flex_none()
                    .border_l_1()
                    .border_color(rgb(palette.activity_border))
                    .bg(rgb(palette.activity_bg))
                    .child(render_mail_loading_sidebar(skeleton)),
            )
            .into_any_element()
    }

    fn render_mail_startup_error(&self, error: &str, cx: &mut Context<Self>) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .min_h(px(0.0))
            .flex()
            .items_center()
            .justify_center()
            .font(font(MAIL_FONT_FAMILY))
            .bg(rgb(palette.surface_bg))
            .child(
                div()
                    .w(px(420.0))
                    .rounded(px(8.0))
                    .px(px(28.0))
                    .py(px(24.0))
                    .bg(rgb(palette.message_card_bg))
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(16.0))
                            .line_height(px(24.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(alpha(palette.text_rgb, 0.9))
                            .child("Mail couldn't load"),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .line_height(px(18.0))
                            .text_color(alpha(palette.text_rgb, 0.5))
                            .child(error.to_string()),
                    )
                    .child(
                        div()
                            .pt(px(4.0))
                            .text_size(px(12.0))
                            .line_height(px(16.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(alpha(palette.text_rgb, 0.8))
                            .child("Press R to retry"),
                    )
                    .child(
                        div()
                            .id("mail-reconnect")
                            .cursor_pointer()
                            .pt(px(8.0))
                            .text_size(px(13.0))
                            .text_color(rgb(palette.compose_caret))
                            .on_click(
                                cx.listener(|this, _, _, cx| this.open_mail_sign_in(false, cx)),
                            )
                            .child("Connect an account"),
                    ),
            )
            .into_any_element()
    }
}

fn mail_inbox_zero_image() -> Arc<Image> {
    static IMAGE: OnceLock<Arc<Image>> = OnceLock::new();
    IMAGE
        .get_or_init(|| {
            Arc::new(Image::from_bytes(
                ImageFormat::Png,
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/images/inbox-zero-botanical.png"
                ))
                .to_vec(),
            ))
        })
        .clone()
}

fn render_mail_inbox_zero_backdrop() -> AnyElement {
    div()
        .absolute()
        .inset_0()
        .overflow_hidden()
        .child(
            img(mail_inbox_zero_image())
                .absolute()
                .inset_0()
                .size_full()
                .object_fit(ObjectFit::Cover),
        )
        .child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .top_0()
                .h(px(324.0))
                .bg(linear_gradient(
                    180.0,
                    linear_color_stop(alpha(0x000000, 0.58), 0.0),
                    linear_color_stop(alpha(0x000000, 0.0), 1.0),
                )),
        )
        .child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .bottom_0()
                .h(px(324.0))
                .bg(linear_gradient(
                    180.0,
                    linear_color_stop(alpha(0x000000, 0.0), 0.0),
                    linear_color_stop(alpha(0x000000, 0.68), 1.0),
                )),
        )
        .into_any_element()
}

fn render_mail_loading_header(skeleton: gpui::Hsla) -> AnyElement {
    div()
        .h(px(MAIL_TABBAR_HEIGHT))
        .min_h(px(MAIL_TABBAR_HEIGHT))
        .px(px(18.0))
        .flex()
        .items_center()
        .gap(px(18.0))
        .child(div().w(px(18.0)).h(px(2.0)).rounded(px(1.0)).bg(skeleton))
        .child(div().w(px(94.0)).h(px(12.0)).rounded(px(2.0)).bg(skeleton))
        .child(div().w(px(54.0)).h(px(12.0)).rounded(px(2.0)).bg(skeleton))
        .into_any_element()
}

fn render_mail_loading_rows(skeleton: gpui::Hsla) -> AnyElement {
    div()
        .flex_grow(1.0)
        .min_h(px(0.0))
        .px(px(6.0))
        .pt(px(2.0))
        .children((0..14).map(|index| {
            div()
                .h(px(MAIL_ROW_HEIGHT))
                .pl(px(MAIL_SECTION_LABEL_LEFT))
                .pr(px(30.0))
                .flex()
                .items_center()
                .gap(px(30.0))
                .child(
                    div()
                        .w(px(if index % 3 == 0 { 132.0 } else { 176.0 }))
                        .h(px(10.0))
                        .rounded(px(2.0))
                        .bg(skeleton),
                )
                .child(
                    div()
                        .flex_grow(1.0)
                        .h(px(10.0))
                        .rounded(px(2.0))
                        .bg(skeleton),
                )
                .child(div().w(px(42.0)).h(px(10.0)).rounded(px(2.0)).bg(skeleton))
        }))
        .into_any_element()
}

fn render_mail_loading_sidebar(skeleton: gpui::Hsla) -> AnyElement {
    div()
        .h_full()
        .px(px(30.0))
        .pt(px(30.0))
        .flex()
        .flex_col()
        .gap(px(14.0))
        .child(div().w(px(118.0)).h(px(12.0)).rounded(px(2.0)).bg(skeleton))
        .child(div().w(px(78.0)).h(px(10.0)).rounded(px(2.0)).bg(skeleton))
        .child(div().w_full().h(px(10.0)).rounded(px(2.0)).bg(skeleton))
        .child(
            div()
                .w(relative(0.72))
                .h(px(10.0))
                .rounded(px(2.0))
                .bg(skeleton),
        )
        .into_any_element()
}
