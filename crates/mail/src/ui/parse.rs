use crate::ui::types::*;
use html5ever::{parse_document, tendril::TendrilSink, Attribute, ParseOpts};
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::{cell::RefCell, io::Cursor};

mod css;
mod dom;
mod flow;
mod inline;
mod style_values;
mod table;
#[cfg(test)]
mod tests;

use self::css::{MailCssCascadeContext, MailCssRules};
use self::dom::{
    collect_style_texts, default_display, find_body, html_attr, inline_html_element,
    node_is_inline_content, safe_link_url, skipped_html_element, text_block,
};
use self::flow::{block_is_inline_level, collapse_edge_margins};
use self::style_values::apply_html_style_attrs;

const MAIL_CSS_VIEWPORT_WIDTH: f32 = super::MAIL_RICH_BODY_WIDTH;

pub fn parse_mail_html(source: &str) -> Option<MailDocument> {
    let mut cursor = Cursor::new(source.as_bytes());
    let dom = parse_document(RcDom::default(), ParseOpts::default())
        .from_utf8()
        .read_from(&mut cursor)
        .ok()?;
    let mut parser = MailHtmlParser {
        css_rules: MailCssRules::from_style_texts(collect_style_texts(&dom.document)),
        ..Default::default()
    };
    let initial_style = MailStyle::html_initial();
    let blocks = if let Some(body) = find_body(&dom.document) {
        parser.parse_node(&body, &initial_style)
    } else {
        parser.parse_children(&dom.document, &initial_style)
    };
    let mut blocks = blocks;
    collapse_edge_margins(&mut blocks);
    let mut document = MailDocument::from_blocks(blocks);
    document.is_rich_layout |= parser.rich_layout;
    document.body_margin = parser.body_margin;
    document.body_background = parser.body_background;
    if document.is_empty() {
        None
    } else {
        Some(document)
    }
}

pub fn parse_plain_mail_body(text: &str, preview: &str) -> MailDocument {
    let paragraphs = text
        .trim()
        .split("\n\n")
        .map(|paragraph| paragraph.trim().replace('\n', " "))
        .filter(|paragraph| !paragraph.is_empty())
        .map(|paragraph| vec![MailTextRun::text(paragraph)])
        .collect::<Vec<_>>();
    if paragraphs.is_empty() {
        let fallback = preview.trim();
        if fallback.is_empty() {
            return MailDocument::default();
        }
        return MailDocument::from_text_paragraphs(vec![vec![MailTextRun::text(fallback)]]);
    }
    MailDocument::from_text_paragraphs(paragraphs)
}

#[derive(Default)]
pub(super) struct MailHtmlParser {
    rich_layout: bool,
    body_margin: MailEdgeInsets,
    body_background: Option<u32>,
    css_rules: MailCssRules,
}

impl MailHtmlParser {
    fn parse_node(&mut self, node: &Handle, inherited: &MailStyle) -> Vec<MailBlock> {
        match &node.data {
            NodeData::Text { contents } => text_block(&contents.borrow(), inherited),
            NodeData::Element { attrs, name, .. } => {
                self.parse_element(node, name.local.as_ref(), attrs, inherited)
            }
            _ => self.parse_children(node, inherited),
        }
    }

    fn parse_element(
        &mut self,
        node: &Handle,
        name: &str,
        attrs: &RefCell<Vec<Attribute>>,
        inherited: &MailStyle,
    ) -> Vec<MailBlock> {
        if skipped_html_element(name) {
            return Vec::new();
        }
        let mut style = self.style_from_attrs(node, name, attrs, inherited);
        style.line_align = inherited.text_align;
        if style.display_none {
            return Vec::new();
        }
        let link_target = if name == "a" {
            html_attr(attrs, "href").filter(|href| safe_link_url(href))
        } else {
            None
        };
        if matches!(name, "img" | "hr" | "br") {
            return match name {
                "img" => self.parse_image(attrs, style).into_iter().collect(),
                "hr" => vec![MailBlock::Rule(style)],
                "br" => vec![MailBlock::Spacer(
                    style
                        .line_height
                        .map(|line_height| line_height.resolve(style.font_size.unwrap_or(16.0)))
                        .unwrap_or(8.0),
                )],
                _ => unreachable!(),
            };
        }
        match name {
            "table" => self.parse_table(node, attrs, style),
            "ul" | "ol" => self.parse_list(node, name, &style),
            "li" => self.parse_list_item(node, &style, "- "),
            "p" => self.parse_container_element(node, style, None),
            "body" => self.parse_body(node, style),
            _ if inline_html_element(name) && style.has_box_style() => {
                self.parse_inline_box_element(node, style, link_target)
            }
            _ if inline_html_element(name) && self.can_collect_inline(node, &style) => {
                self.parse_paragraph_element(node, style, link_target)
            }
            _ => self.parse_container_element(node, style, link_target),
        }
    }

    fn parse_body(&mut self, node: &Handle, mut style: MailStyle) -> Vec<MailBlock> {
        // The body's margin insets the document rather than a box within it, so
        // it is taken off the style before anything else looks at it: leaving it
        // there would make every body a styled box and wrap the whole email in a
        // container it does not have.
        self.body_margin = std::mem::take(&mut style.margin);
        self.body_background = style.background_color.take();
        let children = self.parse_children(node, &style);
        if style.has_box_style() {
            self.rich_layout = true;
            vec![MailBlock::Container(MailContainer {
                children,
                style,
                link_target: None,
            })]
        } else {
            children
        }
    }

    fn parse_paragraph_element(
        &mut self,
        node: &Handle,
        style: MailStyle,
        link_target: Option<String>,
    ) -> Vec<MailBlock> {
        let runs = self.collect_inline_runs(node, style.inline_style(), link_target);
        if runs.is_empty() {
            Vec::new()
        } else {
            vec![MailBlock::Paragraph(MailParagraph { runs, style })]
        }
    }

    fn parse_container_element(
        &mut self,
        node: &Handle,
        style: MailStyle,
        link_target: Option<String>,
    ) -> Vec<MailBlock> {
        if self.can_collect_inline(node, &style) {
            let paragraph = self.parse_paragraph_element(node, style.clone(), link_target.clone());
            if paragraph.is_empty() && style.has_box_style() {
                self.rich_layout = true;
                return vec![MailBlock::Container(MailContainer {
                    children: Vec::new(),
                    style,
                    link_target,
                })];
            }
            return paragraph;
        }
        let children = self.parse_children(node, &style);
        if children.is_empty() {
            if style.has_box_style() {
                self.rich_layout = true;
                return vec![MailBlock::Container(MailContainer {
                    children: Vec::new(),
                    style,
                    link_target,
                })];
            }
            return Vec::new();
        }
        // An inline element holding only blocks generates no box of its own
        // (CSS 2.1 §9.2.1.1): its blocks stand in the flow, so their margins
        // meet their neighbours'. A logo wrapped in a link collapses its 4px
        // bottom margin into the next paragraph's 6px.
        let mut style = style;
        let is_inline_element = matches!(
            &node.data,
            NodeData::Element { name, .. } if inline_html_element(name.local.as_ref())
        );
        if is_inline_element && children.iter().all(|child| !block_is_inline_level(child)) {
            style.display = MailDisplay::Block;
        }
        let is_rich_layout =
            style.has_box_style() || children.iter().any(MailBlock::is_rich_layout);
        if is_rich_layout || link_target.is_some() {
            self.rich_layout |= is_rich_layout;
            vec![MailBlock::Container(MailContainer {
                children,
                style,
                link_target,
            })]
        } else {
            children
        }
    }

    fn parse_inline_box_element(
        &mut self,
        node: &Handle,
        style: MailStyle,
        link_target: Option<String>,
    ) -> Vec<MailBlock> {
        let text_style = MailStyle::inherit_text(&style);
        let children = if self.can_collect_inline(node, &text_style) {
            self.parse_paragraph_element(node, text_style, None)
        } else {
            self.parse_children(node, &text_style)
        };
        self.rich_layout = true;
        vec![MailBlock::Container(MailContainer {
            children,
            style,
            link_target,
        })]
    }

    fn can_collect_inline(&self, node: &Handle, inherited: &MailStyle) -> bool {
        node.children.borrow().iter().all(|child| {
            node_is_inline_content(child) && !self.inline_child_paints_box(child, inherited)
        })
    }

    fn inline_child_paints_box(&self, node: &Handle, inherited: &MailStyle) -> bool {
        let NodeData::Element { attrs, name, .. } = &node.data else {
            return false;
        };
        let name = name.local.as_ref();
        inline_html_element(name)
            && self
                .style_from_attrs(node, name, attrs, inherited)
                .has_box_style()
    }

    fn style_from_attrs(
        &self,
        node: &Handle,
        tag_name: &str,
        attrs: &RefCell<Vec<Attribute>>,
        inherited: &MailStyle,
    ) -> MailStyle {
        let mut style = MailStyle::inherit_text(inherited);
        style.display = default_display(tag_name);
        apply_user_agent_style(tag_name, inherited, &mut style);
        apply_html_style_attrs(tag_name, attrs, &mut style);
        self.css_rules.apply(
            MailCssCascadeContext::new(node, tag_name, attrs, inherited),
            &mut style,
        );
        style
    }
}

/// The browser default stylesheet for the block elements email bodies rely on.
/// Without it a `<p>` sits flush against its neighbours and a heading renders at
/// body size. Author attributes and CSS are applied after this and override it.
fn apply_user_agent_style(tag_name: &str, inherited: &MailStyle, style: &mut MailStyle) {
    let inherited_size = inherited.font_size.unwrap_or(16.0);
    let (scale, bold, margin_em) = match tag_name {
        "h1" => (2.0, true, 0.67),
        "h2" => (1.5, true, 0.83),
        "h3" => (1.17, true, 1.0),
        "h4" => (1.0, true, 1.33),
        "h5" => (0.83, true, 1.67),
        "h6" => (0.67, true, 2.33),
        "p" | "blockquote" | "pre" | "figure" => (1.0, false, 1.0),
        // A table resets text alignment. This is what lets the classic
        // `<center>`/`align="center"` wrapper centre the table box without
        // centring the text inside it, which is how most email bodies are built.
        "table" => {
            style.text_align = MailTextAlign::Start;
            return;
        }
        // Every browser insets the body by 8px. An authored `margin` is applied
        // after this and overrides it, which is what the near-universal email
        // reset does.
        "body" => {
            style.margin = MailEdgeInsets {
                top: 8.0,
                right: 8.0,
                bottom: 8.0,
                left: 8.0,
            };
            return;
        }
        // `<center>` is the presentational form of `text-align: center`.
        "center" => {
            style.text_align = MailTextAlign::Center;
            return;
        }
        _ => return,
    };
    if scale != 1.0 {
        style.font_size = Some(inherited_size * scale);
    }
    if bold {
        style.font_weight = Some(MailFontWeight::Bold);
    }
    let margin = margin_em * style.font_size.unwrap_or(inherited_size);
    style.margin.top = margin;
    style.margin.bottom = margin;
    if matches!(tag_name, "blockquote" | "figure") {
        style.margin.left = 40.0;
        style.margin.right = 40.0;
    }
}
