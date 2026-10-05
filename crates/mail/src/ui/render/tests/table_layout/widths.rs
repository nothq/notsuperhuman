use super::{
    fixtures::{paragraph, table_cell},
    mail_render_options, CellBounds,
};
use crate::ui::{
    render::layout::render_table,
    types::{MailStyle, MailTable, MailTableRow},
};
use gpui::{div, point, prelude::*, px, size, TestAppContext};
use std::{cell::RefCell, rc::Rc};

#[gpui::test]
fn auto_width_email_table_does_not_exceed_parent_content_box(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let child_bounds: CellBounds = Rc::new(RefCell::new(Vec::new()));
    let child_bounds_listener = Rc::clone(&child_bounds);
    let table = MailTable {
        style: MailStyle::default(),
        rows: vec![MailTableRow {
            cells: vec![table_cell(
                vec![paragraph(
                    "Please ignore this email if this was not you trying to create an account.",
                )],
                MailStyle::default(),
            )],
        }],
        cell_spacing: 0.0,
        row_spacing: 0.0,
        cell_padding: 0.0,
    };
    let options = mail_render_options();

    cx.draw(
        point(px(0.0), px(0.0)),
        size(px(528.0), px(100.0)),
        |_, _| {
            div()
                .w(px(528.0))
                .on_children_prepainted(move |bounds, _, _| {
                    child_bounds_listener.replace(bounds.to_vec());
                })
                .child(render_table(&table, &options, "auto-width-table", 528.0))
                .into_any_element()
        },
    );

    let measured = child_bounds.borrow();
    assert_eq!(measured.len(), 1);
    assert!(
        measured[0].size.width.as_f32() <= 528.0,
        "auto-width table must wrap within its parent's email content box"
    );
}

/// A percentage `max-width` is what shrinks an authored-wide email table, and
/// an absolute one does not. Chromium lays both of these tables out inside a
/// 300px parent at their authored 600px and lets them overflow; only
/// `max-width: 100%` resolves against the parent and pulls the table in.
#[gpui::test]
fn percentage_max_width_shrinks_an_authored_wide_email_table(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let table_bounds: CellBounds = Rc::new(RefCell::new(Vec::new()));
    let table_bounds_listener = Rc::clone(&table_bounds);
    let table = MailTable {
        style: MailStyle {
            width: Some(600.0),
            max_width_percent: Some(1.0),
            ..Default::default()
        },
        rows: vec![MailTableRow {
            cells: vec![table_cell(
                vec![paragraph(
                    "This real email paragraph must wrap within the containing message column instead of overflowing at the authored desktop width.",
                )],
                MailStyle::default(),
            )],
        }],
        cell_spacing: 0.0,
        row_spacing: 0.0,
        cell_padding: 0.0,
    };
    let options = mail_render_options();

    cx.draw(
        point(px(0.0), px(0.0)),
        size(px(300.0), px(160.0)),
        |_, _| {
            div()
                .w(px(300.0))
                .child(
                    render_table(&table, &options, "fixed-max-width-table", 300.0)
                        .on_children_prepainted(move |bounds, _, _| {
                            table_bounds_listener.replace(bounds.to_vec());
                        }),
                )
                .into_any_element()
        },
    );

    let measured = table_bounds.borrow();
    assert_eq!(measured.len(), 1);
    assert!(
        (measured[0].size.width.as_f32() - 300.0).abs() <= 1.0,
        "max-width: 100% must shrink the authored 600px table to the parent width; got {}",
        measured[0].size.width.as_f32()
    );
}

/// An absolute width a browser honours, notsuperhuman honours too: Chromium draws this
/// table at 600px inside its 300px parent and lets it overflow, and the message
/// card clips it the same way the browser viewport does.
#[gpui::test]
fn absolute_width_email_table_keeps_its_authored_width(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let table_bounds: CellBounds = Rc::new(RefCell::new(Vec::new()));
    let table_bounds_listener = Rc::clone(&table_bounds);
    let table = MailTable {
        style: MailStyle {
            width: Some(600.0),
            max_width: Some(600.0),
            ..Default::default()
        },
        rows: vec![MailTableRow {
            cells: vec![table_cell(
                vec![paragraph(
                    "This real email paragraph must wrap within the containing message column instead of overflowing at the authored desktop width.",
                )],
                MailStyle::default(),
            )],
        }],
        cell_spacing: 0.0,
        row_spacing: 0.0,
        cell_padding: 0.0,
    };
    let options = mail_render_options();

    cx.draw(
        point(px(0.0), px(0.0)),
        size(px(300.0), px(160.0)),
        |_, _| {
            div()
                .w(px(300.0))
                .child(
                    render_table(&table, &options, "absolute-width-table", 300.0)
                        .on_children_prepainted(move |bounds, _, _| {
                            table_bounds_listener.replace(bounds.to_vec());
                        }),
                )
                .into_any_element()
        },
    );

    let measured = table_bounds.borrow();
    assert_eq!(measured.len(), 1);
    assert!(
        (measured[0].size.width.as_f32() - 600.0).abs() <= 1.0,
        "an absolute width is not shrunk by the parent; got {}",
        measured[0].size.width.as_f32()
    );
}

#[gpui::test]
fn explicit_width_email_table_preserves_authored_width_inside_wider_parent(
    cx: &mut TestAppContext,
) {
    let cx = cx.add_empty_window();
    let table_bounds: CellBounds = Rc::new(RefCell::new(Vec::new()));
    let table_bounds_listener = Rc::clone(&table_bounds);
    let table = MailTable {
        style: MailStyle {
            width: Some(560.0),
            ..Default::default()
        },
        rows: vec![MailTableRow {
            cells: vec![table_cell(vec![paragraph("OpenAI")], MailStyle::default())],
        }],
        cell_spacing: 0.0,
        row_spacing: 0.0,
        cell_padding: 0.0,
    };
    let options = mail_render_options();

    cx.draw(
        point(px(0.0), px(0.0)),
        size(px(650.0), px(100.0)),
        |_, _| {
            div()
                .w(px(650.0))
                .on_children_prepainted(move |bounds, _, _| {
                    table_bounds_listener.replace(bounds.to_vec());
                })
                .child(render_table(
                    &table,
                    &options,
                    "explicit-width-table",
                    650.0,
                ))
                .into_any_element()
        },
    );

    let measured = table_bounds.borrow();
    assert_eq!(measured.len(), 1);
    assert!(
        (measured[0].size.width.as_f32() - 560.0).abs() <= 1.0,
        "authored 560px table wrapper collapsed to {}px",
        measured[0].size.width.as_f32()
    );
}

#[gpui::test]
fn percentage_width_email_table_is_applied_once(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let wrapper_bounds: CellBounds = Rc::new(RefCell::new(Vec::new()));
    let wrapper_bounds_listener = Rc::clone(&wrapper_bounds);
    let table_bounds: CellBounds = Rc::new(RefCell::new(Vec::new()));
    let table_bounds_listener = Rc::clone(&table_bounds);
    let table = MailTable {
        style: MailStyle {
            width_percent: Some(0.5),
            ..Default::default()
        },
        rows: vec![MailTableRow {
            cells: vec![table_cell(
                vec![paragraph("Half width")],
                MailStyle::default(),
            )],
        }],
        cell_spacing: 0.0,
        row_spacing: 0.0,
        cell_padding: 0.0,
    };
    let options = mail_render_options();

    cx.draw(
        point(px(0.0), px(0.0)),
        size(px(600.0), px(100.0)),
        |_, _| {
            div()
                .w(px(600.0))
                .on_children_prepainted(move |bounds, _, _| {
                    wrapper_bounds_listener.replace(bounds.to_vec());
                })
                .child(
                    render_table(&table, &options, "percentage-width-table", 600.0)
                        .on_children_prepainted(move |bounds, _, _| {
                            table_bounds_listener.replace(bounds.to_vec());
                        }),
                )
                .into_any_element()
        },
    );

    let wrapper = wrapper_bounds.borrow();
    let inner = table_bounds.borrow();
    assert_eq!(wrapper.len(), 1);
    assert_eq!(inner.len(), 1);
    assert!((wrapper[0].size.width.as_f32() - 300.0).abs() <= 1.0);
    assert!((inner[0].size.width.as_f32() - 300.0).abs() <= 1.0);
}

#[gpui::test]
fn auto_margin_table_is_centered_exactly_once(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let table_bounds: CellBounds = Rc::new(RefCell::new(Vec::new()));
    let table_bounds_listener = Rc::clone(&table_bounds);
    let table = MailTable {
        style: MailStyle {
            width: Some(560.0),
            margin_left_auto: true,
            margin_right_auto: true,
            ..Default::default()
        },
        rows: vec![MailTableRow {
            cells: vec![table_cell(vec![paragraph("OpenAI")], MailStyle::default())],
        }],
        cell_spacing: 0.0,
        row_spacing: 0.0,
        cell_padding: 0.0,
    };
    let options = mail_render_options();

    cx.draw(
        point(px(0.0), px(0.0)),
        size(px(650.0), px(100.0)),
        |_, _| {
            div()
                .w(px(650.0))
                .child(
                    render_table(&table, &options, "centered-table", 650.0).on_children_prepainted(
                        move |bounds, _, _| {
                            table_bounds_listener.replace(bounds.to_vec());
                        },
                    ),
                )
                .into_any_element()
        },
    );

    let measured = table_bounds.borrow();
    assert_eq!(measured.len(), 1);
    assert!(
        (measured[0].origin.x.as_f32() - 45.0).abs() <= 1.0,
        "560px table in 650px parent should begin at 45px, got {}px",
        measured[0].origin.x.as_f32()
    );
    assert!((measured[0].size.width.as_f32() - 560.0).abs() <= 1.0);
}
