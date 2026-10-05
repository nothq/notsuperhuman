use super::{
    alpha, div, img, list, mail_palette, px, rgb, AnyElement, AppearanceMode, Context, Div,
    FluentBuilder, FontWeight, InteractiveElement, IntoElement, ListSizingBehavior,
    MailListAttachment, MailListText, MailListThread, MailRowAction, MailTriageControl,
    MailTriageFlags, MouseButton, MouseDownEvent, ParentElement, Styled, SurfaceState,
    MAIL_ROW_HEIGHT, MAIL_SEARCH_FONT_FAMILY, MAIL_SECTION_LABEL_LEFT, MAIL_SECTION_LABEL_RIGHT,
};
use gpui::{font, HighlightStyle, StyledText};

mod actions;
mod ordinary;
mod search;

impl SurfaceState {
    pub(crate) fn render_mail_list(&self, cx: &mut Context<Self>) -> AnyElement {
        let threads = self.mail_list_threads.clone();
        if threads.is_empty() {
            return self.render_mail_empty_list();
        }
        let list_state = if self.mail_search_open() {
            self.mail_search_list_state.clone()
        } else {
            self.mail_list_state.clone()
        };
        let list_view = cx.entity();
        div()
            .id("mail-list-scroll")
            .flex_grow(1.0)
            .min_h(px(0.0))
            .overflow_hidden()
            .when(self.mail_search_open(), |this| this.px(px(6.0)).pt(px(2.0)))
            .when(!self.mail_search_open(), |this| {
                this.px(px(6.0)).pt(px(2.0))
            })
            .pb(px(20.0))
            .child(
                list(list_state, move |index, _window, cx| {
                    let threads = threads.clone();
                    list_view.update(cx, |this, cx| {
                        let thread = threads
                            .get(index)
                            .expect("mail thread row index should exist");
                        this.render_mail_thread_group(index, thread, cx)
                    })
                })
                .with_sizing_behavior(ListSizingBehavior::Auto)
                .size_full(),
            )
            .into_any_element()
    }

    fn render_mail_empty_list(&self) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        let search_loading = self
            .mail_search_session()
            .is_some_and(|session| session.in_flight.is_some());
        let message = if search_loading {
            None
        } else if self.mail_search_submitted_query().is_some() {
            Some("No conversations found.")
        } else if self.mail_search_open() {
            None
        } else {
            Some("You're all caught up.")
        };
        div()
            .flex_grow(1.0)
            .min_h(px(0.0))
            .when_some(message, |this, message| {
                this.child(
                    div()
                        .mx(px(6.0))
                        .mt(px(3.0))
                        .h(px(MAIL_ROW_HEIGHT))
                        .pl(px(MAIL_SECTION_LABEL_LEFT))
                        .pr(px(30.0))
                        .py(px(10.0))
                        .font(font(MAIL_SEARCH_FONT_FAMILY))
                        .text_size(px(12.0))
                        .line_height(px(16.0))
                        .font_weight(FontWeight::NORMAL)
                        .text_color(alpha(palette.text_rgb, 0.5))
                        .child(message),
                )
            })
            .into_any_element()
    }

    pub(crate) fn render_mail_thread_group(
        &self,
        index: usize,
        thread: &MailListThread,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if thread.section_label.is_none()
            || (index == 0 && thread.section_label.as_deref() == Some("Today"))
        {
            return self.render_mail_thread_row(thread, cx);
        }

        let palette = mail_palette(self.appearance_mode);
        div()
            .w_full()
            .flex()
            .flex_col()
            .when_some(thread.section_label.as_ref(), |this, section_label| {
                this.child(
                    div()
                        .pl(px(MAIL_SECTION_LABEL_LEFT))
                        .pr(px(MAIL_SECTION_LABEL_RIGHT))
                        .pt(px(if index == 0 { 10.0 } else { 22.0 }))
                        .pb(px(10.0))
                        .text_size(px(12.0))
                        .line_height(px(16.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(alpha(palette.text_rgb, 0.35))
                        .child(section_label.clone()),
                )
            })
            .child(self.render_mail_thread_row(thread, cx))
            .into_any_element()
    }

    pub(crate) fn render_mail_thread_row(
        &self,
        thread: &MailListThread,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let selected = self.mail_selected_thread_id.as_deref() == Some(thread.id.as_str());
        let hovered = self.mail_hovered_thread_id.as_deref() == Some(thread.id.as_str());
        if self.mail_search_open() {
            self.render_mail_search_thread_row(thread, selected, hovered, cx)
        } else {
            self.render_mail_thread_row_ordinary(thread, selected, hovered, cx)
        }
    }

    fn render_mail_thread_attachment_icon(&self, cx: &mut Context<Self>) -> AnyElement {
        let icon = mail_palette(self.appearance_mode).icon_rgb;
        img(self.render_mail_svg_icon(
            "0 0 18 8",
            format!(
                r##"<path fill="#{icon:06X}" d="M15 2H6c-1.1 0-2 .9-2 2s.9 2 2 2h8V5H6c-.6 0-1-.4-1-1s.4-1 1-1h9c1.1 0 2 .9 2 2s-.9 2-2 2H4C2.3 7 1 5.7 1 4s1.3-3 3-3h10V0H4C1.8 0 0 1.8 0 4s1.8 4 4 4h11c1.7 0 3-1.3 3-3s-1.3-3-3-3z"></path>"##
            ),
            cx,
        ))
        .opacity(0.3)
        .w(px(19.0))
        .h(px(8.0))
        .into_any_element()
    }

    pub(crate) fn render_mail_row_icon(
        &self,
        view_box: &str,
        body: impl Into<String>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        img(self.render_mail_svg_icon(view_box, body, cx))
            .opacity(0.3)
            .size(px(14.0))
            .into_any_element()
    }

    fn render_mail_search_highlighted_text(&self, value: &MailListText) -> StyledText {
        StyledText::new(value.text.clone()).with_highlights(
            value.highlight_ranges.iter().cloned().map(|range| {
                (
                    range,
                    HighlightStyle {
                        background_color: Some(rgb(0x635a36).into()),
                        ..Default::default()
                    },
                )
            }),
        )
    }
}

pub(crate) fn mail_thread_row_background(
    appearance_mode: AppearanceMode,
    selected: bool,
    hovered: bool,
) -> gpui::Hsla {
    let palette = mail_palette(appearance_mode);
    if selected {
        alpha(palette.selected_row_bg, 1.0)
    } else if hovered {
        alpha(palette.hover_row_bg, 1.0)
    } else {
        alpha(palette.hover_row_bg, 0.0)
    }
}
