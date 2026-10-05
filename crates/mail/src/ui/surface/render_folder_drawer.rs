use super::{
    alpha, div, mail_palette, px, rgb, AnyElement, Context, FluentBuilder, FontWeight,
    InteractiveElement, IntoElement, MailTab, MouseButton, MouseDownEvent, ParentElement,
    StatefulInteractiveElement, Styled, SurfaceState,
};
use gpui_components::backdrop::{dismissible_backdrop, BackdropDismissal};

const MAIL_FOLDER_DRAWER_WIDTH: f32 = 398.6953;
const MAIL_FOLDER_DRAWER_ACCOUNT_HEIGHT: f32 = 76.0;
const MAIL_FOLDER_DRAWER_ROW_HEIGHT: f32 = 36.0;

impl SurfaceState {
    pub(crate) fn render_mail_folder_drawer(&self, cx: &mut Context<Self>) -> AnyElement {
        let top = self.mail_content_top_inset();
        div()
            .id("mail-folder-drawer")
            .debug_selector(|| "mail-folder-drawer".to_string())
            .absolute()
            .top(px(top))
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
                    .bg(alpha(0x090a0d, 0.36)),
                BackdropDismissal::new(|this: &mut SurfaceState, _, _, cx| {
                    this.close_mail_folder_drawer(cx);
                }),
                cx,
            ))
            .child(self.render_mail_folder_drawer_panel(cx))
            .into_any_element()
    }

    fn render_mail_folder_drawer_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        let tabs = self.mail_tabs();
        div()
            .id("mail-folder-drawer-panel")
            .debug_selector(|| "mail-folder-drawer-panel".to_string())
            .w(px(MAIL_FOLDER_DRAWER_WIDTH))
            .h_full()
            .flex_none()
            .overflow_hidden()
            .flex()
            .flex_col()
            .bg(rgb(palette.activity_bg))
            .border_r_1()
            .border_color(alpha(palette.text_rgb, 0.08))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                }),
            )
            .child(
                div()
                    .h(px(MAIL_FOLDER_DRAWER_ACCOUNT_HEIGHT))
                    .min_h(px(MAIL_FOLDER_DRAWER_ACCOUNT_HEIGHT))
                    .pt(px(20.0))
                    .child(self.render_mail_account_chip(cx)),
            )
            .child(
                div()
                    .id("mail-folder-drawer-rows")
                    .flex_grow(1.0)
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .children(
                        tabs.iter()
                            .map(|tab| self.render_mail_folder_drawer_row(tab, cx)),
                    ),
            )
            .into_any_element()
    }

    fn render_mail_folder_drawer_row(&self, tab: &MailTab, cx: &mut Context<Self>) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        let selected = self.mail_list_source == tab.source;
        let tab_source = tab.source.clone();
        let debug_id = tab.source.debug_id().to_string();
        div()
            .id(format!("mail-folder-drawer-row-{debug_id}"))
            .debug_selector({
                let debug_id = debug_id.clone();
                move || format!("mail-folder-drawer-row-{debug_id}")
            })
            .mx(px(6.0))
            .h(px(MAIL_FOLDER_DRAWER_ROW_HEIGHT))
            .rounded(px(2.0))
            .pl(px(46.5))
            .pr(px(24.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .when(selected, |this| this.bg(rgb(palette.selected_row_bg)))
            .hover(|style| style.bg(rgb(palette.hover_row_bg)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.set_mail_list_source(tab_source.clone(), cx);
                    this.mail_shortcuts_focused = true;
                    cx.focus_self(window);
                }),
            )
            .child(
                div()
                    .min_w(px(0.0))
                    .flex_grow(1.0)
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(px(12.0))
                    .line_height(px(16.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(alpha(palette.text_rgb, 0.75))
                    .child(tab.label.clone()),
            )
            .when(!tab.count.is_empty(), |this| {
                this.child(
                    div()
                        .flex_none()
                        .text_size(px(12.0))
                        .line_height(px(16.0))
                        .text_color(alpha(palette.text_rgb, 0.42))
                        .child(tab.count.clone()),
                )
            })
            .into_any_element()
    }
}
