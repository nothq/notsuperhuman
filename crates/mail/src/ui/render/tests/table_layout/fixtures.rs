use super::{
    MailBlock, MailContainer, MailDirection, MailDisplay, MailEdgeInsets, MailImage, MailParagraph,
    MailStyle, MailTable, MailTableCell, MailTableRow, MailTextRun, MailVerticalAlign,
};

pub(super) fn spacer_and_fixed_content_table() -> MailTable {
    full_width_table(
        vec![
            table_cell(Vec::new(), MailStyle::default()),
            table_cell(
                vec![MailBlock::Table(fixed_invoice_table())],
                MailStyle::default(),
            ),
        ],
        0.0,
    )
}

fn fixed_invoice_table() -> MailTable {
    full_width_table(
        vec![table_cell(
            vec![MailBlock::Container(MailContainer {
                link_target: None,
                children: vec![paragraph("Invoice from Nash Technologies Inc.")],
                style: MailStyle {
                    background_color: Some(0xffffff),
                    padding: MailEdgeInsets {
                        top: 24.0,
                        right: 32.0,
                        bottom: 24.0,
                        left: 32.0,
                    },
                    ..Default::default()
                },
            })],
            MailStyle {
                width_percent: Some(1.0),
                ..Default::default()
            },
        )],
        0.0,
    )
}

pub(super) fn label_spacer_value_table() -> MailTable {
    full_width_table(
        vec![
            table_cell(vec![paragraph("Memo")], MailStyle::default()),
            table_cell(
                Vec::new(),
                MailStyle {
                    width: Some(24.0),
                    ..Default::default()
                },
            ),
            table_cell(
                vec![paragraph(
                    "Deliveries performed 06/01/26 - 06/07/26 Total deliveries: 1100 across every \
                 serviced zone, including redeliveries and operator adjustments",
                )],
                MailStyle::default(),
            ),
        ],
        0.0,
    )
}

pub(super) fn logo_and_text_table() -> MailTable {
    full_width_table(
        vec![
            table_cell(
                vec![MailBlock::Container(logo_container())],
                MailStyle::default(),
            ),
            table_cell(
                vec![paragraph("Nash Technologies Inc.")],
                MailStyle::default(),
            ),
        ],
        12.0,
    )
}

pub(super) fn hubspot_two_column_table(direction: MailDirection) -> MailTable {
    MailTable {
        style: MailStyle {
            width: Some(600.0),
            direction,
            ..Default::default()
        },
        rows: vec![MailTableRow {
            cells: vec![
                table_cell(vec![paragraph("Dedicated Servers")], hubspot_column_style()),
                table_cell(
                    vec![MailBlock::Image(MailImage {
                        src: "https://example.test/product.png".to_string(),
                        alt: "Dedicated Servers".to_string(),
                        width: Some(260.0),
                        height: Some(180.0),
                        style: MailStyle::default(),
                    })],
                    hubspot_column_style(),
                ),
            ],
        }],
        cell_spacing: 0.0,
        row_spacing: 0.0,
        cell_padding: 0.0,
    }
}

fn hubspot_column_style() -> MailStyle {
    MailStyle {
        display: MailDisplay::TableCell,
        width: Some(300.0),
        max_width: Some(300.0),
        vertical_align: MailVerticalAlign::Top,
        ..Default::default()
    }
}

pub(super) fn vertical_align_cell(vertical_align: MailVerticalAlign) -> MailTableCell {
    table_cell(
        vec![MailBlock::Container(MailContainer {
            link_target: None,
            children: Vec::new(),
            style: MailStyle {
                background_color: Some(0xff0000),
                width: Some(20.0),
                height: Some(20.0),
                ..Default::default()
            },
        })],
        MailStyle {
            width: Some(80.0),
            height: Some(90.0),
            vertical_align,
            ..Default::default()
        },
    )
}

pub(super) fn unequal_height_vertical_align_table() -> MailTable {
    full_width_table(
        vec![
            table_cell(
                vec![MailBlock::Container(MailContainer {
                    link_target: None,
                    children: Vec::new(),
                    style: MailStyle {
                        background_color: Some(0xff0000),
                        height: Some(90.0),
                        ..Default::default()
                    },
                })],
                MailStyle {
                    width: Some(80.0),
                    vertical_align: MailVerticalAlign::Top,
                    ..Default::default()
                },
            ),
            table_cell(
                vec![MailBlock::Container(MailContainer {
                    link_target: None,
                    children: Vec::new(),
                    style: MailStyle {
                        background_color: Some(0x0000ff),
                        height: Some(20.0),
                        ..Default::default()
                    },
                })],
                MailStyle {
                    width: Some(80.0),
                    vertical_align: MailVerticalAlign::Bottom,
                    ..Default::default()
                },
            ),
        ],
        0.0,
    )
}

fn logo_container() -> MailContainer {
    MailContainer {
        link_target: None,
        children: Vec::new(),
        style: MailStyle {
            background_color: Some(0xffffff),
            width: Some(32.0),
            height: Some(32.0),
            border_radius: Some(9999.0),
            ..Default::default()
        },
    }
}

fn full_width_table(cells: Vec<MailTableCell>, cell_spacing: f32) -> MailTable {
    MailTable {
        style: MailStyle {
            width_percent: Some(1.0),
            ..Default::default()
        },
        rows: vec![MailTableRow { cells }],
        cell_spacing,
        row_spacing: 0.0,
        cell_padding: 0.0,
    }
}

pub(super) fn table_cell(children: Vec<MailBlock>, style: MailStyle) -> MailTableCell {
    MailTableCell {
        children,
        style,
        colspan: 1,
    }
}

pub(super) fn paragraph(text: impl Into<String>) -> MailBlock {
    MailBlock::Paragraph(MailParagraph {
        runs: vec![MailTextRun::text(text.into())],
        style: MailStyle::default(),
    })
}
