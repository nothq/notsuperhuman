//! Block flow: the anonymous boxes CSS wraps around inline-level content,
//! and the margins that collapse between a block and its first or last child.

use super::dom::node_is_inline_content;
use super::inline::{strip_leading_break, trim_block_end};
use super::MailHtmlParser;
use crate::ui::types::*;
use markup5ever_rcdom::Handle;

/// Whether a run of inline-level siblings directly follows, or directly
/// precedes, an inline box that paints.
struct InlineRunNeighbors {
    after_inline_box: bool,
    before_inline_box: bool,
}

impl MailHtmlParser {
    pub(super) fn parse_children(
        &mut self,
        node: &Handle,
        inherited: &MailStyle,
    ) -> Vec<MailBlock> {
        // CSS 2.1 §9.2.1.1: consecutive inline-level siblings share one
        // anonymous block box. Giving every text node and every `<a>` a
        // paragraph of its own set each on its own line as soon as one block
        // sibling — a footer's copyright `<div>`, say — sat beside them.
        let mut blocks = Vec::new();
        let mut inline = Vec::new();
        let mut after_inline_box = false;
        for child in node.children.borrow().iter() {
            if node_is_inline_content(child) && !self.inline_child_paints_box(child, inherited) {
                inline.push(child.clone());
                continue;
            }
            let parsed = self.parse_node(child, inherited);
            let inline_box = !parsed.is_empty() && parsed.iter().all(block_is_inline_level);
            self.flush_inline_run(
                &mut inline,
                inherited,
                InlineRunNeighbors {
                    after_inline_box,
                    before_inline_box: inline_box,
                },
                &mut blocks,
            );
            after_inline_box = inline_box;
            blocks.extend(parsed);
        }
        self.flush_inline_run(
            &mut inline,
            inherited,
            InlineRunNeighbors {
                after_inline_box,
                before_inline_box: false,
            },
            &mut blocks,
        );
        // Beside block siblings, an inline image sits on a line of its own,
        // which the block's strut gives a height: a 1px tracking pixel after
        // the last `<div>` is an 18.5px line in a 16px Times body.
        if blocks
            .iter()
            .any(|block| !block_is_inline_level(block) && !matches!(block, MailBlock::Spacer(_)))
        {
            for block in &mut blocks {
                if matches!(block, MailBlock::Image(_)) && block_is_inline_level(block) {
                    let image = std::mem::replace(block, MailBlock::Spacer(0.0));
                    *block = MailBlock::Container(MailContainer {
                        children: vec![image],
                        style: MailStyle::inherit_text(inherited),
                        link_target: None,
                    });
                }
            }
        }
        blocks
    }

    /// The anonymous block box around a run of inline-level siblings.
    ///
    /// An inline box that paints — a button — is laid out as a box of its own
    /// here, but in the source it shares a line with the text around it. A
    /// `<br>` between the two is a line break within that shared line, which
    /// the renderer models as a [`MailBlock::Spacer`] between inline-level
    /// siblings: a name, a break and a button stack; a button, `<br><br>` and
    /// a table are the button, one empty line and the table.
    fn flush_inline_run(
        &mut self,
        nodes: &mut Vec<Handle>,
        inherited: &MailStyle,
        neighbors: InlineRunNeighbors,
        blocks: &mut Vec<MailBlock>,
    ) {
        let InlineRunNeighbors {
            after_inline_box,
            before_inline_box,
        } = neighbors;
        if nodes.is_empty() {
            return;
        }
        let mut text_style = MailStyle::inherit_text(inherited);
        text_style.display = MailDisplay::Inline;
        let mut runs = self.collect_inline_runs_from(nodes.drain(..), text_style.inline_style());
        // The leading break is settled first: a lone `<br>` after a button is
        // that button's line ending, not a blank line of its own.
        if after_inline_box && strip_leading_break(&mut runs) {
            blocks.push(MailBlock::Spacer(0.0));
        }
        let ended_with_break = trim_block_end(&mut runs);
        // A run list that survives trimming is a line — the empty run a lone
        // `<br>` leaves behind included, since that is the blank line the
        // author asked for.
        if !runs.is_empty() {
            blocks.push(MailBlock::Paragraph(MailParagraph {
                runs,
                style: text_style,
            }));
        }
        if ended_with_break && before_inline_box {
            blocks.push(MailBlock::Spacer(0.0));
        }
    }
}

/// CSS 2.1 §8.3.1: a block's margin collapses through the edge of its parent
/// when nothing — no padding, no border — separates them, and the larger of
/// the two survives on the parent. A paragraph with a 28px bottom margin at
/// the end of a wrapper carrying 72px therefore ends 72px before what
/// follows, not 100px. Sibling margins collapse at render time; this pass
/// settles the parent-child case on the tree, so the render sees one margin
/// where the browser sees one.
pub(super) fn collapse_edge_margins(blocks: &mut [MailBlock]) {
    for block in blocks.iter_mut() {
        let MailBlock::Container(container) = block else {
            if let MailBlock::Table(table) = block {
                for cell in table.rows.iter_mut().flat_map(|row| row.cells.iter_mut()) {
                    collapse_edge_margins(&mut cell.children);
                }
            }
            continue;
        };
        collapse_edge_margins(&mut container.children);
        if !margins_collapse_through(&container.style) {
            continue;
        }
        if container.style.padding.top == 0.0 && container.style.border_width.unwrap_or(0.0) == 0.0
        {
            if let Some(child) = container
                .children
                .first_mut()
                .and_then(collapsing_style_mut)
            {
                container.style.margin.top = container.style.margin.top.max(child.margin.top);
                child.margin.top = 0.0;
            }
        }
        if container.style.padding.bottom == 0.0
            && container.style.border_width.unwrap_or(0.0) == 0.0
        {
            if let Some(child) = container.children.last_mut().and_then(collapsing_style_mut) {
                container.style.margin.bottom =
                    container.style.margin.bottom.max(child.margin.bottom);
                child.margin.bottom = 0.0;
            }
        }
    }
}

fn margins_collapse_through(style: &MailStyle) -> bool {
    style.float == MailFloat::None
        && style.position == MailPosition::Static
        && matches!(style.display, MailDisplay::Block | MailDisplay::Unspecified)
        && style.height.is_none()
}

fn collapsing_style_mut(block: &mut MailBlock) -> Option<&mut MailStyle> {
    let style = match block {
        MailBlock::Container(container) => &mut container.style,
        MailBlock::Paragraph(paragraph) => &mut paragraph.style,
        // A block image's margin reaches through a link wrapped round it: a
        // logo's 4px bottom margin and the next paragraph's 6px top margin
        // are one 6px margin, not 10px.
        MailBlock::Image(image) if image.style.display == MailDisplay::Block => &mut image.style,
        _ => return None,
    };
    margins_collapse_through(style).then_some(style)
}

/// A parsed sibling that shares a line with inline content: an inline or
/// inline-block box, or an image, which is inline-level unless made a block.
pub(super) fn block_is_inline_level(block: &MailBlock) -> bool {
    match block {
        MailBlock::Container(container) => container.style.is_inline_level(),
        MailBlock::Paragraph(paragraph) => paragraph.style.is_inline_level(),
        MailBlock::Image(image) => image.style.display != MailDisplay::Block,
        MailBlock::Table(_) | MailBlock::Rule(_) | MailBlock::Spacer(_) => false,
    }
}
