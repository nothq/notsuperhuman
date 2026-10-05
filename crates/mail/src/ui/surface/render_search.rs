use super::render_sidebar::mail_sidebar_shell;
use super::{
    alpha, div, img, mail_palette, mail_search_list_text, px, rgb, AnyElement, Context, Div,
    FluentBuilder, FontWeight, InteractiveElement, IntoElement, MailListText, MouseButton,
    MouseDownEvent, ParentElement, Styled, SurfaceState, MAIL_SEARCH_FONT_FAMILY,
};
use crate::ui::MailAddress;
use gpui::{font, HighlightStyle, StyledText};

const MAIL_SEARCH_HEADER_HEIGHT: f32 = 76.0;
const MAIL_SEARCH_TEXT_INSET: f32 = 52.5;
const MAIL_SEARCH_CONTACT_NAME_WIDTH: f32 = 193.867;

const MAIL_SEARCH_TIPS: [(&str, &str); 16] = [
    ("from:nicole", "from Nicole"),
    ("to:roman", "to Roman"),
    ("\"be brilliant\"", "contains \"be brilliant\""),
    ("has:attachment", "with attachments"),
    ("subject:lunch", "subject contains \"lunch\""),
    ("in:sent", "in Sent"),
    ("in:inbox", "in the Inbox"),
    ("-in:inbox", "not in the Inbox"),
    ("in:fundraising", "in this label"),
    ("is:unread", "unread conversations"),
    ("is:starred", "starred conversations"),
    ("is:shared", "shared conversations"),
    ("before:2017/06/01", "before June 2017"),
    ("after:2017/06/01", "June 2017 or later"),
    ("older_than:3d", "more than 3 days ago"),
    ("newer_than:1m", "1 month ago or later"),
];

impl SurfaceState {
    pub(crate) fn render_mail_search_header(&self, cx: &mut Context<Self>) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        div()
            .id("mail-search-header")
            .mx(px(6.0))
            .h(px(MAIL_SEARCH_HEADER_HEIGHT))
            .min_h(px(MAIL_SEARCH_HEADER_HEIGHT))
            .relative()
            .pt(px(28.0))
            .pl(px(MAIL_SEARCH_TEXT_INSET))
            .pr(px(MAIL_SEARCH_TEXT_INSET))
            .when(self.mail_search_input_focused(), |this| {
                this.child(
                    div()
                        .absolute()
                        .left(px(0.0))
                        .top(px(28.0))
                        .w(px(3.0))
                        .h(px(28.0))
                        .bg(rgb(0xb3b5df)),
                )
            })
            .child(self.render_mail_search_input_layer(palette.text_rgb, cx))
            .child(self.render_mail_search_close(palette.icon_rgb, cx))
            .into_any_element()
    }

    fn render_mail_search_input_layer(&self, text_rgb: u32, cx: &mut Context<Self>) -> Div {
        div()
            .h(px(28.0))
            .w_full()
            .min_w(px(0.0))
            .relative()
            .when_some(self.mail_search_completion(), |this, (typed, suffix)| {
                this.child(
                    div()
                        .absolute()
                        .left(px(0.0))
                        .top(px(0.0))
                        .h(px(28.0))
                        .flex()
                        .items_center()
                        .font(font(MAIL_SEARCH_FONT_FAMILY))
                        .text_size(px(16.0))
                        .line_height(px(28.0))
                        .child(div().flex_none().text_color(rgb(text_rgb)).child(typed))
                        .child(
                            div()
                                .flex_none()
                                .text_color(alpha(text_rgb, 0.28))
                                .child(suffix),
                        ),
                )
            })
            .child(self.mail_search_input_entity(cx))
    }

    fn render_mail_search_close(&self, icon: u32, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("mail-search-close")
            .debug_selector(|| "mail-search-close".to_string())
            .absolute()
            .right(px(15.0))
            .top(px(25.0))
            .w(px(30.0))
            .h(px(34.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.close_mail_search(cx);
                    this.mail_shortcuts_focused = true;
                    cx.focus_self(window);
                }),
            )
            .child(
                img(self.render_mail_svg_icon(
                    "0 0 14 14",
                    format!(
                        r##"<path d="M2 2L12 12M12 2L2 12" fill="none" stroke="#{icon:06X}" stroke-width="1.25" stroke-linecap="round"/>"##
                    ),
                    cx,
                ))
                .opacity(0.45)
                .size(px(14.0)),
            )
            .into_any_element()
    }

    pub(crate) fn render_mail_search_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let session = self
            .mail_search_session()
            .expect("mail search content requires an open search session");
        if session.submitted_query.is_none() {
            return self.render_mail_search_suggestions(cx);
        }
        if session.messages.is_empty() {
            if session.in_flight.is_some() {
                return div().flex_grow(1.0).min_h(px(0.0)).into_any_element();
            }
            let palette = mail_palette(self.appearance_mode);
            return div()
                .mx(px(6.0))
                .h(px(36.0))
                .min_h(px(36.0))
                .pl(px(MAIL_SEARCH_TEXT_INSET))
                .pr(px(30.0))
                .pt(px(10.0))
                .pb(px(10.0))
                .font(font(MAIL_SEARCH_FONT_FAMILY))
                .text_size(px(12.0))
                .line_height(px(16.0))
                .text_color(alpha(palette.text_rgb, 0.5))
                .child("No conversations found.")
                .into_any_element();
        }
        self.render_mail_list(cx)
    }

    fn render_mail_search_suggestions(&self, cx: &mut Context<Self>) -> AnyElement {
        let text_rgb = mail_palette(self.appearance_mode).text_rgb;
        let terms = self
            .mail_search_session()
            .map(|session| session.raw_query.trim())
            .filter(|query| !query.is_empty())
            .map(|query| vec![query.to_string()])
            .unwrap_or_default();
        div()
            .id("mail-search-suggestions")
            .mx(px(6.0))
            .pt(px(2.0))
            .flex_grow(1.0)
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .children(self.mail_search_contacts().into_iter().map(|contact| {
                self.render_mail_search_contact(contact, terms.as_slice(), text_rgb, cx)
            }))
            .into_any_element()
    }

    fn render_mail_search_contact(
        &self,
        contact: MailAddress,
        terms: &[String],
        text_rgb: u32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let submitted_query = format!("{} ", contact.email);
        let name = if contact.name.is_empty() {
            contact.email.clone()
        } else {
            contact.name.clone()
        };
        let name = mail_search_list_text(name, terms);
        let email = mail_search_list_text(contact.email.clone(), terms);
        div()
            .id(format!("mail-search-contact-{}", contact.email))
            .h(px(36.0))
            .min_h(px(36.0))
            .pl(px(MAIL_SEARCH_TEXT_INSET))
            .pr(px(20.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .font(font(MAIL_SEARCH_FONT_FAMILY))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.submit_mail_search_text(submitted_query.clone(), cx);
                    this.mail_shortcuts_focused = true;
                    cx.focus_self(window);
                }),
            )
            .child(self.render_mail_search_contact_name(&name, text_rgb))
            .child(self.render_mail_search_contact_email(&email, text_rgb))
            .into_any_element()
    }

    fn render_mail_search_contact_name(&self, name: &MailListText, text_rgb: u32) -> Div {
        div()
            .w(px(MAIL_SEARCH_CONTACT_NAME_WIDTH))
            .min_w(px(MAIL_SEARCH_CONTACT_NAME_WIDTH))
            .max_w(px(MAIL_SEARCH_CONTACT_NAME_WIDTH))
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .text_size(px(12.0))
            .line_height(px(16.0))
            .font_weight(FontWeight::BOLD)
            .text_color(alpha(text_rgb, 0.85))
            .child(self.render_mail_search_contact_text(name, alpha(text_rgb, 1.0)))
    }

    fn render_mail_search_contact_email(&self, email: &MailListText, text_rgb: u32) -> Div {
        div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .text_size(px(12.0))
            .line_height(px(16.0))
            .font_weight(FontWeight::BOLD)
            .text_color(alpha(text_rgb, 0.4))
            .child(self.render_mail_search_contact_text(email, alpha(text_rgb, 0.85)))
    }

    fn render_mail_search_contact_text(
        &self,
        text: &MailListText,
        highlight_color: gpui::Hsla,
    ) -> StyledText {
        StyledText::new(text.text.clone()).with_highlights(
            text.highlight_ranges.iter().cloned().map(move |range| {
                (
                    range,
                    HighlightStyle {
                        color: Some(highlight_color),
                        ..Default::default()
                    },
                )
            }),
        )
    }

    pub(crate) fn render_mail_search_blank_sidebar(&self, width: f32) -> Div {
        mail_sidebar_shell(width, mail_palette(self.appearance_mode))
    }

    pub(crate) fn render_mail_search_tips_sidebar(
        &self,
        width: f32,
        cx: &mut Context<Self>,
    ) -> Div {
        let palette = mail_palette(self.appearance_mode);
        mail_sidebar_shell(width, palette)
            .overflow_hidden()
            .child(
                div()
                    .px(px(31.0))
                    .pt(px(32.0))
                    .child(
                        div()
                            .pb(px(24.0))
                            .font(font(MAIL_SEARCH_FONT_FAMILY))
                            .text_size(px(16.0))
                            .line_height(px(24.0))
                            .font_weight(FontWeight::NORMAL)
                            .text_color(alpha(palette.text_rgb, 0.85))
                            .child("Tips"),
                    )
                    .child(div().pt(px(8.0)).flex().flex_col().gap(px(16.0)).children(
                        MAIL_SEARCH_TIPS.into_iter().map(|(prompt, description)| {
                            div()
                                .h(px(32.0))
                                .min_h(px(32.0))
                                .flex()
                                .flex_col()
                                .font(font(MAIL_SEARCH_FONT_FAMILY))
                                .text_size(px(12.0))
                                .line_height(px(16.0))
                                .text_color(alpha(palette.text_rgb, 0.65))
                                .child(div().font_weight(FontWeight::BOLD).child(prompt))
                                .child(div().font_weight(FontWeight::NORMAL).child(description))
                        }),
                    )),
            )
            .child(div().flex_grow(1.0))
            .child(self.render_mail_activity_footer(cx))
    }
}
