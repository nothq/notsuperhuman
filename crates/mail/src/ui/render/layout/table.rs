use super::{
    apply_horizontal_vertical_align, apply_vertical_content_align, baseline_image_line,
    mail_block_is_layout_spacer, mail_blocks_use_inline_flow, style_paints_box,
};
use crate::ui::{
    render::{
        apply_box_style, child_render_path, collapsed_top_margin,
        image::render_box_background_image, render_block_with_collapsed_top_margin,
        MailRenderOptions,
    },
    types::*,
};
use gpui::{div, prelude::*, px, relative, AnyElement, Div};

pub(in crate::ui::render) fn render_table(
    table: &MailTable,
    options: &MailRenderOptions,
    path: &str,
    available: f32,
) -> Div {
    let aligns_itself = table_aligns_itself(table);
    let table_box = render_table_box(table, options, path, available);
    render_table_wrapper(table, aligns_itself).child(table_box)
}

fn render_table_box(
    table: &MailTable,
    options: &MailRenderOptions,
    path: &str,
    available: f32,
) -> Div {
    let element = render_table_contents(table, options, path, available);
    let mut style = table.style.clone();
    style.margin_left_auto = false;
    style.margin_right_auto = false;
    // Column widths are already resolved, so the box takes exactly the width the
    // table layout chose rather than being stretched or shrunk by flexbox.
    let used = super::columns::table_used_width(table, options, available);
    apply_box_style(element, &style, options)
        .w(px(used))
        .flex_none()
}

fn render_table_contents(
    table: &MailTable,
    options: &MailRenderOptions,
    path: &str,
    available: f32,
) -> Div {
    let row_gap = table.row_spacing.max(0.0);
    let cell_gap = table.cell_spacing.max(0.0);
    let mut element = div().min_w(px(0.0)).flex().flex_col().gap(px(row_gap));
    if let Some(background) = render_box_background_image(&table.style, options, path) {
        element = element.child(background);
    }
    element
        .children(table.rows.iter().enumerate().map(|(row_index, row)| {
            let row_path = child_render_path(path, "r", row_index);
            render_table_row(row, table, options, row_path.as_str(), available)
        }))
        .when(row_gap > 0.0, |this| this.pt(px(row_gap)).pb(px(row_gap)))
        .when(cell_gap > 0.0, |this| {
            this.pl(px(cell_gap)).pr(px(cell_gap))
        })
}

fn render_table_wrapper(table: &MailTable, aligns_itself: bool) -> Div {
    let wrapper = div()
        .min_w(px(0.0))
        .max_w_full()
        .flex()
        .flex_shrink_1()
        .when(
            table.style.margin_left_auto && table.style.margin_right_auto,
            |this| this.w_full().justify_center(),
        )
        .when(table.style.float == MailFloat::Right, |this| {
            this.w_full().justify_end()
        });
    let wrapper = apply_table_wrapper_width(wrapper, table, aligns_itself);
    apply_table_wrapper_constraints(wrapper, table, aligns_itself)
}

fn apply_table_wrapper_width(wrapper: Div, table: &MailTable, aligns_itself: bool) -> Div {
    if aligns_itself {
        wrapper
    } else if let Some(width) = table.style.width {
        wrapper.w_full().max_w(px(width))
    } else if let Some(width_percent) = table.style.width_percent {
        if width_percent >= 0.999 {
            wrapper.w_full()
        } else {
            wrapper.w(relative(width_percent))
        }
    } else {
        wrapper
    }
}

fn apply_table_wrapper_constraints(wrapper: Div, table: &MailTable, aligns_itself: bool) -> Div {
    if aligns_itself {
        return wrapper;
    }
    wrapper
        .when_some(table.style.min_width, |this, width| this.min_w(px(width)))
        .when_some(table.style.max_width, |this, width| this.max_w(px(width)))
        .when_some(table.style.max_width_percent, |this, width| {
            this.max_w(relative(width))
        })
}

fn table_aligns_itself(table: &MailTable) -> bool {
    (table.style.margin_left_auto && table.style.margin_right_auto)
        || table.style.float == MailFloat::Right
}

pub(in crate::ui::render) fn render_table_row(
    row: &MailTableRow,
    table: &MailTable,
    options: &MailRenderOptions,
    path: &str,
    available: f32,
) -> Div {
    let column_widths = super::columns::solve_table_columns(table, options, available);
    div()
        .flex()
        .items_stretch()
        .gap(px(table.cell_spacing.max(0.0)))
        .when(table.style.direction == MailDirection::Rtl, |this| {
            this.flex_row_reverse()
        })
        .children(row.cells.iter().enumerate().map(|(index, cell)| {
            let cell_path = child_render_path(path, "c", index);
            render_table_cell_with_layout(
                cell,
                options,
                cell_span_width(&column_widths, row, index),
                cell_path.as_str(),
            )
        }))
}

pub(in crate::ui::render) fn render_table_cell_with_layout(
    cell: &MailTableCell,
    options: &MailRenderOptions,
    width: f32,
    path: &str,
) -> Div {
    let is_layout_spacer = table_cell_is_layout_spacer(cell);
    let uses_inline_flow = table_cell_uses_inline_flow(cell);
    let uses_horizontal_flow = uses_inline_flow;
    let inner = (width - super::super::intrinsic::horizontal_edges(&cell.style)).max(0.0);
    let base = render_table_cell_contents(
        TableCellContents {
            cell,
            options,
            path,
            available: inner,
        },
        uses_inline_flow,
        uses_horizontal_flow,
    );
    let mut style = cell.style.clone();
    let (above, below) = baseline_image_line(&cell.children, &cell.style, options, inner);
    style.padding.top += above;
    style.padding.bottom += below;
    let mut element = apply_box_style(base, &style, options);
    if !is_layout_spacer && !uses_horizontal_flow {
        element = apply_vertical_content_align(element, cell.style.vertical_align);
    }
    element.w(px(width)).flex_none()
}

/// A cell's children as they render: the cell, its render options and path,
/// and the width inside its padding.
#[derive(Clone, Copy)]
struct TableCellContents<'a> {
    cell: &'a MailTableCell,
    options: &'a MailRenderOptions,
    path: &'a str,
    available: f32,
}

fn render_table_cell_contents(
    contents: TableCellContents<'_>,
    uses_inline_flow: bool,
    uses_horizontal_flow: bool,
) -> Div {
    let TableCellContents {
        cell,
        options,
        path,
        ..
    } = contents;
    let centers_child_tables = !uses_horizontal_flow
        && cell.style.text_align.resolve(cell.style.direction) == MailTextAlign::Center;
    let mut base = div().min_w(px(0.0)).flex().gap(px(0.0));
    if let Some(background) = render_box_background_image(&cell.style, options, path) {
        base = base.child(background);
    }
    base.children(cell.children.iter().enumerate().map(|(index, block)| {
        if uses_inline_flow && matches!(block, MailBlock::Spacer(_)) {
            return super::render_inline_break();
        }
        render_table_cell_child(contents, block, index, centers_child_tables)
    }))
    .when(uses_inline_flow, |this| this.flex_wrap().items_center())
    .when(
        uses_horizontal_flow && cell.style.direction == MailDirection::Rtl,
        |this| this.flex_row_reverse(),
    )
    .when(uses_horizontal_flow, |this| {
        apply_horizontal_vertical_align(this, cell.style.vertical_align)
    })
    .when(!uses_horizontal_flow, |this| this.flex_col())
}

fn render_table_cell_child(
    contents: TableCellContents<'_>,
    block: &MailBlock,
    index: usize,
    centers_child_tables: bool,
) -> AnyElement {
    let TableCellContents {
        cell,
        options,
        path,
        available,
    } = contents;
    let child_path = child_render_path(path, "b", index);
    let child = render_block_with_collapsed_top_margin(
        block,
        collapsed_top_margin(&cell.children, index),
        options,
        child_path.as_str(),
        available,
    );
    // A floated table is taken out of the flow and aligned to its float side,
    // so the cell's `text-align` does not move it: `<table align="left">` in a
    // centred footer cell stays at the left edge.
    let floats = match block {
        MailBlock::Table(table) => table.style.float != MailFloat::None,
        _ => false,
    };
    if centers_child_tables && !floats && matches!(block, MailBlock::Table(_)) {
        div()
            .w_full()
            .flex()
            .justify_center()
            .child(child)
            .into_any_element()
    } else {
        child
    }
}

/// The width a cell occupies: its column, plus the columns it spans and the
/// spacing between them.
fn cell_span_width(columns: &[f32], row: &MailTableRow, index: usize) -> f32 {
    let start: usize = row
        .cells
        .iter()
        .take(index)
        .map(|cell| cell.colspan.max(1))
        .sum();
    let span = row.cells[index].colspan.max(1);
    columns.iter().skip(start).take(span).sum::<f32>().max(0.0)
}

pub(super) fn table_cell_is_layout_spacer(cell: &MailTableCell) -> bool {
    !style_paints_box(&cell.style) && cell.children.iter().all(mail_block_is_layout_spacer)
}

pub(in crate::ui::render) fn table_cell_uses_inline_flow(cell: &MailTableCell) -> bool {
    mail_blocks_use_inline_flow(&cell.children)
}
