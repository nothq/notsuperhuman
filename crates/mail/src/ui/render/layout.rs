use crate::ui::types::*;
use gpui::{div, prelude::*, px, Div};

mod columns;
mod table;

pub(in crate::ui::render) use columns::table_intrinsic_width;
pub(super) use table::render_table;
use table::table_cell_is_layout_spacer;
#[cfg(test)]
pub(super) use table::{
    render_table_cell_with_layout, render_table_row, table_cell_uses_inline_flow,
};

/// A `<br>` inside an inline formatting context, as a forced line break.
///
/// Between block-level siblings a `<br>` is vertical space, and that is what it
/// parses to. Between inline-level ones it ends the line instead, so here it
/// takes the rest of the line and no height of its own — without that it was a
/// zero-width item in the row and the break was simply lost.
pub(super) fn render_inline_break() -> gpui::AnyElement {
    div().w_full().h(px(0.0)).into_any_element()
}

pub(super) fn mail_blocks_use_inline_flow(blocks: &[MailBlock]) -> bool {
    if blocks.len() < 2
        || blocks
            .iter()
            .all(|block| matches!(block, MailBlock::Spacer(_)))
    {
        return false;
    }
    // Either an inline formatting context, or sibling `display: table-cell` boxes,
    // which CSS wraps in an anonymous table row and lays out as columns.
    blocks.iter().all(mail_block_is_inline_level)
        || blocks.iter().all(mail_block_is_anonymous_table_cell)
}

fn mail_block_is_anonymous_table_cell(block: &MailBlock) -> bool {
    match block {
        MailBlock::Container(container) => container.style.display == MailDisplay::TableCell,
        MailBlock::Image(image) => image.style.display == MailDisplay::TableCell,
        MailBlock::Spacer(_) => true,
        _ => false,
    }
}

fn mail_block_is_inline_level(block: &MailBlock) -> bool {
    match block {
        MailBlock::Paragraph(paragraph) => paragraph.style.is_inline_level(),
        MailBlock::Container(container) => {
            container.style.is_inline_level() || container.style.float != MailFloat::None
        }
        // An image is a replaced inline-level element; only `display: block`
        // (or a float) takes it out of the line.
        MailBlock::Image(image) => {
            image.style.display != MailDisplay::Block || image.style.float != MailFloat::None
        }
        // `display: table` is block-level; only a float takes it out of flow.
        MailBlock::Table(table) => table.style.float != MailFloat::None,
        // Spacers carry no box of their own and never force block layout.
        MailBlock::Spacer(_) => true,
        MailBlock::Rule(_) => false,
    }
}

pub(super) fn mail_image_is_inline_icon(image: &MailImage) -> bool {
    let width = image.width.or(image.style.width);
    let height = image.height.or(image.style.height);
    matches!((width, height), (Some(width), Some(height)) if width <= 24.0 && height <= 24.0)
}

/// The room a line keeps above and below an inline image that is its only
/// content: the image sits on the baseline, and the strut — the block's own
/// font — reaches up past a short image and down by its descent.
/// Chromium puts 4px under a 16px Arial line, 7px under `line-height: 24px`
/// at 15px, and 2px under `line-height: 1`; a fixed share of the font size
/// matched none of them.
pub(super) fn baseline_image_line(
    blocks: &[MailBlock],
    style: &MailStyle,
    options: &super::MailRenderOptions,
    available: f32,
) -> (f32, f32) {
    let [MailBlock::Image(image)] = blocks else {
        return (0.0, 0.0);
    };
    if image.style.vertical_align != MailVerticalAlign::Baseline
        || image.style.display == MailDisplay::Block
    {
        return (0.0, 0.0);
    }
    let size = style.font_size.unwrap_or(options.base_font_size);
    let family = super::text::rendered_font_family(style.font_family, options);
    let line_height = style
        .line_height
        .map(|line_height| line_height.resolve(size));
    let (above, below) = super::text::line_extents(&family, size, line_height);
    let resolved = options
        .image_resolver
        .as_ref()
        .and_then(|resolve| resolve(&image.src));
    let (_, height) =
        super::image::image_display_size(image, options, resolved.as_deref(), available);
    ((above - height).max(0.0), below.max(0.0))
}

pub(super) fn mail_block_is_layout_spacer(block: &MailBlock) -> bool {
    match block {
        MailBlock::Spacer(_) => true,
        MailBlock::Paragraph(paragraph) => {
            !style_paints_box(&paragraph.style)
                && paragraph.runs.iter().all(|run| run.text.trim().is_empty())
        }
        MailBlock::Container(container) => {
            !style_paints_box(&container.style)
                && container.children.iter().all(mail_block_is_layout_spacer)
        }
        MailBlock::Table(table) => {
            !style_paints_box(&table.style)
                && table
                    .rows
                    .iter()
                    .flat_map(|row| row.cells.iter())
                    .all(table_cell_is_layout_spacer)
        }
        MailBlock::Image(_) | MailBlock::Rule(_) => false,
    }
}

fn style_paints_box(style: &MailStyle) -> bool {
    style.background_color.is_some()
        || style.background_image_url.is_some()
        || style.border_color.is_some()
        || style.border_width.unwrap_or_default() > 0.0
        || style.border_edges.any()
}

fn apply_vertical_content_align(element: Div, align: MailVerticalAlign) -> Div {
    match align {
        MailVerticalAlign::Top => element.justify_start(),
        MailVerticalAlign::Middle | MailVerticalAlign::Baseline => element.justify_center(),
        MailVerticalAlign::Bottom => element.justify_end(),
    }
}

fn apply_horizontal_vertical_align(element: Div, align: MailVerticalAlign) -> Div {
    match align {
        MailVerticalAlign::Top => element.items_start(),
        MailVerticalAlign::Middle => element.items_center(),
        MailVerticalAlign::Bottom => element.items_end(),
        MailVerticalAlign::Baseline => element,
    }
}
