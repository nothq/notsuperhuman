use crate::ui::types::*;
use html5ever::{local_name, Attribute};
use markup5ever_rcdom::{Handle, NodeData};
use std::cell::RefCell;

pub(super) fn find_body(node: &Handle) -> Option<Handle> {
    if let NodeData::Element { name, .. } = &node.data {
        if name.local == local_name!("body") {
            return Some(node.clone());
        }
    }
    node.children.borrow().iter().find_map(find_body)
}

pub(super) fn collect_style_texts(node: &Handle) -> Vec<String> {
    let mut styles = Vec::new();
    collect_style_texts_into(node, &mut styles);
    styles
}

fn collect_style_texts_into(node: &Handle, styles: &mut Vec<String>) {
    if let NodeData::Element { name, .. } = &node.data {
        if name.local == local_name!("style") {
            let style = node
                .children
                .borrow()
                .iter()
                .filter_map(|child| match &child.data {
                    NodeData::Text { contents } => Some(contents.borrow().to_string()),
                    _ => None,
                })
                .collect::<String>();
            if !style.trim().is_empty() {
                styles.push(style);
            }
            return;
        }
    }
    for child in node.children.borrow().iter() {
        collect_style_texts_into(child, styles);
    }
}

pub(super) fn text_block(text: &str, inherited: &MailStyle) -> Vec<MailBlock> {
    let collapsed = collapse_html_text(text);
    if collapsed.is_empty() {
        return Vec::new();
    }
    // An anonymous box wrapping bare text still holds inline content, so it must
    // stay inline-level; otherwise it would force block layout on its siblings.
    let mut text_style = MailStyle::inherit_text(inherited);
    text_style.display = MailDisplay::Inline;
    vec![MailBlock::Paragraph(MailParagraph {
        runs: vec![MailTextRun {
            text: collapsed,
            href: None,
            style: text_style.inline_style(),
        }],
        style: text_style,
    })]
}

pub(super) fn table_row_handles(node: &Handle) -> Vec<Handle> {
    let mut rows = Vec::new();
    collect_table_rows(node, &mut rows);
    rows
}

fn collect_table_rows(node: &Handle, rows: &mut Vec<Handle>) {
    for child in node.children.borrow().iter() {
        match &child.data {
            NodeData::Element { name, .. } if name.local == local_name!("tr") => {
                rows.push(child.clone());
            }
            NodeData::Element { name, .. }
                if matches!(name.local.as_ref(), "thead" | "tbody" | "tfoot") =>
            {
                collect_table_rows(child, rows);
            }
            _ => {}
        }
    }
}

pub(super) fn node_is_inline_content(node: &Handle) -> bool {
    match &node.data {
        NodeData::Text { .. } => true,
        NodeData::Element { name, .. } if inline_html_element(name.local.as_ref()) => {
            node.children.borrow().iter().all(node_is_inline_content)
        }
        NodeData::Element { .. } => false,
        _ => true,
    }
}

pub(super) fn skipped_html_element(name: &str) -> bool {
    matches!(
        name,
        "head"
            | "script"
            | "style"
            | "title"
            | "meta"
            | "iframe"
            | "object"
            | "embed"
            | "video"
            | "audio"
            | "canvas"
            | "form"
            | "input"
            | "button"
            | "textarea"
            | "svg"
    )
}

pub(super) fn inline_html_element(name: &str) -> bool {
    matches!(
        name,
        "a" | "abbr"
            | "b"
            | "br"
            | "code"
            | "em"
            | "font"
            | "i"
            | "label"
            | "small"
            | "span"
            | "strong"
            | "sub"
            | "sup"
            | "u"
    )
}

/// The used `display` every HTML element starts from, before author CSS. Layout
/// decisions downstream are made from this value rather than guessed from an
/// element's siblings, so it must be resolved for every element the parser sees.
pub(super) fn default_display(name: &str) -> MailDisplay {
    match name {
        "table" => MailDisplay::Table,
        "td" | "th" => MailDisplay::TableCell,
        // Replaced elements and text-level semantics are inline by default.
        "img" => MailDisplay::Inline,
        name if inline_html_element(name) => MailDisplay::Inline,
        _ => MailDisplay::Block,
    }
}

pub(super) fn html_attr(attrs: &RefCell<Vec<Attribute>>, name: &str) -> Option<String> {
    attrs
        .borrow()
        .iter()
        .find(|attr| attr.name.local.as_ref().eq_ignore_ascii_case(name))
        .map(|attr| attr.value.to_string())
}

pub(super) fn collapse_html_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn safe_link_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
}

pub(super) fn safe_image_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("data:image/")
}
