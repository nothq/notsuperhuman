use super::dom::{collapse_html_text, html_attr, safe_link_url, skipped_html_element};
use super::style_values::parse_css_color;
use std::cell::RefCell;

use crate::ui::types::{MailFontStyle, MailFontWeight, MailInlineStyle, MailStyle, MailTextRun};
use html5ever::{local_name, Attribute};
use markup5ever_rcdom::{Handle, NodeData};

use super::css::MailCssCascadeContext;
use super::MailHtmlParser;

#[derive(Default)]
struct InlineRunAccumulator {
    pending_space: bool,
    runs: Vec<MailTextRun>,
}

impl MailHtmlParser {
    pub(super) fn collect_inline_runs(
        &mut self,
        node: &Handle,
        style: MailInlineStyle,
        href: Option<String>,
    ) -> Vec<MailTextRun> {
        let mut accumulator = InlineRunAccumulator::default();
        for child in node.children.borrow().iter() {
            self.collect_inline_runs_into(child, style, href.clone(), &mut accumulator);
        }
        trim_block_end(&mut accumulator.runs);
        accumulator.runs
    }

    /// The runs of a sequence of sibling nodes, as one line of inline content.
    /// The runs of a sequence of sibling nodes, untrimmed: the caller settles
    /// the breaks at either end against the siblings around the sequence.
    pub(super) fn collect_inline_runs_from(
        &mut self,
        nodes: impl Iterator<Item = Handle>,
        style: MailInlineStyle,
    ) -> Vec<MailTextRun> {
        let mut accumulator = InlineRunAccumulator::default();
        for node in nodes {
            self.collect_inline_runs_into(&node, style, None, &mut accumulator);
        }
        accumulator.runs
    }

    fn collect_inline_runs_into(
        &mut self,
        node: &Handle,
        style: MailInlineStyle,
        href: Option<String>,
        accumulator: &mut InlineRunAccumulator,
    ) {
        match &node.data {
            NodeData::Text { contents } => push_inline_text(
                &mut accumulator.runs,
                &contents.borrow(),
                style,
                href,
                &mut accumulator.pending_space,
            ),
            NodeData::Element { attrs, name, .. } => {
                if skipped_html_element(name.local.as_ref()) {
                    return;
                }
                if name.local == local_name!("br") {
                    push_inline_break(&mut accumulator.runs);
                    accumulator.pending_space = false;
                    return;
                }
                let next_style =
                    self.inline_style_from_element(node, name.local.as_ref(), attrs, style);
                let next_href = if name.local == local_name!("a") {
                    html_attr(attrs, "href")
                        .filter(|href| safe_link_url(href))
                        .or(href)
                } else {
                    href
                };
                for child in node.children.borrow().iter() {
                    self.collect_inline_runs_into(
                        child,
                        next_style,
                        next_href.clone(),
                        accumulator,
                    );
                }
            }
            _ => {
                for child in node.children.borrow().iter() {
                    self.collect_inline_runs_into(child, style, href.clone(), accumulator);
                }
            }
        }
    }

    fn inline_style_from_element(
        &self,
        node: &Handle,
        name: &str,
        attrs: &RefCell<Vec<Attribute>>,
        inherited: MailInlineStyle,
    ) -> MailInlineStyle {
        let inherited_style = MailStyle {
            color: inherited.color,
            font_family: inherited.font_family,
            font_style: if inherited.italic {
                MailFontStyle::Italic
            } else {
                MailFontStyle::Normal
            },
            font_size: inherited.font_size,
            line_height: inherited
                .line_height
                .map(|line_height| line_height.inherited(inherited.font_size.unwrap_or(16.0))),
            font_weight: inherited.font_weight,
            text_decoration: inherited.text_decoration,
            text_transform: inherited.text_transform,
            preserve_whitespace: inherited.preserve_whitespace,
            ..Default::default()
        };
        let mut style = inherited_style.clone();
        if let Some(color) = html_attr(attrs, "color")
            .as_deref()
            .and_then(parse_css_color)
        {
            style.color = Some(color);
        }
        match name {
            // The UA default for a link is an underline; author CSS below can
            // remove it, which many emails do for buttons. Only an href we will
            // actually link to counts, so a dropped `javascript:` URL is not
            // painted as a link.
            "a" if html_attr(attrs, "href").is_some_and(|href| safe_link_url(href.as_str())) => {
                style.text_decoration.underline = true;
            }
            "b" | "strong" => style.font_weight = Some(MailFontWeight::Bold),
            "em" | "i" => style.font_style = MailFontStyle::Italic,
            "small" => style.font_size = Some(style.font_size.unwrap_or(14.0) * 0.86),
            _ => {}
        }
        self.css_rules.apply(
            MailCssCascadeContext::new(node, name, attrs, &inherited_style),
            &mut style,
        );
        style.inline_style()
    }
}

fn push_inline_text(
    runs: &mut Vec<MailTextRun>,
    text: &str,
    style: MailInlineStyle,
    href: Option<String>,
    pending_space: &mut bool,
) {
    let leading_whitespace = text.chars().next().is_some_and(char::is_whitespace);
    let trailing_whitespace = text.chars().next_back().is_some_and(char::is_whitespace);
    let authored_blank_line = !style.preserve_whitespace
        && text.contains('\u{00a0}')
        && text.chars().all(char::is_whitespace);
    let text = if style.preserve_whitespace {
        normalize_preserved_html_text(text)
    } else if authored_blank_line {
        "\u{00a0}".to_string()
    } else {
        collapse_html_text(text)
    };
    if text.trim().is_empty() && !authored_blank_line {
        if !style.preserve_whitespace && !runs.is_empty() {
            *pending_space |= leading_whitespace || trailing_whitespace;
        }
        return;
    }
    let needs_space = !style.preserve_whitespace
        && (*pending_space || leading_whitespace)
        && runs
            .last()
            .is_some_and(|run| !run.text.ends_with(" ") && !run.text.ends_with("\n"));
    if needs_space {
        // Whitespace between two inline elements belongs to the enclosing text
        // flow, not to the element that follows it. Prepending it to the incoming
        // run pulled it inside links, so the underline ran under the space.
        if let Some(last) = runs.last_mut() {
            last.text.push(' ');
        }
    }
    if let Some(last) = runs
        .last_mut()
        .filter(|run| run.href == href && run.style == style)
    {
        last.text.push_str(&text);
        *pending_space = !authored_blank_line && !style.preserve_whitespace && trailing_whitespace;
        return;
    }
    runs.push(MailTextRun { text, href, style });
    *pending_space = !authored_blank_line && !style.preserve_whitespace && trailing_whitespace;
}

fn normalize_preserved_html_text(text: &str) -> String {
    text.replace("\r\n", "\n").replace("\r", "\n")
}

/// Drop what a browser drops at the end of a block: collapsible trailing
/// whitespace, and then the last `<br>`.
///
/// A break ends a line, and a line nothing follows is never laid out: `a<br>`
/// is one line high, exactly like `a`. Keeping that break added a blank line to
/// every line of an Outlook signature, which ends all of them with one. A block
/// whose only content was the break still keeps its single empty line.
pub(super) fn trim_block_end(runs: &mut Vec<MailTextRun>) -> bool {
    trim_trailing_space(runs);
    if runs.last().is_some_and(|run| run.text == "\n") {
        runs.pop();
        trim_trailing_space(runs);
        if runs.is_empty() {
            runs.push(MailTextRun {
                text: String::new(),
                href: None,
                style: MailInlineStyle::default(),
            });
        }
        return true;
    }
    false
}

/// Drops a `<br>` that opens the run, and the collapsible whitespace around
/// it; reports whether there was one.
pub(super) fn strip_leading_break(runs: &mut Vec<MailTextRun>) -> bool {
    let Some(index) = runs.iter().position(|run| !is_collapsible_space(run)) else {
        return false;
    };
    if runs[index].text != "\n" {
        return false;
    }
    runs.drain(..=index);
    while runs.first().is_some_and(is_collapsible_space) {
        runs.remove(0);
    }
    true
}

/// Only the collapsible characters count: a non-breaking space is content,
/// and a blank paragraph built from one is a blank line the author asked for.
fn is_collapsible_space(run: &MailTextRun) -> bool {
    !run.text.is_empty()
        && run
            .text
            .chars()
            .all(|character| matches!(character, ' ' | '\t' | '\r' | '\u{000C}'))
}

fn trim_trailing_space(runs: &mut Vec<MailTextRun>) {
    while runs.last().is_some_and(is_collapsible_space) {
        runs.pop();
    }
}

fn push_inline_break(runs: &mut Vec<MailTextRun>) {
    // Every `<br>` forces its own break. They are elements, not whitespace, so
    // consecutive ones are never collapsed — `<br><br>` leaves a blank line, and
    // that is how a great deal of email separates its paragraphs.
    runs.push(MailTextRun {
        text: "\n".to_string(),
        href: None,
        style: MailInlineStyle::default(),
    });
}
