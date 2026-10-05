mod element;
mod input;
mod interaction;
mod layout;
mod props;
mod selection;

use std::{ops::Range, rc::Rc};

use gpui::{px, Bounds, Context, FocusHandle, Pixels, SharedString, Window, WrappedLine};

use self::layout::sanitize_text;
pub use self::props::{LongFormEditorProps, LongFormEditorStyle};

pub(super) const CARET_BLINK_MS: u64 = 530;
pub(super) const DEFAULT_LINE_SCROLL_PX: f32 = 24.0;

pub type LongFormEditorChange = Rc<dyn Fn(String, &mut Window, &mut Context<LongFormEditor>)>;
pub type LongFormEditorAction = Rc<dyn Fn(&mut Window, &mut Context<LongFormEditor>)>;

#[derive(Clone)]
pub(super) struct EditorSnapshot {
    content: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
}

pub(super) struct EditorLayoutLine {
    start: usize,
    end: usize,
    line: WrappedLine,
}

pub struct LongFormEditor {
    focus_handle: FocusHandle,
    content: SharedString,
    placeholder: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Vec<EditorLayoutLine>,
    last_bounds: Option<Bounds<Pixels>>,
    scroll_y: Pixels,
    content_height: Pixels,
    viewport_height: Pixels,
    desired_x: Option<Pixels>,
    is_selecting: bool,
    style: LongFormEditorStyle,
    disabled: bool,
    request_focus: bool,
    tab_text: SharedString,
    on_change: Option<LongFormEditorChange>,
    on_submit: Option<LongFormEditorAction>,
    on_escape: Option<LongFormEditorAction>,
    on_tab: Option<LongFormEditorAction>,
    on_focus: Option<LongFormEditorAction>,
    undo_stack: Vec<EditorSnapshot>,
    redo_stack: Vec<EditorSnapshot>,
    caret_visible: bool,
    blink_scheduled: bool,
    blink_generation: u64,
    focused_last_render: bool,
}

impl LongFormEditor {
    pub fn new(props: LongFormEditorProps, cx: &mut Context<Self>) -> Self {
        let content: SharedString = sanitize_text(props.value.as_ref()).into();
        let offset = content.len();
        Self {
            focus_handle: cx.focus_handle(),
            content,
            placeholder: props.placeholder,
            selected_range: offset..offset,
            selection_reversed: false,
            marked_range: None,
            last_layout: Vec::new(),
            last_bounds: None,
            scroll_y: px(0.0),
            content_height: px(0.0),
            viewport_height: props.style.height.max(props.style.min_height),
            desired_x: None,
            is_selecting: false,
            style: props.style,
            disabled: props.disabled,
            request_focus: props.request_focus,
            tab_text: props.tab_text,
            on_change: props.on_change,
            on_submit: props.on_submit,
            on_escape: props.on_escape,
            on_tab: props.on_tab,
            on_focus: props.on_focus,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            caret_visible: true,
            blink_scheduled: false,
            blink_generation: 0,
            focused_last_render: false,
        }
    }

    pub fn apply_props(&mut self, props: LongFormEditorProps, cx: &mut Context<Self>) {
        self.placeholder = props.placeholder;
        self.style = props.style;
        self.disabled = props.disabled;
        self.request_focus = props.request_focus;
        self.tab_text = props.tab_text;
        self.on_change = props.on_change;
        self.on_submit = props.on_submit;
        self.on_escape = props.on_escape;
        self.on_tab = props.on_tab;
        self.on_focus = props.on_focus;
        self.set_text(props.value, cx);
    }

    pub fn set_text(&mut self, value: impl Into<SharedString>, cx: &mut Context<Self>) {
        let value = sanitize_text(value.into().as_ref());
        if self.content.as_ref() == value {
            return;
        }
        self.content = value.into();
        let cursor = self.cursor_offset().min(self.content.len());
        self.selected_range = cursor..cursor;
        self.selection_reversed = false;
        self.marked_range = None;
        self.last_layout.clear();
        self.desired_x = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.reset_caret_blink();
        self.clamp_scroll();
        cx.notify();
    }

    pub fn text(&self) -> &str {
        self.content.as_ref()
    }

    pub fn focus_handle_clone(&self) -> FocusHandle {
        self.focus_handle.clone()
    }

    pub fn select_all(&mut self, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx);
    }
}
