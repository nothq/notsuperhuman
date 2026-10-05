use super::{LongFormEditor, DEFAULT_LINE_SCROLL_PX};
use gpui::{
    point, px, ClipboardItem, Context, EntityInputHandler, KeyDownEvent, Modifiers, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, ScrollDelta, ScrollWheelEvent, Window,
};

impl LongFormEditor {
    pub(super) fn on_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled || !self.focus_handle.is_focused(window) {
            return;
        }
        if self.handle_key_down(event, window, cx) {
            cx.stop_propagation();
        }
    }

    pub(super) fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let modifiers = event.keystroke.modifiers;
        let key = event.keystroke.key.as_str();
        if modifiers.platform {
            return self.handle_platform_key(key, modifiers, window, cx);
        }
        if modifiers.control || modifiers.function {
            return false;
        }
        if modifiers.alt {
            return self.handle_alt_key(key, modifiers, cx);
        }
        self.handle_editing_key(key, modifiers, window, cx)
    }

    pub(super) fn handle_platform_key(
        &mut self,
        key: &str,
        modifiers: Modifiers,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        match key {
            "a" => {
                self.select_all(cx);
                true
            }
            "c" => {
                self.copy(cx);
                true
            }
            "x" => {
                self.cut(window, cx);
                true
            }
            "v" => {
                self.paste(window, cx);
                true
            }
            "z" if modifiers.shift => self.redo(window, cx),
            "z" => self.undo(window, cx),
            "enter" => {
                if let Some(on_submit) = self.on_submit.clone() {
                    on_submit(window, cx);
                }
                true
            }
            "left" => self.move_or_select_to_visual_line_edge(true, modifiers.shift, cx),
            "right" => self.move_or_select_to_visual_line_edge(false, modifiers.shift, cx),
            "up" => self.move_or_select_to_document_edge(true, modifiers.shift, cx),
            "down" => self.move_or_select_to_document_edge(false, modifiers.shift, cx),
            _ => false,
        }
    }

    pub(super) fn handle_alt_key(
        &mut self,
        key: &str,
        modifiers: Modifiers,
        cx: &mut Context<Self>,
    ) -> bool {
        match key {
            "left" => {
                let offset = self.previous_word_boundary(self.cursor_offset());
                self.move_or_select_to(offset, modifiers.shift, cx);
                true
            }
            "right" => {
                let offset = self.next_word_boundary(self.cursor_offset());
                self.move_or_select_to(offset, modifiers.shift, cx);
                true
            }
            _ => false,
        }
    }

    pub(super) fn handle_editing_key(
        &mut self,
        key: &str,
        modifiers: Modifiers,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        match key {
            "backspace" => self.handle_backspace(window, cx),
            "delete" => self.handle_delete(window, cx),
            "left" => self.handle_left(modifiers.shift, cx),
            "right" => self.handle_right(modifiers.shift, cx),
            "up" => self.handle_vertical(-1, modifiers.shift, cx),
            "down" => self.handle_vertical(1, modifiers.shift, cx),
            "pageup" => self.handle_page_vertical(-1, modifiers.shift, cx),
            "pagedown" => self.handle_page_vertical(1, modifiers.shift, cx),
            "home" => self.move_or_select_to_visual_line_edge(true, modifiers.shift, cx),
            "end" => self.move_or_select_to_visual_line_edge(false, modifiers.shift, cx),
            "escape" => self.handle_escape(window, cx),
            "enter" => self.handle_enter(modifiers, window, cx),
            "tab" => self.handle_tab(window, cx),
            _ => false,
        }
    }

    pub(super) fn handle_backspace(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
        true
    }

    pub(super) fn handle_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
        true
    }

    pub(super) fn handle_left(&mut self, selecting: bool, cx: &mut Context<Self>) -> bool {
        self.desired_x = None;
        if selecting {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx);
        } else if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx);
        }
        true
    }

    pub(super) fn handle_right(&mut self, selecting: bool, cx: &mut Context<Self>) -> bool {
        self.desired_x = None;
        if selecting {
            self.select_to(self.next_boundary(self.cursor_offset()), cx);
        } else if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx);
        }
        true
    }

    pub(super) fn handle_vertical(
        &mut self,
        direction: i32,
        selecting: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(bounds) = self.last_bounds else {
            return true;
        };
        let Some(cursor) = self.content_position_for_index(self.cursor_offset()) else {
            return true;
        };
        let x = self.desired_x.unwrap_or(cursor.x);
        self.desired_x = Some(x);
        let next_y = cursor.y + self.style.line_height * direction as f32;
        let offset = self.index_for_content_position(point(x, next_y), bounds);
        self.move_or_select_to(offset, selecting, cx);
        true
    }

    pub(super) fn handle_page_vertical(
        &mut self,
        direction: i32,
        selecting: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(bounds) = self.last_bounds else {
            return true;
        };
        let Some(cursor) = self.content_position_for_index(self.cursor_offset()) else {
            return true;
        };
        let x = self.desired_x.unwrap_or(cursor.x);
        self.desired_x = Some(x);
        let page_delta =
            (self.viewport_height - self.style.line_height).max(self.style.line_height);
        let next_y = cursor.y + page_delta * direction as f32;
        let offset = self.index_for_content_position(point(x, next_y), bounds);
        self.move_or_select_to(offset, selecting, cx);
        true
    }

    pub(super) fn handle_escape(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if let Some(on_escape) = self.on_escape.clone() {
            on_escape(window, cx);
            return true;
        }
        false
    }

    pub(super) fn handle_enter(
        &mut self,
        modifiers: Modifiers,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if let Some(on_submit) = self.on_submit.clone() {
            if modifiers.platform {
                on_submit(window, cx);
                return true;
            }
        }
        self.replace_text_in_range(None, "\n", window, cx);
        true
    }

    pub(super) fn handle_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if let Some(on_tab) = self.on_tab.clone() {
            on_tab(window, cx);
            return true;
        }
        let tab_text = self.tab_text.to_string();
        self.replace_text_in_range(None, &tab_text, window, cx);
        true
    }

    pub(super) fn on_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pixel_delta = match event.delta {
            ScrollDelta::Pixels(delta) => delta,
            ScrollDelta::Lines(delta) => point(
                px(delta.x * DEFAULT_LINE_SCROLL_PX),
                px(delta.y * DEFAULT_LINE_SCROLL_PX),
            ),
        };
        if pixel_delta.y == Pixels::ZERO {
            return;
        }
        let previous = self.scroll_y;
        self.scroll_y = (self.scroll_y - pixel_delta.y).clamp(px(0.0), self.max_scroll_y());
        if self.scroll_y != previous {
            cx.stop_propagation();
            cx.notify();
        }
    }

    pub(super) fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        window.focus(&self.focus_handle, cx);
        if let Some(on_focus) = self.on_focus.clone() {
            on_focus(window, cx);
        }
        window.prevent_default();
        cx.stop_propagation();
        self.is_selecting = true;
        self.desired_x = None;
        if event.modifiers.shift {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        } else {
            self.move_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    pub(super) fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.is_selecting = false;
        cx.stop_propagation();
    }

    pub(super) fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
            cx.stop_propagation();
        }
    }

    pub(super) fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, text.as_ref(), window, cx);
        }
    }

    pub(super) fn copy(&self, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }

    pub(super) fn cut(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            self.copy(cx);
            self.replace_text_in_range(None, "", window, cx);
        }
    }

    pub(super) fn undo(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(snapshot) = self.undo_stack.pop() else {
            return true;
        };
        self.redo_stack.push(self.snapshot());
        self.restore_snapshot(snapshot);
        self.reset_caret_blink();
        self.ensure_cursor_visible();
        self.emit_change(window, cx);
        cx.notify();
        true
    }

    pub(super) fn redo(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(snapshot) = self.redo_stack.pop() else {
            return true;
        };
        self.undo_stack.push(self.snapshot());
        self.restore_snapshot(snapshot);
        self.reset_caret_blink();
        self.ensure_cursor_visible();
        self.emit_change(window, cx);
        cx.notify();
        true
    }
}
