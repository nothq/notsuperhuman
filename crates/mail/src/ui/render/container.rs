use super::{
    apply_box_style, child_render_path, collapsed_top_margin,
    image::render_box_background_image,
    layout::{baseline_image_line, mail_blocks_use_inline_flow, render_inline_break},
    render_block_with_collapsed_top_margin, MailRenderOptions,
};
use crate::ui::types::*;
use gpui::{div, prelude::*, px, AnyElement, Div};

struct ContainerFlow {
    inline: bool,
    baseline_line: (f32, f32),
    inline_box: bool,
}

impl ContainerFlow {
    fn for_container(container: &MailContainer, options: &MailRenderOptions, inner: f32) -> Self {
        let inline = mail_blocks_use_inline_flow(&container.children);
        let inline_box = matches!(
            container.style.display,
            MailDisplay::Inline | MailDisplay::InlineBlock | MailDisplay::TableCell
        ) || container.style.float != MailFloat::None;
        Self {
            inline,
            baseline_line: baseline_image_line(
                &container.children,
                &container.style,
                options,
                inner,
            ),
            inline_box,
        }
    }
}

pub(super) fn render_container(
    container: &MailContainer,
    options: &MailRenderOptions,
    path: &str,
    available: f32,
) -> AnyElement {
    let used = super::intrinsic::block_used_width(&container.style, available);
    let inner = used - super::intrinsic::horizontal_edges(&container.style);
    let flow = ContainerFlow::for_container(container, options, inner.max(0.0));
    let base = render_container_contents(container, options, path, &flow, inner.max(0.0));
    let base = apply_container_style(base, container, options, &flow);
    // A block box takes the width CSS resolved for it. Asking flexbox for
    // `width: 100%` instead measured the whole containing block and then pushed
    // the margins outside it, so a box with horizontal margins never narrowed.
    let base = base.when(!flow.inline_box, |this| this.w(px(used)).flex_shrink_0());
    let element = render_container_link(base, container.link_target.as_deref(), path);
    if container.style.display == MailDisplay::InlineBlock {
        return super::overlay::overlay(element).into_any_element();
    }
    element
}

fn render_container_contents(
    container: &MailContainer,
    options: &MailRenderOptions,
    path: &str,
    flow: &ContainerFlow,
    available: f32,
) -> Div {
    let mut base = div().flex().gap(px(0.0));
    if let Some(background) = render_box_background_image(&container.style, options, path) {
        base = base.child(background);
    }
    base.children(container.children.iter().enumerate().map(|(index, block)| {
        if flow.inline && matches!(block, MailBlock::Spacer(_)) {
            return render_inline_break();
        }
        let child_path = child_render_path(path, "c", index);
        render_block_with_collapsed_top_margin(
            block,
            collapsed_top_margin(&container.children, index),
            options,
            child_path.as_str(),
            available,
        )
    }))
    // An inline formatting context breaks a line when its boxes no longer fit,
    // which is what lets two `width: 100%` inline-blocks stack instead of
    // sitting side by side in a row that overflows its container.
    .when(flow.inline, |this| this.flex_wrap().items_center())
    .when(
        flow.inline && container.style.direction == MailDirection::Rtl,
        |this| this.flex_row_reverse(),
    )
    .when(!flow.inline, |this| this.flex_col())
}

fn apply_container_style(
    base: Div,
    container: &MailContainer,
    options: &MailRenderOptions,
    flow: &ContainerFlow,
) -> Div {
    let align = container
        .style
        .line_align
        .resolve(container.style.direction);
    let mut box_style = container.style.clone();
    box_style.padding.top += flow.baseline_line.0;
    box_style.padding.bottom += flow.baseline_line.1;
    // CSS 2.1 §10.6.1: an inline box's vertical padding and border paint but
    // take no room in the line — a `<span style="padding: 7px 0">` sits on a
    // 22px line, not a 36px one. The edges are drawn and then given back.
    let inline_edges = container.style.display == MailDisplay::Inline;
    let (top_edge, bottom_edge) = (
        container.style.padding.top + super::box_border_top_width(&container.style),
        container.style.padding.bottom + super::box_border_bottom_width(&container.style),
    );
    apply_box_style(base, &box_style, options)
        .when(inline_edges && top_edge > 0.0, |this| {
            this.mt(px(-top_edge))
        })
        .when(inline_edges && bottom_edge > 0.0, |this| {
            this.mb(px(-bottom_edge))
        })
        .when(flow.inline_box, |this| {
            this.w_auto()
                .flex_none()
                .when(align == MailTextAlign::Left, |this| this.self_start())
                .when(align == MailTextAlign::Center, |this| this.self_center())
                .when(
                    align == MailTextAlign::Right || container.style.float == MailFloat::Right,
                    |this| this.self_end(),
                )
        })
}

fn render_container_link(base: Div, link_target: Option<&str>, path: &str) -> AnyElement {
    let Some(link_target) = link_target else {
        return base.into_any_element();
    };
    let link_target = link_target.to_owned();
    base.relative()
        .child(
            div()
                .id(format!("mail-container-link-{path}"))
                .absolute()
                .inset_0()
                .cursor_pointer()
                .on_click(move |_, _, cx| cx.open_url(link_target.as_str())),
        )
        .into_any_element()
}
