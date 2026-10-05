use super::{
    mail_account_display_name, mail_account_initials, MailAccountPaletteRender,
    MAIL_ACCOUNT_ROW_HEIGHT,
};
use crate::model::MailAccountInfo;
use crate::ui::{
    alpha, div, px, AnyElement, Context, Div, FluentBuilder, FontWeight, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, ParentElement, Styled, SurfaceState,
};
use gpui::font;

struct MailAccountRowRender<'a> {
    index: usize,
    slot: usize,
    account: &'a MailAccountInfo,
    is_selected: bool,
    is_active: bool,
}

impl SurfaceState {
    pub(super) fn render_mail_account_palette_sections(
        &self,
        render: &MailAccountPaletteRender<'_>,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        render
            .options
            .iter()
            .enumerate()
            .map(|(index, (slot, account))| {
                self.render_mail_account_palette_row(
                    MailAccountRowRender {
                        index,
                        slot: *slot,
                        account,
                        is_selected: index == render.selected_index,
                        is_active: render.active_account_id == Some(account.id.as_str()),
                    },
                    cx,
                )
            })
            .collect()
    }

    fn render_mail_account_palette_row(
        &self,
        render: MailAccountRowRender<'_>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let account_id = render.account.id.clone();
        let selector_account_id = account_id.clone();
        let index = render.index;
        let mut row = div()
            .id(format!("mail-account-row-{account_id}"))
            .debug_selector(move || format!("mail-account-row-{selector_account_id}"))
            .h(px(MAIL_ACCOUNT_ROW_HEIGHT))
            .mb(px(8.0))
            .pl(px(34.0))
            .pr(px(36.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .bg(alpha(0xffffff, if render.is_selected { 0.1 } else { 0.0 }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                    if let Some(state) = this.mail_account_palette.as_mut() {
                        state.selected_index = index;
                    }
                    this.switch_mail_account(account_id.clone(), cx);
                }),
            )
            .child(self.render_mail_account_palette_row_body(
                render.account,
                render.slot,
                render.is_active,
            ));
        row.interactivity()
            .on_hover(cx.listener(move |this, hovered, _, cx| {
                if *hovered {
                    if let Some(state) = this.mail_account_palette.as_mut() {
                        state.selected_index = index;
                    }
                    cx.notify();
                }
            }));
        row.into_any_element()
    }

    fn render_mail_account_palette_row_body(
        &self,
        account: &MailAccountInfo,
        slot: usize,
        is_active: bool,
    ) -> Div {
        let status = if account.is_read_only {
            Some("Read only")
        } else if !account.can_submit {
            Some("No sending")
        } else {
            None
        };
        div()
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.0))
            .child(self.render_mail_account_palette_identity(account))
            .child(self.render_mail_account_palette_status(slot, status, is_active))
    }

    fn render_mail_account_palette_identity(&self, account: &MailAccountInfo) -> Div {
        div()
            .min_w(px(0.0))
            .flex_grow(1.0)
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(self.render_mail_account_palette_avatar(account))
            .child(self.render_mail_account_palette_address(account))
    }

    fn render_mail_account_palette_avatar(&self, account: &MailAccountInfo) -> Div {
        div()
            .size(px(24.0))
            .rounded_full()
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .bg(alpha(0xffffff, 0.18))
            .text_size(px(9.5))
            .line_height(px(12.0))
            .font_weight(FontWeight::BOLD)
            .text_color(alpha(0xffffff, 0.9))
            .child(mail_account_initials(account))
    }

    fn render_mail_account_palette_address(&self, account: &MailAccountInfo) -> Div {
        div()
            .min_w(px(0.0))
            .flex_grow(1.0)
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .font(font("SF Mono"))
            .text_size(px(16.0))
            .line_height(px(20.0))
            .font_weight(FontWeight::NORMAL)
            .text_color(alpha(0xffffff, 0.9))
            .child(
                account
                    .address
                    .clone()
                    .unwrap_or_else(|| mail_account_display_name(account)),
            )
    }

    fn render_mail_account_palette_status(
        &self,
        slot: usize,
        status: Option<&'static str>,
        is_active: bool,
    ) -> Div {
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(8.0))
            .when_some(status, |this, status| {
                this.child(
                    div()
                        .mr(px(4.0))
                        .text_size(px(12.0))
                        .line_height(px(16.0))
                        .text_color(alpha(0xffffff, 0.55))
                        .child(status),
                )
            })
            .child(
                div()
                    .w(px(16.0))
                    .text_size(px(16.0))
                    .line_height(px(20.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(alpha(0xffffff, 0.9))
                    .child(if is_active { "✓" } else { "" }),
            )
            .when(slot <= 9, |this| {
                this.child(self.render_mail_account_palette_keycap("control"))
                    .child(self.render_mail_account_palette_keycap(slot.to_string()))
            })
    }

    fn render_mail_account_palette_keycap(&self, label: impl Into<String>) -> Div {
        div()
            .h(px(18.0))
            .min_w(px(20.0))
            .rounded(px(3.0))
            .px(px(5.0))
            .flex()
            .items_center()
            .justify_center()
            .bg(alpha(0x373c43, 0.86))
            .text_size(px(12.0))
            .line_height(px(12.0))
            .font_weight(FontWeight::BOLD)
            .text_color(alpha(0xffffff, 0.9))
            .child(label.into())
    }
}
