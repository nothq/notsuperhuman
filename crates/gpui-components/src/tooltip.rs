use std::rc::Rc;

use gpui::prelude::FluentBuilder;
use gpui::{
    anchored, div, point, px, rgb, AnyElement, AnyView, App, AppContext, BoxShadow, Context,
    FontWeight, IntoElement, ParentElement, Pixels, Point, Render, SharedString, Styled, Window,
    WindowAppearance,
};

type TooltipRenderer = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

const TOOLTIP_MAX_WIDTH: f32 = 180.0;

#[derive(Clone)]
pub struct TooltipRow {
    label: SharedString,
    shortcut: Option<SharedString>,
}

impl TooltipRow {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
        }
    }

    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }
}

pub struct Tooltip {
    render: TooltipRenderer,
    position: Option<Point<Pixels>>,
}

impl Tooltip {
    pub fn text(label: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView {
        Self::rows([TooltipRow::new(label)])
    }

    pub fn rows(
        rows: impl IntoIterator<Item = TooltipRow>,
    ) -> impl Fn(&mut Window, &mut App) -> AnyView {
        let rows = rows.into_iter().collect::<Vec<_>>();
        Self::build(None, move |window, _| {
            render_rows(&rows, window).into_any_element()
        })
    }

    /// Renders canonical tooltip rows at a window coordinate.
    pub fn at(
        position: Point<Pixels>,
        rows: impl IntoIterator<Item = TooltipRow>,
    ) -> impl Fn(&mut Window, &mut App) -> AnyView {
        let rows = rows.into_iter().collect::<Vec<_>>();
        Self::build(Some(position), move |window, _| {
            render_rows(&rows, window).into_any_element()
        })
    }

    pub fn element(
        render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> impl Fn(&mut Window, &mut App) -> AnyView {
        Self::build(None, render)
    }

    fn build(
        position: Option<Point<Pixels>>,
        render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> impl Fn(&mut Window, &mut App) -> AnyView {
        let render: TooltipRenderer = Rc::new(render);
        move |_, cx| {
            let render = render.clone();
            cx.new(|_| Self { render, position }).into()
        }
    }
}

impl Render for Tooltip {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = (self.render)(window, cx);
        if let Some(position) = self.position {
            anchored()
                .position(position)
                .child(content)
                .into_any_element()
        } else {
            content
        }
    }
}

fn render_rows(rows: &[TooltipRow], window: &Window) -> gpui::Div {
    let dark = match window.appearance() {
        WindowAppearance::Light | WindowAppearance::VibrantLight => false,
        WindowAppearance::Dark | WindowAppearance::VibrantDark => true,
    };
    div()
        .max_w(px(TOOLTIP_MAX_WIDTH))
        .overflow_hidden()
        .px(px(7.0))
        .pt(px(6.0))
        .pb(px(3.0))
        .rounded(px(4.0))
        .bg(rgb(if dark { 0x45494e } else { 0xf5f5f5 }))
        .shadow(vec![
            BoxShadow {
                color: crate::alpha(0x000000, 0.1),
                offset: point(px(0.0), px(2.0)),
                blur_radius: px(4.0),
                spread_radius: px(0.0),
                inset: false,
            },
            BoxShadow {
                color: crate::alpha(0x000000, 0.05),
                offset: point(px(0.0), px(2.0)),
                blur_radius: px(12.0),
                spread_radius: px(0.0),
                inset: false,
            },
        ])
        .children(
            rows.iter()
                .enumerate()
                .map(|(index, row)| render_row(row, index, dark)),
        )
}

fn render_row(row: &TooltipRow, index: usize, dark: bool) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(10.0))
        .when(index > 0, |this| this.mt(px(6.0)))
        .child(
            div()
                .min_w(px(0.0))
                .text_size(px(12.0))
                .line_height(px(20.0))
                .font_weight(FontWeight::NORMAL)
                .text_color(rgb(if dark { 0xebeef2 } else { 0x18191b }))
                .child(row.label.clone()),
        )
        .when_some(row.shortcut.clone(), |this, shortcut| {
            this.child(
                div()
                    .min_w(px(19.0))
                    .h(px(20.0))
                    .px(px(5.0))
                    .rounded(px(3.0))
                    .bg(crate::alpha(if dark { 0x686d75 } else { 0x555b64 }, 1.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.0))
                    .line_height(px(12.0))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(if dark {
                        crate::alpha(0xdfe2e7, 1.0)
                    } else {
                        crate::alpha(0xffffff, 0.8)
                    })
                    .child(shortcut),
            )
        })
}
