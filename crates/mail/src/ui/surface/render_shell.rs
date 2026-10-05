use super::{
    alpha, div, img, mail_palette, px, rgb, AnyElement, Context, Div, FluentBuilder, FontWeight,
    InteractiveElement, IntoElement, MailComposeMode, MailTab, MouseButton, MouseDownEvent,
    ParentElement, Styled, SurfaceState, MAIL_TABBAR_HEIGHT,
};

const MAIL_TABBAR_LEFT_PADDING: f32 = 8.5;
const MAIL_TABBAR_RIGHT_PADDING: f32 = 24.0;

impl SurfaceState {
    pub(crate) fn render_mail_shell(&self, cx: &mut Context<Self>) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        let shell_content = if self.mail_switching_account_id.is_some() {
            self.render_mail_account_switching_content()
        } else if self.mail_open_thread_id.is_some() {
            (self.render_mail_open_thread_view(cx)).into_any_element()
        } else if self.mail_compose_mode != MailComposeMode::Closed {
            (self.render_mail_compose_view(cx)).into_any_element()
        } else if self.mail_search_open() {
            self.render_mail_search_content(cx)
        } else {
            (self.render_mail_list(cx)).into_any_element()
        };
        let shell_header = if self.mail_search_input_visible() {
            self.render_mail_search_header(cx)
        } else {
            self.render_mail_tabbar(cx).into_any_element()
        };
        (div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(alpha(palette.text_rgb, 0.09))
            .bg(rgb(palette.shell_bg))
            .when(
                (self.mail_open_thread_id.is_none()
                    && self.mail_compose_mode == MailComposeMode::Closed)
                    || self.mail_switching_account_id.is_some(),
                |this| this.child(shell_header),
            )
            .child(shell_content))
        .into_any_element()
    }

    pub(crate) fn render_mail_tabbar(&self, cx: &mut Context<Self>) -> Div {
        let palette = mail_palette(self.appearance_mode);
        let icon = palette.icon_rgb;
        let tabs = if self.mail_switching_account_id.is_some() {
            Vec::new()
        } else {
            self.mail_tabs()
        };
        div()
            .h(px(MAIL_TABBAR_HEIGHT))
            .min_h(px(MAIL_TABBAR_HEIGHT))
            .pl(px(MAIL_TABBAR_LEFT_PADDING))
            .pr(px(MAIL_TABBAR_RIGHT_PADDING))
            .pt(px(8.0))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(14.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(14.0))
                    .child(self.render_mail_menu_button(cx))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(22.0))
                            .children(tabs.iter().map(|tab| self.render_mail_tab(tab, cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(14.0))
                    .child(self.render_mail_compose_action_icon(cx))
                    .child(self.render_mail_search_action_icon(icon, cx)),
            )
    }

    pub(crate) fn render_mail_menu_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let icon = mail_palette(self.appearance_mode).icon_rgb;
        div()
            .id("mail-menu")
            .debug_selector(|| "mail-menu".to_string())
            .w(px(36.0))
            .h(px(32.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.toggle_mail_folder_drawer(cx);
                    cx.focus_self(window);
                }),
            )
            .child(
                img(self.render_mail_svg_icon(
                    "0 0 18 18",
                    format!(
                        r##"<path d="M2.5 4.5H15.5" fill="none" stroke="#{icon:06X}" stroke-width="1.35" stroke-linecap="round"/><path d="M2.5 9H15.5" fill="none" stroke="#{icon:06X}" stroke-width="1.35" stroke-linecap="round"/><path d="M2.5 13.5H15.5" fill="none" stroke="#{icon:06X}" stroke-width="1.35" stroke-linecap="round"/>"##
                    ),
                    cx,
                ))
                .opacity(0.3)
                .w(px(15.0))
                .h(px(15.0)),
            )
            .into_any_element()
    }

    fn render_mail_compose_action_icon(&self, cx: &mut Context<Self>) -> AnyElement {
        let icon = mail_palette(self.appearance_mode).text_rgb;
        let enabled = self.mail_active_account_can_submit();
        (div()
                .id("mail-compose".to_string())
                .debug_selector(|| "mail-compose".to_string())
                .size(px(18.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .when(enabled, |this| this.cursor_pointer())
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _: &MouseDownEvent, _, cx| {
                        if this.mail_active_account_can_submit() {
                            this.open_new_mail_composer(cx);
                        }
                    }),
                )
                .child(
                    img(self.render_mail_svg_icon(
                        "0 0 14 14",
                        format!(
                            r##"<path d="M13.5625 13.0625C13.8386 13.0625 14.0625 13.2864 14.0625 13.5625C14.0624 13.8385 13.8386 14.0625 13.5625 14.0625H7.4375C7.1616 13.0623 6.93763 13.8384 6.9375 13.5625C6.9375 13.2865 7.16152 13.0627 7.4375 13.0625H13.5625ZM10.001 0.620117C10.3427 0.278391 10.8966 0.278412 11.2383 0.620117L13.3799 2.76172C13.7212 3.10344 13.7214 3.65743 13.3799 3.99902L3.8623 13.5176L3.65332 13.7266L3.58496 13.7891C3.41898 13.9251 3.20962 13.9999 2.99316 14H0.483398C0.483398 14 0.454307 14.0019 0.416992 14.001L0.299805 13.9883C0.265433 13.9795 0.231747 13.9656 0.201172 13.9482L0.117188 13.8838C0.0668928 13.8335 0.0304255 13.77 0.0126953 13.7012C-0.00429492 13.6346 -0.00121805 13.5252 -0.000976562 13.5176L0 11.0078C0.000102712 10.7605 0.098528 10.5226 0.273438 10.3477L0.483398 10.1387L10.001 0.620117ZM1.19043 10.8457L0.999023 11.0361V13H2.96582L3.15527 12.8105L12.584 3.37988L10.6191 1.41504L1.19043 10.8457Z" fill="#{icon:06X}"/>"##,
                            icon = icon,
                        ),
                        cx,
                    ))
                    .opacity(if enabled { 0.3 } else { 0.12 })
                    .w(px(14.0))
                    .h(px(14.0)),
                )).into_any_element()
    }

    fn render_mail_search_action_icon(&self, icon: u32, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("mail-search")
            .debug_selector(|| "mail-search".to_string())
            .size(px(18.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.open_mail_search(cx);
                }),
            )
            .child(
                img(self.render_mail_svg_icon(
                    "0 0 18 18",
                    format!(
                        r##"<circle cx="8" cy="8" r="5.5" fill="none" stroke="#{icon:06X}" stroke-width="1.35"/><path d="M12.4 12.4L16 16" fill="none" stroke="#{icon:06X}" stroke-width="1.35" stroke-linecap="round"/>"##
                    ),
                    cx,
                ))
                .opacity(0.3)
                .w(px(15.0))
                .h(px(15.0)),
            )
            .into_any_element()
    }

    pub(crate) fn render_mail_tab(&self, tab: &MailTab, cx: &mut Context<Self>) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        let selected = self.mail_list_source == tab.source;
        let tab_source = tab.source.clone();
        (div()
            .debug_selector({
                let debug_id = tab.source.debug_id().to_string();
                move || format!("mail-tab-{debug_id}")
            })
            .py(px(2.0))
            .cursor_pointer()
            .flex()
            .items_baseline()
            .gap(px(5.0))
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
                    .max_w(px(120.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(px(16.0))
                    .line_height(px(24.0))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(alpha(palette.text_rgb, if selected { 0.8 } else { 0.4 }))
                    .child(tab.label.clone()),
            )
            .when(!tab.count.is_empty(), |this| {
                this.child(
                    div()
                        .text_size(px(14.0))
                        .line_height(px(20.0))
                        .font_weight(FontWeight::NORMAL)
                        .text_color(alpha(palette.text_rgb, if selected { 0.36 } else { 0.24 }))
                        .child(tab.count.clone()),
                )
            }))
        .into_any_element()
    }
}
