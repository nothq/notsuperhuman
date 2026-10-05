//! Automatic table layout.
//!
//! CSS resolves column widths in two passes: measure a min-content and a
//! max-content width for every column, then distribute the table's available
//! width between those bounds. A column is never given less than its
//! min-content width — an over-constrained table overflows instead. Flexbox
//! cannot express that distribution, so it is computed here and the cells are
//! emitted at definite widths.

use super::super::intrinsic::{
    children_intrinsic_width, content_box_width, definite_width, horizontal_edges,
    intrinsic_definite_width, max_width_constraint, MailIntrinsicWidth,
};
use super::super::MailRenderOptions;
use crate::ui::types::*;

/// The width available to a table's columns once its own edges and spacing are
/// removed.
fn columns_available_width(table: &MailTable, available: f32, columns: usize) -> f32 {
    let spacing = table.cell_spacing.max(0.0) * columns.saturating_sub(1) as f32;
    (available - horizontal_edges(&table.style) - spacing).max(0.0)
}

pub(super) fn table_column_count(table: &MailTable) -> usize {
    table
        .rows
        .iter()
        .map(|row| row.cells.iter().map(|cell| cell.colspan.max(1)).sum())
        .max()
        .unwrap_or(0)
}

/// A column's bounds, and whether a cell declared its width.
///
/// CSS distributes a table's surplus width to the columns whose width is `auto`
/// and leaves a declared width alone: Chromium lays a `width: 100%` table with
/// a 120px column and an auto column out as 120 / 520, not proportionally.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct MailColumn {
    pub(super) bounds: MailIntrinsicWidth,
    pub(super) fixed: bool,
}

/// Min and max content width of every column.
pub(super) fn table_column_bounds(
    table: &MailTable,
    options: &MailRenderOptions,
    available: f32,
) -> Vec<MailColumn> {
    let count = table_column_count(table);
    let mut bounds = vec![MailColumn::default(); count];
    let inner = columns_available_width(table, available, count);
    // Single-column cells establish each column's bounds.
    for row in &table.rows {
        let mut index = 0usize;
        for cell in &row.cells {
            let span = cell.colspan.max(1);
            if span == 1 && index < count {
                let cell_bounds = cell_intrinsic_width(cell, options, inner);
                bounds[index].bounds.min = bounds[index].bounds.min.max(cell_bounds.min);
                bounds[index].bounds.max = bounds[index].bounds.max.max(cell_bounds.max);
                bounds[index].fixed |= intrinsic_definite_width(&cell.style).is_some();
            }
            index += span;
        }
    }
    // A spanning cell only widens the columns it covers if they cannot already
    // hold it, and then in proportion to what each contributes.
    for row in &table.rows {
        let mut index = 0usize;
        for cell in &row.cells {
            let span = cell.colspan.max(1);
            if span > 1 && index + span <= count {
                let cell_bounds = cell_intrinsic_width(cell, options, inner);
                let spanned = &mut bounds[index..index + span];
                distribute_span(spanned, cell_bounds, table.cell_spacing.max(0.0));
            }
            index += span;
        }
    }
    bounds
}

fn distribute_span(columns: &mut [MailColumn], cell: MailIntrinsicWidth, spacing: f32) {
    let gaps = spacing * columns.len().saturating_sub(1) as f32;
    for (target, current) in [(cell.min - gaps, true), (cell.max - gaps, false)] {
        let total: f32 = columns
            .iter()
            .map(|column| {
                if current {
                    column.bounds.min
                } else {
                    column.bounds.max
                }
            })
            .sum();
        if total >= target || target <= 0.0 {
            continue;
        }
        let extra = target - total;
        let count = columns.len() as f32;
        for column in columns.iter_mut() {
            let share = if total > 0.0 {
                let own = if current {
                    column.bounds.min
                } else {
                    column.bounds.max
                };
                extra * own / total
            } else {
                extra / count
            };
            if current {
                column.bounds.min += share;
            } else {
                column.bounds.max += share;
            }
        }
    }
}

fn cell_intrinsic_width(
    cell: &MailTableCell,
    options: &MailRenderOptions,
    available: f32,
) -> MailIntrinsicWidth {
    let edges = horizontal_edges(&cell.style);
    let mut bounds =
        children_intrinsic_width(&cell.children, options, (available - edges).max(0.0));
    if let Some(width) = intrinsic_definite_width(&cell.style) {
        // A declared width *is* the column's preferred width, not a floor under
        // the content's: Chromium lays a 588px cell out at 588 and wraps a
        // 60px headline inside it rather than widening the column to the
        // headline's unwrapped width. The column still cannot go below its
        // min-content, and the table still clamps it to what the container
        // offers, which is why a 700px cell in a 650px box comes out at 650.
        bounds.max = width.max(bounds.min);
    }
    bounds.clamp_to_bounds(&cell.style, available).expand(edges)
}

pub(in crate::ui::render) fn table_intrinsic_width(
    table: &MailTable,
    options: &MailRenderOptions,
    available: f32,
) -> MailIntrinsicWidth {
    let count = table_column_count(table);
    let bounds = table_column_bounds(table, options, available);
    let spacing = table.cell_spacing.max(0.0) * count.saturating_sub(1) as f32;
    let edges = horizontal_edges(&table.style) + table.style.margin.horizontal() + spacing;
    let mut result = MailIntrinsicWidth {
        min: bounds.iter().map(|column| column.bounds.min).sum::<f32>() + edges,
        max: bounds.iter().map(|column| column.bounds.max).sum::<f32>() + edges,
    };
    // Only a width in pixels pins the table's contribution. A percentage is
    // `auto` while its container is being sized (CSS Sizing 3 §5.2.1): two
    // `width: 100%` card tables contribute their 256px photos, and share the
    // row equally, rather than each claiming the whole row and then dividing
    // it by the length of the restaurant names.
    if let Some(width) = intrinsic_definite_width(&table.style) {
        result.max = result.max.max(width);
        result.min = result
            .min
            .min(width)
            .max(bounds.iter().map(|c| c.bounds.min).sum());
    }
    if let Some(max_width) = max_width_constraint(&table.style, available) {
        result.max = result.max.min(max_width.max(result.min));
    }
    result
}

/// The used width of a table box and the widths of its columns.
///
/// CSS 2.1 §17.5.2 resolves the two together: the table is sized from its
/// columns' intrinsic widths, and that width is then shared back out across
/// them. Splitting the two apart sized the columns against the containing block
/// instead of against the table, so a shrink-to-fit table stretched its columns
/// to a width the table itself never had.
fn solve_table(table: &MailTable, options: &MailRenderOptions, available: f32) -> (f32, Vec<f32>) {
    let count = table_column_count(table);
    let columns = table_column_bounds(table, options, available);
    let spacing = table.cell_spacing.max(0.0) * count.saturating_sub(1) as f32;
    let edges = horizontal_edges(&table.style);
    let outer = table.style.margin.horizontal();
    let total_min: f32 = columns.iter().map(|column| column.bounds.min).sum();
    let total_max: f32 = columns.iter().map(|column| column.bounds.max).sum();
    // A table is shrink-to-fit: an auto width takes the max-content width but no
    // more than the containing block offers, while a declared width is honoured
    // even when it overflows — Chromium draws a `width: 600px` table inside a
    // 300px parent at 600px. Either way the columns' minimums are a hard floor,
    // because an over-constrained table overflows rather than crushing a cell.
    let mut content = match definite_width(&table.style, available) {
        Some(width) => content_box_width(&table.style, width),
        None => (total_max + spacing).min((available - outer - edges).max(0.0)),
    };
    if let Some(max_width) = max_width_constraint(&table.style, available) {
        content = content.min(content_box_width(&table.style, max_width));
    }
    // Fixed layout does not consult content at all, so the columns' minimums are
    // not a floor on the table either.
    if table.style.table_layout == MailTableLayout::Fixed {
        return (
            content + edges,
            fixed_table_columns(table, (content - spacing).max(0.0)),
        );
    }
    let content = content.max(total_min + spacing);
    (
        content + edges,
        distribute_columns(&columns, (content - spacing).max(0.0)),
    )
}

/// Column widths under fixed table layout, per CSS 2.1 §17.5.2.1.
///
/// The first row decides the columns and the table's own width decides the rest:
/// a cell that declares a width takes it, and the columns that do not share what
/// is left equally. Cell content is never measured, which is the point — email
/// reaches for `table-layout: fixed` precisely to stop a long word or an image
/// from reflowing a layout.
fn fixed_table_columns(table: &MailTable, target: f32) -> Vec<f32> {
    let count = table_column_count(table);
    let mut declared = vec![None; count];
    if let Some(row) = table.rows.first() {
        let mut index = 0usize;
        for cell in &row.cells {
            let span = cell.colspan.max(1);
            if let Some(width) = intrinsic_definite_width(&cell.style) {
                let share = (width + horizontal_edges(&cell.style)) / span as f32;
                for column in declared.iter_mut().skip(index).take(span) {
                    *column = Some(share);
                }
            }
            index += span;
        }
    }
    let specified: f32 = declared.iter().flatten().sum();
    let flexible = declared.iter().filter(|column| column.is_none()).count();
    let share = if flexible > 0 {
        ((target - specified) / flexible as f32).max(0.0)
    } else {
        0.0
    };
    declared
        .iter()
        .map(|column| column.unwrap_or(share))
        .collect()
}

pub(super) fn table_used_width(
    table: &MailTable,
    options: &MailRenderOptions,
    available: f32,
) -> f32 {
    solve_table(table, options, available).0
}

pub(super) fn solve_table_columns(
    table: &MailTable,
    options: &MailRenderOptions,
    available: f32,
) -> Vec<f32> {
    solve_table(table, options, available).1
}

/// Share a table's resolved content width across its columns.
///
/// Every column starts at its min-content width. A column whose width a cell
/// declared is satisfied next and then left alone: what remains goes to the
/// `auto` columns, in proportion to how much each can still grow, and so does
/// any surplus beyond every column's max-content width. Chromium lays a
/// `width: 100%` table with a 120px column beside an auto column out as
/// 120 / 520, not in proportion to the two. Only a table whose every column is
/// declared shares its surplus out proportionally.
fn distribute_columns(columns: &[MailColumn], target: f32) -> Vec<f32> {
    let mut widths: Vec<f32> = columns.iter().map(|column| column.bounds.min).collect();
    let mut remaining = target - widths.iter().sum::<f32>();
    if remaining <= 0.0 {
        return widths;
    }
    grow_columns(&mut widths, columns, &mut remaining, true);
    grow_columns(&mut widths, columns, &mut remaining, false);
    if remaining <= 0.0 {
        return widths;
    }
    let flexible = columns.iter().any(|column| !column.fixed);
    let total: f32 = columns
        .iter()
        .filter(|column| !flexible || !column.fixed)
        .map(|column| column.bounds.max)
        .sum();
    for (width, column) in widths.iter_mut().zip(columns) {
        if flexible && column.fixed {
            continue;
        }
        *width += if total > 0.0 {
            remaining * column.bounds.max / total
        } else {
            remaining / columns.len() as f32
        };
    }
    widths
}

/// Grow either the declared or the auto columns from their minimum toward their
/// max-content width, sharing `remaining` in proportion to the room each has.
fn grow_columns(widths: &mut [f32], columns: &[MailColumn], remaining: &mut f32, fixed: bool) {
    let room: f32 = columns
        .iter()
        .filter(|column| column.fixed == fixed)
        .map(|column| column.bounds.max - column.bounds.min)
        .sum();
    if room <= 0.0 {
        return;
    }
    let used = room.min(*remaining);
    for (width, column) in widths.iter_mut().zip(columns) {
        if column.fixed == fixed {
            *width += used * (column.bounds.max - column.bounds.min) / room;
        }
    }
    *remaining -= used;
}
