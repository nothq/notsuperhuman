use crate::ui::render::*;
use gpui::{point, size, Bounds, Pixels, TestAppContext, VisualTestContext};
use std::{cell::RefCell, rc::Rc};

use super::super::collapsed_top_margin;
use super::super::layout::{
    baseline_image_line, render_table_cell_with_layout, render_table_row,
};
use super::mail_render_options;

mod fixtures;
mod widths;

use fixtures::*;

type CellBounds = Rc<RefCell<Vec<Bounds<Pixels>>>>;

#[test]
fn inline_baseline_image_reserves_html_font_descent() {
    let baseline = vec![MailBlock::Image(MailImage {
        src: "https://example.test/logo.png".to_string(),
        alt: "logo".to_string(),
        width: Some(64.0),
        height: Some(20.0),
        style: MailStyle::default(),
    })];
    let bottom_aligned = vec![MailBlock::Image(MailImage {
        style: MailStyle {
            vertical_align: MailVerticalAlign::Bottom,
            ..Default::default()
        },
        ..match &baseline[0] {
            MailBlock::Image(image) => image.clone(),
            _ => unreachable!(),
        }
    })];

    // At 16px with normal line height the strut reaches Arial's descent plus
    // half its line gap below the baseline: Chromium draws the cell 4px
    // taller than the image.
    let cell = MailStyle {
        font_size: Some(16.0),
        ..Default::default()
    };
    let options = mail_render_options();
    assert_eq!(
        baseline_image_line(&baseline, &cell, &options, 200.0),
        (0.0, 4.0)
    );
    assert_eq!(
        baseline_image_line(&bottom_aligned, &cell, &options, 200.0),
        (0.0, 0.0)
    );
}

#[test]
fn adjacent_block_margins_collapse_to_the_larger_authored_margin() {
    let blocks = vec![
        paragraph_with_margin("first", 6.4, 19.0),
        paragraph_with_margin("second", 6.4, 19.0),
        MailBlock::Table(MailTable {
            rows: Vec::new(),
            style: MailStyle {
                margin: MailEdgeInsets {
                    top: 30.0,
                    right: 0.0,
                    bottom: 30.0,
                    left: 0.0,
                },
                ..Default::default()
            },
            cell_spacing: 0.0,
            row_spacing: 0.0,
            cell_padding: 0.0,
        }),
    ];

    assert!((collapsed_top_margin(&blocks, 1) - 6.4).abs() < 0.001);
    assert!((collapsed_top_margin(&blocks, 2) - 19.0).abs() < 0.001);
}

fn paragraph_with_margin(text: &str, top: f32, bottom: f32) -> MailBlock {
    let MailBlock::Paragraph(mut paragraph) = paragraph(text) else {
        unreachable!("paragraph fixture must remain a paragraph");
    };
    paragraph.style.margin = MailEdgeInsets {
        top,
        right: 0.0,
        bottom,
        left: 0.0,
    };
    MailBlock::Paragraph(paragraph)
}

#[gpui::test]
fn spacer_only_table_cell_keeps_fixed_width_content_cell_readable(cx: &mut TestAppContext) {
    let cell_bounds = measure_table_cells(cx, spacer_and_fixed_content_table(), 640.0, 240.0);

    assert_eq!(cell_bounds.len(), 2, "outer row should render two cells");
    assert!(
        cell_bounds[0].size.width.as_f32() <= 1.0,
        "spacer-only cell should keep minimal width"
    );
    assert!(
        cell_bounds[1].size.width.as_f32() >= 600.0,
        "content cell should own the row width so its 480px child is not clipped"
    );
}

#[gpui::test]
fn label_spacer_value_table_row_gives_long_value_remaining_width(cx: &mut TestAppContext) {
    let cell_bounds = measure_table_cells(cx, label_spacer_value_table(), 640.0, 160.0);

    assert_eq!(
        cell_bounds.len(),
        3,
        "label/value row should render three cells"
    );
    assert!(
        cell_bounds[0].size.width.as_f32() < 80.0,
        "label cell should stay intrinsic"
    );
    assert!(
        (cell_bounds[1].size.width.as_f32() - 24.0).abs() <= 1.0,
        "spacer cell should keep fixed width"
    );
    assert!(
        cell_bounds[2].size.width.as_f32() > 500.0,
        "long value cell should own remaining width"
    );
}

#[gpui::test]
fn fixed_width_visual_table_cell_keeps_logo_slot(cx: &mut TestAppContext) {
    let cell_bounds = measure_table_cells(cx, logo_and_text_table(), 320.0, 80.0);

    assert_eq!(cell_bounds.len(), 2, "logo row should render two cells");
    // Chromium lays this row out as 51 / 233: with room to spare the surplus is
    // shared in proportion to each column's max-content width, so the logo
    // column grows past its 32px content rather than staying pinned to it.
    let logo_width = cell_bounds[0].size.width.as_f32();
    assert!(
        (32.0..80.0).contains(&logo_width),
        "logo cell should grow only slightly past its 32px content, got {logo_width}"
    );
    assert!(
        cell_bounds[1].size.width.as_f32() > 250.0,
        "text cell should own remaining width"
    );
}

#[gpui::test]
fn display_table_cell_columns_keep_fixed_width(cx: &mut TestAppContext) {
    let cell_bounds = measure_table_cells(
        cx,
        hubspot_two_column_table(MailDirection::Ltr),
        640.0,
        120.0,
    );

    assert_eq!(
        cell_bounds.len(),
        2,
        "HubSpot row should render two columns"
    );
    assert!(
        (cell_bounds[0].size.width.as_f32() - 300.0).abs() <= 1.0,
        "first table-cell column should keep 300px width"
    );
    assert!(
        (cell_bounds[1].size.width.as_f32() - 300.0).abs() <= 1.0,
        "second table-cell column should keep 300px width"
    );
}

#[gpui::test]
fn rtl_table_direction_reverses_visual_cell_order(cx: &mut TestAppContext) {
    let cell_bounds = measure_table_cells(
        cx,
        hubspot_two_column_table(MailDirection::Rtl),
        640.0,
        120.0,
    );

    assert_eq!(cell_bounds.len(), 2, "RTL row should render two columns");
    assert!(
        cell_bounds[0].origin.x > cell_bounds[1].origin.x,
        "first source cell should be visually right of second source cell in RTL rows"
    );
}

#[gpui::test]
fn table_cell_vertical_align_moves_content(cx: &mut TestAppContext) {
    let top = measure_table_cell_children(
        cx.add_empty_window(),
        vertical_align_cell(MailVerticalAlign::Top),
    );
    let middle = measure_table_cell_children(
        cx.add_empty_window(),
        vertical_align_cell(MailVerticalAlign::Middle),
    );
    let bottom = measure_table_cell_children(
        cx.add_empty_window(),
        vertical_align_cell(MailVerticalAlign::Bottom),
    );

    assert_eq!(top.len(), 1);
    assert_eq!(middle.len(), 1);
    assert_eq!(bottom.len(), 1);
    assert!(
        top[0].origin.y < middle[0].origin.y && middle[0].origin.y < bottom[0].origin.y,
        "top/middle/bottom vertical-align should produce increasing child offsets"
    );
}

#[gpui::test]
fn vertical_align_keeps_cells_stretched_to_row_height(cx: &mut TestAppContext) {
    let cell_bounds = measure_table_cells(cx, unequal_height_vertical_align_table(), 160.0, 120.0);

    assert_eq!(cell_bounds.len(), 2);
    assert!(
        (cell_bounds[0].size.height.as_f32() - cell_bounds[1].size.height.as_f32()).abs() <= 1.0,
        "vertical-align must move cell content without shrinking the cell background"
    );
    assert!(cell_bounds[0].size.height.as_f32() >= 90.0);
}

fn measure_table_cells(
    cx: &mut TestAppContext,
    table: MailTable,
    width: f32,
    height: f32,
) -> Vec<Bounds<Pixels>> {
    draw_table_cells(cx.add_empty_window(), table, width, height)
}

fn draw_table_cells(
    cx: &mut VisualTestContext,
    table: MailTable,
    width: f32,
    height: f32,
) -> Vec<Bounds<Pixels>> {
    let cell_bounds: CellBounds = Rc::new(RefCell::new(Vec::new()));
    let cell_bounds_listener = Rc::clone(&cell_bounds);
    let options = mail_render_options();

    cx.draw(
        point(px(0.0), px(0.0)),
        size(px(width), px(height)),
        |_, _| {
            div()
                .w(px(width))
                .child(
                    render_table_row(&table.rows[0], &table, &options, "test-table-row", width)
                        .on_children_prepainted(move |bounds, _, _| {
                            cell_bounds_listener.replace(bounds.to_vec());
                        }),
                )
                .into_any_element()
        },
    );
    let measured_bounds = cell_bounds.borrow().clone();
    measured_bounds
}

fn measure_table_cell_children(
    cx: &mut VisualTestContext,
    cell: MailTableCell,
) -> Vec<Bounds<Pixels>> {
    let child_bounds: CellBounds = Rc::new(RefCell::new(Vec::new()));
    let child_bounds_listener = Rc::clone(&child_bounds);
    let options = mail_render_options();

    cx.draw(
        point(px(0.0), px(0.0)),
        size(px(120.0), px(120.0)),
        |_, _| {
            render_table_cell_with_layout(&cell, &options, 320.0, "test-table-cell")
                .on_children_prepainted(move |bounds, _, _| {
                    child_bounds_listener.replace(bounds.to_vec());
                })
                .into_any_element()
        },
    );
    let measured_bounds = child_bounds.borrow().clone();
    measured_bounds
}
