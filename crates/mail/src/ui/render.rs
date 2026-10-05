use crate::ui::types::*;
use gpui::{div, prelude::*, px, rgb, AnyElement, App, Hsla, RenderImage, SharedString, Window};
use std::sync::Arc;

use box_style::*;

mod box_style;
mod container;
mod image;
mod intrinsic;
mod layout;
mod overlay;
mod text;
pub(crate) use text::font_weight;

use container::render_container;
use image::{render_box_background_image, render_image};
use layout::render_table;
use text::render_paragraph;

pub type MailImageResolver = Arc<dyn Fn(&str) -> Option<Arc<RenderImage>> + 'static>;
/// The font a run of text will actually be drawn with. Measuring with anything
/// else sizes the box for the wrong glyphs — a bold label measured as regular
/// gets a box too small for itself and wraps.
#[derive(Clone, Debug)]
pub struct MailTextFont {
    pub family: SharedString,
    pub size: f32,
    pub weight: MailFontWeight,
    pub italic: bool,
}

/// Measures one unwrapped line of text. Intrinsic sizing — and therefore
/// automatic table layout — is undefined without the platform's font metrics,
/// so the renderer takes this as a required dependency.
pub type MailTextMeasure = Arc<dyn Fn(&str, &MailTextFont) -> f32 + 'static>;
pub const MAIL_MESSAGE_CARD_MAX_WIDTH: f32 = 700.0;
pub const MAIL_RICH_BODY_SIDE_PADDING: f32 = 24.0;
const MAIL_MESSAGE_CARD_BORDER_WIDTH: f32 = 1.0;
pub const MAIL_RICH_BODY_WIDTH: f32 = MAIL_MESSAGE_CARD_MAX_WIDTH
    - 2.0 * MAIL_RICH_BODY_SIDE_PADDING
    - 2.0 * MAIL_MESSAGE_CARD_BORDER_WIDTH;

#[derive(Clone)]
pub struct MailRenderOptions {
    pub text_color: Hsla,
    pub muted_text_color: Hsla,
    pub link_color: Hsla,
    pub border_color: Hsla,
    pub placeholder_bg: Hsla,
    pub font_family: SharedString,
    pub base_font_size: f32,
    pub line_height: f32,
    pub content_width: f32,
    pub image_resolver: Option<MailImageResolver>,
    pub measure_text: MailTextMeasure,
}

pub fn render_mail_document(document: &MailDocument, options: MailRenderOptions) -> AnyElement {
    let inset = document.body_margin;
    let content_width = options.content_width - inset.horizontal();
    // Styled HTML sits on a browser's canvas: the body's colour, or white.
    let canvas = document
        .body_background
        .or(document.is_rich_layout.then_some(0xffffff));
    div()
        .when_some(canvas, |this, color| this.bg(rgb(color)))
        .pt(px(inset.top))
        .pr(px(inset.right))
        .pb(px(inset.bottom))
        .pl(px(inset.left))
        .flex()
        .flex_col()
        .gap(px(if document.is_rich_layout { 0.0 } else { 12.0 }))
        .children(document.blocks.iter().enumerate().map(|(index, block)| {
            let path = root_render_path(index);
            render_block_with_collapsed_top_margin(
                block,
                collapsed_top_margin(&document.blocks, index),
                &options,
                path.as_str(),
                content_width,
            )
        }))
        .when(document.clipped, |this| {
            this.child(
                div()
                    .text_size(px(options.base_font_size))
                    .line_height(px(options.line_height))
                    .text_color(options.muted_text_color)
                    .child("..."),
            )
        })
        .into_any_element()
}

pub fn render_mail_document_block(block: &MailBlock, options: MailRenderOptions) -> AnyElement {
    let content_width = options.content_width;
    render_block(block, &options, "block", content_width)
}

pub fn render_mail_document_clipped_marker(options: MailRenderOptions) -> AnyElement {
    div()
        .text_size(px(options.base_font_size))
        .line_height(px(options.line_height))
        .text_color(options.muted_text_color)
        .child("...")
        .into_any_element()
}

fn root_render_path(index: usize) -> String {
    format!("b{index}")
}

pub(super) fn child_render_path(parent: &str, segment: &str, index: usize) -> String {
    format!("{parent}.{segment}{index}")
}

fn text_element_id(render_path: &str) -> String {
    format!("mail-paragraph-text-{render_path}")
}

fn render_block(
    block: &MailBlock,
    options: &MailRenderOptions,
    path: &str,
    available: f32,
) -> AnyElement {
    match block {
        MailBlock::Paragraph(paragraph) => {
            render_paragraph(paragraph, options, text_element_id(path))
        }
        MailBlock::Container(container) => render_container(container, options, path, available),
        MailBlock::Table(table) => render_table(table, options, path, available).into_any_element(),
        MailBlock::Image(image) => render_image(image, options, available),
        MailBlock::Rule(style) => render_rule(style, options, path),
        MailBlock::Spacer(height) => div().h(px(*height)).into_any_element(),
    }
}

pub(super) fn render_block_with_collapsed_top_margin(
    block: &MailBlock,
    collapsed_top_margin: f32,
    options: &MailRenderOptions,
    path: &str,
    available: f32,
) -> AnyElement {
    if collapsed_top_margin <= 0.0 {
        return render_block(block, options, path, available);
    }
    let mut block = block.clone();
    if let Some(style) = block_style_mut(&mut block) {
        style.margin.top = (style.margin.top - collapsed_top_margin).max(0.0);
    }
    render_block(&block, options, path, available)
}

pub(super) fn collapsed_top_margin(blocks: &[MailBlock], index: usize) -> f32 {
    let Some(current) = blocks.get(index) else {
        return 0.0;
    };
    let Some(previous) = index.checked_sub(1).and_then(|index| blocks.get(index)) else {
        return 0.0;
    };
    if !block_collapses_vertical_margins(previous) || !block_collapses_vertical_margins(current) {
        return 0.0;
    }
    block_style(previous)
        .zip(block_style(current))
        .map(|(previous, current)| previous.margin.bottom.min(current.margin.top))
        .unwrap_or_default()
        .max(0.0)
}

fn block_collapses_vertical_margins(block: &MailBlock) -> bool {
    let Some(style) = block_style(block) else {
        return false;
    };
    style.float == MailFloat::None
        && style.position == MailPosition::Static
        && !matches!(
            style.display,
            MailDisplay::Inline | MailDisplay::InlineBlock | MailDisplay::TableCell
        )
}

/// The style whose vertical margins take part in collapsing. An image does
/// only as a block: inline by default, it sits in a line box instead.
fn block_style(block: &MailBlock) -> Option<&MailStyle> {
    match block {
        MailBlock::Paragraph(paragraph) => Some(&paragraph.style),
        MailBlock::Container(container) => Some(&container.style),
        MailBlock::Table(table) => Some(&table.style),
        MailBlock::Rule(style) => Some(style),
        MailBlock::Image(image) if image.style.display == MailDisplay::Block => Some(&image.style),
        MailBlock::Image(_) | MailBlock::Spacer(_) => None,
    }
}

fn block_style_mut(block: &mut MailBlock) -> Option<&mut MailStyle> {
    match block {
        MailBlock::Paragraph(paragraph) => Some(&mut paragraph.style),
        MailBlock::Container(container) => Some(&mut container.style),
        MailBlock::Table(table) => Some(&mut table.style),
        MailBlock::Rule(style) => Some(style),
        MailBlock::Image(image) if image.style.display == MailDisplay::Block => {
            Some(&mut image.style)
        }
        MailBlock::Image(_) | MailBlock::Spacer(_) => None,
    }
}

fn render_rule(style: &MailStyle, options: &MailRenderOptions, path: &str) -> AnyElement {
    let mut element = div()
        .h(px(style.border_width.unwrap_or(1.0)))
        .w_full()
        .bg(style
            .border_color
            .map(hsla_color)
            .unwrap_or(options.border_color));
    if let Some(background) = render_box_background_image(style, options, path) {
        element = element.child(background);
    }
    apply_box_style(element, style, options).into_any_element()
}

fn hsla_color(hex: u32) -> Hsla {
    Hsla::from(rgb(hex))
}

#[allow(dead_code)]
fn _keep_handler_types(_: &mut Window, _: &mut App) {}

#[cfg(test)]
mod tests;
