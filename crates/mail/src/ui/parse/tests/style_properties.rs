use crate::ui::types::{
    MailBackgroundPosition, MailBackgroundRepeat, MailBlock, MailDirection, MailDisplay,
    MailFontStyle, MailOverflow, MailParagraph, MailStyle, MailTableCell, MailTextTransform,
    MailVerticalAlign,
};

use super::parse_mail_html;

pub(super) fn hubspot_layout_css_properties_survive_parsing() {
    let html = r##"
        <body>
          <style>
            .hse-section .hse-size-6 {
              display: table-cell;
              width: 300px;
              min-width: 280px;
              max-width: 300px;
            }
            .hse-section .fluid { max-width: 100%; }
          </style>
          <div class="hse-section"
               style="direction:rtl; overflow:hidden; background-repeat:no-repeat; background-position:center">
            <div class="hse-size-6"
                 style="vertical-align:top; border-top:1px solid #23496d; font-style:italic; text-decoration:underline; text-transform:uppercase">
              inside
            </div>
            <div class="fluid">fluid</div>
            <table><tr><td valign="middle">cell</td></tr></table>
          </div>
        </body>
    "##;
    let document = parse_mail_html(html).expect("html document");
    let section = first_container(&document.blocks).expect("section container");
    let sized = paragraph_with_text(&document.blocks, "inside").expect("sized paragraph");
    let fluid = paragraph_with_text(&document.blocks, "fluid").expect("fluid paragraph");
    let cell = first_table_cell(&document.blocks).expect("table cell");

    assert_section_style(&section.style);
    assert_sized_column_style(&sized.style);
    assert_eq!(fluid.style.max_width_percent, Some(1.0));
    assert_eq!(cell.style.vertical_align, MailVerticalAlign::Middle);
}

fn assert_section_style(style: &MailStyle) {
    assert_eq!(style.direction, MailDirection::Rtl);
    assert_eq!(style.overflow, MailOverflow::Hidden);
    assert_eq!(style.background_repeat, MailBackgroundRepeat::NoRepeat);
    assert_eq!(style.background_position, MailBackgroundPosition::Center);
}

fn assert_sized_column_style(style: &MailStyle) {
    assert_eq!(style.display, MailDisplay::TableCell);
    assert_eq!(style.vertical_align, MailVerticalAlign::Top);
    assert_eq!(style.width, Some(300.0));
    assert_eq!(style.min_width, Some(280.0));
    assert_eq!(style.max_width, Some(300.0));
    assert_eq!(style.border_edges.top.width, Some(1.0));
    assert_eq!(style.border_edges.top.color, Some(0x23496d));
    assert_eq!(style.font_style, MailFontStyle::Italic);
    assert!(style.text_decoration.underline);
    assert_eq!(style.text_transform, MailTextTransform::Uppercase);
}

fn first_container(blocks: &[MailBlock]) -> Option<&crate::ui::types::MailContainer> {
    blocks.iter().find_map(|block| match block {
        MailBlock::Container(container) => Some(container),
        MailBlock::Table(table) => table.rows.iter().find_map(|row| {
            row.cells
                .iter()
                .find_map(|cell| first_container(&cell.children))
        }),
        _ => None,
    })
}

fn paragraph_with_text<'a>(blocks: &'a [MailBlock], text: &str) -> Option<&'a MailParagraph> {
    blocks.iter().find_map(|block| match block {
        MailBlock::Paragraph(paragraph)
            if paragraph.runs.iter().any(|run| run.text.contains(text)) =>
        {
            Some(paragraph)
        }
        MailBlock::Container(container) => paragraph_with_text(&container.children, text),
        MailBlock::Table(table) => table.rows.iter().find_map(|row| {
            row.cells
                .iter()
                .find_map(|cell| paragraph_with_text(&cell.children, text))
        }),
        _ => None,
    })
}

fn first_table_cell(blocks: &[MailBlock]) -> Option<&MailTableCell> {
    blocks.iter().find_map(|block| match block {
        MailBlock::Table(table) => table.rows.iter().find_map(|row| row.cells.first()),
        MailBlock::Container(container) => first_table_cell(&container.children),
        _ => None,
    })
}
