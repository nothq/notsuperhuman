use super::dom::{html_attr, safe_image_url, table_row_handles};
use super::style_values::parse_css_length;
use std::cell::RefCell;

use crate::ui::types::{
    MailBlock, MailEdgeInsets, MailImage, MailParagraph, MailStyle, MailTable, MailTableCell,
    MailTableRow, MailTextRun,
};
use html5ever::Attribute;
use markup5ever_rcdom::{Handle, NodeData};

use super::MailHtmlParser;

struct TableCellParseContext<'a> {
    node: &'a Handle,
    tag_name: &'a str,
    attrs: &'a RefCell<Vec<Attribute>>,
    inherited: &'a MailStyle,
    cell_padding: f32,
}

impl MailHtmlParser {
    pub(super) fn parse_table(
        &mut self,
        node: &Handle,
        attrs: &RefCell<Vec<Attribute>>,
        mut style: MailStyle,
    ) -> Vec<MailBlock> {
        if style.text_align_from_table_cell_attr {
            style.text_align = crate::ui::types::MailTextAlign::Start;
            style.text_align_from_table_cell_attr = false;
        }
        if html_attr(attrs, "align")
            .as_deref()
            .is_some_and(|align| align.eq_ignore_ascii_case("center"))
        {
            style.margin_left_auto = true;
            style.margin_right_auto = true;
        }
        let cell_padding = html_attr(attrs, "cellpadding")
            .as_deref()
            .and_then(parse_css_length)
            .unwrap_or(0.0);
        let rows = table_row_handles(node)
            .into_iter()
            .filter_map(|row| self.parse_table_row(&row, &style, cell_padding))
            .collect::<Vec<_>>();
        if rows.is_empty() {
            return self.parse_children(node, &style);
        }
        self.rich_layout = true;
        let html_cell_spacing = html_attr(attrs, "cellspacing")
            .as_deref()
            .and_then(parse_css_length)
            .unwrap_or(0.0);
        let (mut cell_spacing, mut row_spacing) = style
            .border_spacing
            .map(|spacing| (spacing.horizontal, spacing.vertical))
            .unwrap_or((html_cell_spacing, html_cell_spacing));
        // CSS 2.1 §17.6.2: the collapsing border model has neither table
        // padding nor spacing between cells. Emails set `border-collapse:
        // collapse` almost universally and still write both.
        if style.border_collapse == crate::ui::types::MailBorderCollapse::Collapse {
            style.padding = Default::default();
            (cell_spacing, row_spacing) = (0.0, 0.0);
        }
        vec![MailBlock::Table(MailTable {
            rows,
            style,
            cell_padding,
            cell_spacing,
            row_spacing,
        })]
    }

    fn parse_table_row(
        &mut self,
        node: &Handle,
        inherited: &MailStyle,
        cell_padding: f32,
    ) -> Option<MailTableRow> {
        let NodeData::Element { name, .. } = &node.data else {
            return None;
        };
        debug_assert_eq!(name.local.as_ref(), "tr");
        self.parse_table_row_cells(node, inherited, cell_padding)
    }

    fn parse_table_row_cells(
        &mut self,
        node: &Handle,
        inherited: &MailStyle,
        cell_padding: f32,
    ) -> Option<MailTableRow> {
        let cells = node
            .children
            .borrow()
            .iter()
            .filter_map(|child| match &child.data {
                NodeData::Element { attrs, name, .. }
                    if matches!(name.local.as_ref(), "td" | "th") =>
                {
                    self.parse_table_cell(TableCellParseContext {
                        node: child,
                        tag_name: name.local.as_ref(),
                        attrs,
                        inherited,
                        cell_padding,
                    })
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        (!cells.is_empty()).then_some(MailTableRow { cells })
    }

    fn parse_table_cell(&mut self, context: TableCellParseContext<'_>) -> Option<MailTableCell> {
        let mut style = self.style_from_attrs(
            context.node,
            context.tag_name,
            context.attrs,
            context.inherited,
        );
        if style.display_none {
            return None;
        }
        if context.cell_padding > 0.0 && !style.padding.any() {
            style.padding = MailEdgeInsets {
                top: context.cell_padding,
                right: context.cell_padding,
                bottom: context.cell_padding,
                left: context.cell_padding,
            };
        }
        let children = if self.can_collect_inline(context.node, &style) {
            self.parse_paragraph_element(context.node, MailStyle::inherit_text(&style), None)
        } else {
            self.parse_children(context.node, &style)
        };
        Some(MailTableCell {
            children,
            style,
            colspan: html_attr(context.attrs, "colspan")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(1)
                .max(1),
        })
    }

    pub(super) fn parse_image(
        &mut self,
        attrs: &RefCell<Vec<Attribute>>,
        style: MailStyle,
    ) -> Option<MailBlock> {
        let src = html_attr(attrs, "src").filter(|src| safe_image_url(src))?;
        let image = MailImage {
            width: style.width,
            height: style.height,
            alt: html_attr(attrs, "alt").unwrap_or_default(),
            src,
            style,
        };
        self.rich_layout = true;
        Some(MailBlock::Image(image))
    }

    pub(super) fn parse_list(
        &mut self,
        node: &Handle,
        name: &str,
        style: &MailStyle,
    ) -> Vec<MailBlock> {
        let ordered = name == "ol";
        node.children
            .borrow()
            .iter()
            .enumerate()
            .flat_map(|(index, child)| {
                let marker = if ordered {
                    format!("{}. ", index + 1)
                } else {
                    "- ".to_string()
                };
                self.parse_list_item(child, style, marker.as_str())
            })
            .collect()
    }

    pub(super) fn parse_list_item(
        &mut self,
        node: &Handle,
        style: &MailStyle,
        marker: &str,
    ) -> Vec<MailBlock> {
        let mut runs = vec![MailTextRun::text(marker)];
        runs.extend(self.collect_inline_runs(node, style.inline_style(), None));
        if runs.len() == 1 {
            return Vec::new();
        }
        vec![MailBlock::Paragraph(MailParagraph {
            runs,
            style: style.clone(),
        })]
    }
}
