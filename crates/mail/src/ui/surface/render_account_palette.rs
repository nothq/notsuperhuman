use super::{
    alpha, div, img, point, px, rgb, AnyElement, AppearanceMode, BoxShadow, Context, Div,
    FluentBuilder, FontWeight, InteractiveElement, IntoElement, MailAccountPaletteState,
    MouseButton, MouseDownEvent, ParentElement, StatefulInteractiveElement, Styled, SurfaceState,
    MAIL_APPBAR_WIDTH,
};
use crate::model::MailAccountInfo;
use gpui_components::backdrop::{dismissible_backdrop, BackdropDismissal};

mod chip;
mod rows;
mod skeleton;

const MAIL_ACCOUNT_PALETTE_WIDTH: f32 = 680.0;
const MAIL_ACCOUNT_PALETTE_CENTER_OFFSET: f32 = 199.0;
pub(super) const MAIL_ACCOUNT_ROW_HEIGHT: f32 = 62.0;

pub(super) struct MailAccountPaletteRender<'a> {
    pub(super) state: &'a MailAccountPaletteState,
    pub(super) options: &'a [(usize, MailAccountInfo)],
    pub(super) selected_index: usize,
    pub(super) active_account_id: Option<&'a str>,
}

impl SurfaceState {
    pub(crate) fn render_mail_account_palette(
        &self,
        state: &MailAccountPaletteState,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let options = self.mail_account_palette_options();
        let selected_index = state.selected_index.min(options.len().saturating_sub(1));
        let active_account_id = self
            .mail_workspace()
            .map(|workspace| workspace.account_id.as_str());
        let render = MailAccountPaletteRender {
            state,
            options: &options,
            selected_index,
            active_account_id,
        };
        let chrome_top = self.mail_content_top_inset();
        let available_height = (self.viewport_height - chrome_top).max(0.0);
        let command_top = (available_height * 0.5 - MAIL_ACCOUNT_PALETTE_CENTER_OFFSET).max(0.0);
        div()
            .id("mail-account-palette")
            .debug_selector(|| "mail-account-palette".to_string())
            .absolute()
            .top(px(chrome_top))
            .right(px(0.0))
            .bottom(px(0.0))
            .left(px(0.0))
            .occlude()
            .flex()
            .items_start()
            .justify_center()
            .pt(px(command_top))
            .child(dismissible_backdrop(
                div()
                    .absolute()
                    .top(px(0.0))
                    .right(px(0.0))
                    .bottom(px(0.0))
                    .left(px(0.0)),
                BackdropDismissal::new(|this: &mut SurfaceState, _, _, cx| {
                    this.close_mail_account_palette(cx);
                }),
                cx,
            ))
            .child(self.render_mail_account_palette_panel(
                &render,
                available_height - command_top,
                cx,
            ))
            .into_any_element()
    }

    fn render_mail_account_palette_panel(
        &self,
        render: &MailAccountPaletteRender<'_>,
        max_height: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let sections = self.render_mail_account_palette_sections(render, cx);
        let panel_background = match self.appearance_mode {
            AppearanceMode::Light => 0x18191a,
            AppearanceMode::Dark => 0x686d77,
        };
        div()
            .id("mail-account-palette-panel")
            .debug_selector(|| "mail-account-palette-panel".to_string())
            .relative()
            .left(px(-MAIL_APPBAR_WIDTH * 0.5))
            .w(px(MAIL_ACCOUNT_PALETTE_WIDTH))
            .max_h(px(max_height.max(0.0)))
            .rounded(px(4.0))
            .overflow_hidden()
            .flex()
            .flex_col()
            .pt(px(16.0))
            .bg(rgb(panel_background))
            .shadow(mail_account_palette_shadows())
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                }),
            )
            .child(self.render_mail_account_palette_title(cx))
            .when_some(render.state.error.as_ref(), |this, error| {
                this.child(self.render_mail_account_palette_error(error))
            })
            .when(render.options.is_empty(), |this| {
                this.child(self.render_mail_account_palette_empty())
            })
            .when(!render.options.is_empty(), |this| {
                this.child(
                    div()
                        .id("mail-account-palette-accounts")
                        .flex_grow(1.0)
                        .min_h(px(0.0))
                        .pb(px(2.0))
                        .overflow_y_scroll()
                        .track_scroll(&self.mail_account_palette_scroll)
                        .children(sections),
                )
            })
            .child(self.render_mail_account_palette_footer(cx))
            .into_any_element()
    }

    fn render_mail_account_palette_title(&self, cx: &mut Context<Self>) -> Div {
        div()
            .mx(px(36.0))
            .mb(px(8.0))
            .h(px(36.0))
            .pb(px(16.0))
            .border_b_1()
            .border_color(alpha(0xffffff, 0.1))
            .flex()
            .items_start()
            .gap(px(13.0))
            .child(
                img(self.render_mail_svg_icon(
                    "0 0 20 20",
                    r##"<circle cx="10" cy="10" r="8.25" fill="none" stroke="#FFFFFF" stroke-width="1.25"/><circle cx="10" cy="7.5" r="2.35" fill="none" stroke="#FFFFFF" stroke-width="1.25"/><path d="M5.8 15.1c.7-2.2 2.05-3.3 4.2-3.3s3.5 1.1 4.2 3.3" fill="none" stroke="#FFFFFF" stroke-width="1.25" stroke-linecap="round"/>"##,
                    cx,
                ))
                .mt(px(1.0))
                .size(px(20.0))
                .opacity(0.7),
            )
            .child(
                div()
                    .mt(px(3.0))
                    .text_size(px(14.0))
                    .line_height(px(20.0))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(alpha(0xffffff, 0.7))
                    .child("Accounts"),
            )
    }

    fn render_mail_account_palette_error(&self, error: &str) -> Div {
        div()
            .mx(px(36.0))
            .mb(px(8.0))
            .px(px(12.0))
            .py(px(9.0))
            .bg(alpha(0xb64242, 0.28))
            .text_size(px(12.0))
            .text_color(alpha(0xffffff, 0.9))
            .child(error.to_string())
    }

    fn render_mail_account_palette_empty(&self) -> Div {
        div()
            .h(px(MAIL_ACCOUNT_ROW_HEIGHT))
            .px(px(36.0))
            .flex()
            .items_center()
            .text_size(px(16.0))
            .line_height(px(20.0))
            .text_color(alpha(0xffffff, 0.5))
            .child("No matching accounts")
    }

    fn render_mail_account_palette_footer(&self, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .flex_col()
            .pb(px(6.0))
            .child(div().mx(px(36.0)).h(px(1.0)).bg(alpha(0xffffff, 0.1)))
            .child(render_mail_add_account_row(
                "mail-account-add-superhuman",
                "Connect Superhuman accounts",
                cx.listener(|this, _, _, cx| {
                    this.mail_account_palette = None;
                    this.start_mail_superhuman_sign_in(cx);
                }),
            ))
            .child(render_mail_add_account_row(
                "mail-account-add-jmap",
                "Add a JMAP account (Fastmail and others)",
                cx.listener(|this, _, _, cx| {
                    this.open_mail_sign_in(true, cx);
                }),
            ))
    }
}

fn render_mail_add_account_row(
    id: &'static str,
    label: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> AnyElement {
    div()
        .id(id)
        .h(px(48.0))
        .px(px(36.0))
        .flex()
        .items_center()
        .gap(px(9.0))
        .cursor_pointer()
        .text_color(alpha(0xffffff, 0.5))
        .hover(|style| {
            style
                .bg(alpha(0xffffff, 0.06))
                .text_color(alpha(0xffffff, 0.8))
        })
        .on_click(on_click)
        .child(
            div()
                .size(px(24.0))
                .rounded_full()
                .border_1()
                .border_color(alpha(0xffffff, 0.35))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(18.0))
                .line_height(px(18.0))
                .child("+"),
        )
        .child(div().text_size(px(16.0)).line_height(px(20.0)).child(label))
        .into_any_element()
}

fn mail_account_palette_shadows() -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: alpha(0x000000, 0.7),
            offset: point(px(0.0), px(15.0)),
            blur_radius: px(50.0),
            spread_radius: px(0.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x000000, 1.0),
            offset: point(px(0.0), px(19.0)),
            blur_radius: px(90.0),
            spread_radius: px(0.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0xffffff, 0.1),
            offset: point(px(0.15), px(0.5)),
            blur_radius: px(0.0),
            spread_radius: px(0.0),
            inset: true,
        },
    ]
}

pub(super) fn mail_account_display_name(account: &MailAccountInfo) -> String {
    if !account.name.trim().is_empty() {
        return account.name.clone();
    }
    account
        .address
        .as_deref()
        .and_then(|address| address.split('@').next())
        .unwrap_or("Mail")
        .to_string()
}

pub(super) fn mail_account_initials(account: &MailAccountInfo) -> String {
    let label = mail_account_display_name(account);
    let mut characters = label
        .chars()
        .filter(|character| character.is_alphanumeric());
    let Some(first) = characters.next() else {
        return "M".to_string();
    };
    let trailing_digit = label
        .chars()
        .rev()
        .find(|character| character.is_ascii_digit());
    let second = trailing_digit.or_else(|| characters.next());
    [Some(first), second]
        .into_iter()
        .flatten()
        .flat_map(char::to_uppercase)
        .collect()
}
