use std::ops::Range;

use super::{EditorLayoutLine, EditorSnapshot, LongFormEditor, CARET_BLINK_MS};
use gpui::{point, px, Bounds, Context, Pixels, Point, Window};
use unicode_segmentation::UnicodeSegmentation;

impl LongFormEditor {
    pub(super) fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset.min(self.content.len())..offset.min(self.content.len());
        self.selection_reversed = false;
        self.marked_range = None;
        self.reset_caret_blink();
        self.ensure_cursor_visible();
        cx.notify();
    }

    pub(super) fn move_or_select_to(
        &mut self,
        offset: usize,
        selecting: bool,
        cx: &mut Context<Self>,
    ) {
        if selecting {
            self.select_to(offset, cx);
        } else {
            self.move_to(offset, cx);
        }
    }

    pub(super) fn move_or_select_to_document_edge(
        &mut self,
        start: bool,
        selecting: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        self.desired_x = None;
        let offset = if start { 0 } else { self.content.len() };
        self.move_or_select_to(offset, selecting, cx);
        true
    }

    pub(super) fn move_or_select_to_visual_line_edge(
        &mut self,
        start: bool,
        selecting: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        self.desired_x = None;
        let Some(bounds) = self.last_bounds else {
            return true;
        };
        let Some(cursor) = self.content_position_for_index(self.cursor_offset()) else {
            return true;
        };
        let x = if start {
            px(0.0)
        } else {
            bounds.size.width.max(px(0.0))
        };
        let offset = self.index_for_content_position(point(x, cursor.y), bounds);
        self.move_or_select_to(offset, selecting, cx);
        true
    }

    pub(super) fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let offset = offset.min(self.content.len());
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        self.marked_range = None;
        self.reset_caret_blink();
        self.ensure_cursor_visible();
        cx.notify();
    }

    pub(super) fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    pub(super) fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        let Some(bounds) = self.last_bounds else {
            return 0;
        };
        let content_position = point(
            (position.x - bounds.left()).clamp(px(0.0), bounds.size.width),
            (position.y - bounds.top() + self.scroll_y).max(px(0.0)),
        );
        self.index_for_content_position(content_position, bounds)
    }

    pub(super) fn index_for_content_position(
        &self,
        position: Point<Pixels>,
        bounds: Bounds<Pixels>,
    ) -> usize {
        if self.content.is_empty() {
            return 0;
        }
        if self.last_layout.is_empty() {
            return self.content.len();
        }
        if position.y < px(0.0) {
            return 0;
        }

        let mut line_top = px(0.0);
        for line in &self.last_layout {
            let line_height = line.line.size(self.style.line_height).height;
            let line_bottom = line_top + line_height;
            if position.y > line_bottom {
                line_top = line_bottom;
                continue;
            }
            let local_point = point(position.x, position.y - line_top);
            let local_index = line
                .line
                .index_for_position(local_point, self.style.line_height)
                .unwrap_or_else(|index| index)
                .min(line.end - line.start);
            return line.start + local_index;
        }
        if position.x <= bounds.size.width || self.last_layout.is_empty() {
            self.content.len()
        } else {
            self.last_layout
                .last()
                .map(|line| line.end)
                .unwrap_or(self.content.len())
        }
    }

    pub(super) fn content_position_for_index(&self, offset: usize) -> Option<Point<Pixels>> {
        let mut line_top = px(0.0);
        for line in &self.last_layout {
            let line_bottom = line_top + line.line.size(self.style.line_height).height;
            if offset >= line.start && offset <= line.end {
                let local = offset.saturating_sub(line.start).min(line.end - line.start);
                return line
                    .line
                    .position_for_index(local, self.style.line_height)
                    .map(|position| point(position.x, line_top + position.y));
            }
            line_top = line_bottom;
        }
        if offset == 0 {
            Some(point(px(0.0), px(0.0)))
        } else {
            None
        }
    }

    pub(super) fn viewport_position_for_index(
        &self,
        offset: usize,
        bounds: Bounds<Pixels>,
    ) -> Option<Point<Pixels>> {
        self.content_position_for_index(offset).map(|position| {
            point(
                bounds.left() + position.x,
                bounds.top() + position.y - self.scroll_y,
            )
        })
    }

    pub(super) fn line_for_offset(&self, offset: usize) -> Option<&EditorLayoutLine> {
        self.last_layout
            .iter()
            .find(|line| offset >= line.start && offset <= line.end)
            .or_else(|| {
                self.last_layout
                    .iter()
                    .rev()
                    .find(|line| offset >= line.start)
            })
            .or_else(|| self.last_layout.first())
    }

    pub(super) fn ensure_cursor_visible(&mut self) {
        let Some(position) = self.content_position_for_index(self.cursor_offset()) else {
            return;
        };
        if position.y < self.scroll_y {
            self.scroll_y = position.y;
        } else if position.y + self.style.line_height > self.scroll_y + self.viewport_height {
            self.scroll_y = position.y + self.style.line_height - self.viewport_height;
        }
        self.clamp_scroll();
    }

    pub(super) fn clamp_scroll(&mut self) {
        self.scroll_y = self.scroll_y.clamp(px(0.0), self.max_scroll_y());
    }

    pub(super) fn max_scroll_y(&self) -> Pixels {
        (self.content_height - self.viewport_height).max(px(0.0))
    }

    pub(super) fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;
        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }
        utf8_offset
    }

    pub(super) fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }
        utf16_offset
    }

    pub(super) fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    pub(super) fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    pub(super) fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    pub(super) fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }

    pub(super) fn previous_word_boundary(&self, offset: usize) -> usize {
        let mut previous_word_start = 0;
        let mut in_word = false;
        for (idx, ch) in self.content.char_indices() {
            if idx >= offset {
                break;
            }
            if ch.is_alphanumeric() || ch == '_' {
                if !in_word {
                    previous_word_start = idx;
                    in_word = true;
                }
            } else {
                in_word = false;
            }
        }
        previous_word_start
    }

    pub(super) fn next_word_boundary(&self, offset: usize) -> usize {
        let mut seen_word = false;
        for (idx, ch) in self.content.char_indices() {
            if idx <= offset {
                continue;
            }
            let word = ch.is_alphanumeric() || ch == '_';
            if seen_word && !word {
                return idx;
            }
            seen_word |= word;
        }
        self.content.len()
    }

    pub(super) fn snapshot(&self) -> EditorSnapshot {
        EditorSnapshot {
            content: self.content.clone(),
            selected_range: self.selected_range.clone(),
            selection_reversed: self.selection_reversed,
        }
    }

    pub(super) fn push_undo_snapshot(&mut self) {
        let snapshot = self.snapshot();
        if self
            .undo_stack
            .last()
            .is_some_and(|last| last.content == snapshot.content)
        {
            return;
        }
        self.undo_stack.push(snapshot);
        self.redo_stack.clear();
        const MAX_UNDO_SNAPSHOTS: usize = 100;
        if self.undo_stack.len() > MAX_UNDO_SNAPSHOTS {
            self.undo_stack.remove(0);
        }
    }

    pub(super) fn restore_snapshot(&mut self, snapshot: EditorSnapshot) {
        self.content = snapshot.content;
        self.selected_range = snapshot.selected_range;
        self.selection_reversed = snapshot.selection_reversed;
        self.marked_range = None;
        self.desired_x = None;
        self.last_layout.clear();
    }

    pub(super) fn emit_change(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(on_change) = self.on_change.clone() {
            on_change(self.content.to_string(), window, cx);
        }
    }

    pub(super) fn reset_caret_blink(&mut self) {
        self.caret_visible = true;
        self.blink_generation = self.blink_generation.wrapping_add(1);
        self.blink_scheduled = false;
    }

    pub(super) fn schedule_blink(&mut self, cx: &mut Context<Self>) {
        if self.blink_scheduled {
            return;
        }
        self.blink_scheduled = true;
        let generation = self.blink_generation;
        cx.spawn(async move |editor, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(CARET_BLINK_MS))
                .await;
            editor
                .update(cx, |editor, cx| {
                    if editor.blink_generation != generation {
                        return;
                    }
                    editor.caret_visible = !editor.caret_visible;
                    editor.blink_scheduled = false;
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }
}
