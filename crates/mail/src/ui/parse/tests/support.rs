use crate::ui::types::{MailBlock, MailImage, MailStyle, MailTable};

pub(super) fn classed_layout_html() -> &'static str {
    r##"
        <body>
          <style>
            body { background: rgb(242, 245, 247); }
            a { color: rgb(0, 133, 255); text-decoration: none; }
            .main-body-table { width: 600px; }
            .summary-table { border-spacing: 0px 20px; }
            .summary-table-td { width: 50%; }
            .color-brand { color: rgb(0, 133, 255) !important; }
            .main-table > tbody > tr > .w20 { display: none; }
          </style>
          <table class="main-table">
            <tbody>
              <tr>
                <td class="w20">hidden spacer</td>
                <td>
                  <table class="main-body-table" width="550">
                    <tbody>
                      <tr><td><strong class="color-brand">Daily summary</strong></td></tr>
                      <tr>
                        <td>
                          <table class="summary-table">
                            <tbody>
                              <tr>
                                <td class="summary-table-td">4</td>
                                <td class="summary-table-td"><a href="https://example.test">open report</a></td>
                              </tr>
                            </tbody>
                          </table>
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </td>
              </tr>
            </tbody>
          </table>
        </body>
    "##
}

pub(super) fn document_contains_table(blocks: &[MailBlock]) -> bool {
    blocks.iter().any(|block| match block {
        MailBlock::Table(_) => true,
        MailBlock::Container(container) => document_contains_table(&container.children),
        _ => false,
    })
}

pub(super) fn document_contains_image(blocks: &[MailBlock]) -> bool {
    blocks.iter().any(|block| match block {
        MailBlock::Image(_) => true,
        MailBlock::Container(container) => document_contains_image(&container.children),
        MailBlock::Table(table) => table.rows.iter().any(|row| {
            row.cells
                .iter()
                .any(|cell| document_contains_image(&cell.children))
        }),
        _ => false,
    })
}

pub(super) fn image_blocks(blocks: &[MailBlock]) -> Vec<&MailImage> {
    blocks
        .iter()
        .flat_map(|block| match block {
            MailBlock::Image(image) => vec![image],
            MailBlock::Container(container) => image_blocks(&container.children),
            MailBlock::Table(table) => table
                .rows
                .iter()
                .flat_map(|row| {
                    row.cells
                        .iter()
                        .flat_map(|cell| image_blocks(&cell.children))
                        .collect::<Vec<_>>()
                })
                .collect(),
            MailBlock::Paragraph(_) | MailBlock::Rule(_) | MailBlock::Spacer(_) => Vec::new(),
        })
        .collect()
}

pub(super) fn first_background_image_style(blocks: &[MailBlock]) -> Option<&MailStyle> {
    blocks.iter().find_map(|block| match block {
        MailBlock::Container(container) => container
            .style
            .background_image_url
            .as_ref()
            .map(|_| &container.style)
            .or_else(|| first_background_image_style(&container.children)),
        MailBlock::Table(table) => table.rows.iter().find_map(|row| {
            row.cells.iter().find_map(|cell| {
                cell.style
                    .background_image_url
                    .as_ref()
                    .map(|_| &cell.style)
                    .or_else(|| first_background_image_style(&cell.children))
            })
        }),
        MailBlock::Paragraph(paragraph) => paragraph
            .style
            .background_image_url
            .as_ref()
            .map(|_| &paragraph.style),
        MailBlock::Rule(style) => style.background_image_url.as_ref().map(|_| style),
        MailBlock::Image(_) | MailBlock::Spacer(_) => None,
    })
}

pub(super) fn first_table(blocks: &[MailBlock]) -> Option<&MailTable> {
    blocks.iter().find_map(|block| match block {
        MailBlock::Table(table) => Some(table),
        MailBlock::Container(container) => first_table(&container.children),
        _ => None,
    })
}

pub(super) fn table_with_width(blocks: &[MailBlock], width: f32) -> Option<&MailTable> {
    blocks.iter().find_map(|block| match block {
        MailBlock::Table(table) if table.style.width == Some(width) => Some(table),
        MailBlock::Table(table) => table.rows.iter().find_map(|row| {
            row.cells
                .iter()
                .find_map(|cell| table_with_width(&cell.children, width))
        }),
        MailBlock::Container(container) => table_with_width(&container.children, width),
        _ => None,
    })
}

pub(super) fn table_with_row_spacing(blocks: &[MailBlock], row_spacing: f32) -> Option<&MailTable> {
    blocks.iter().find_map(|block| match block {
        MailBlock::Table(table) if table.row_spacing == row_spacing => Some(table),
        MailBlock::Table(table) => table.rows.iter().find_map(|row| {
            row.cells
                .iter()
                .find_map(|cell| table_with_row_spacing(&cell.children, row_spacing))
        }),
        MailBlock::Container(container) => table_with_row_spacing(&container.children, row_spacing),
        _ => None,
    })
}
