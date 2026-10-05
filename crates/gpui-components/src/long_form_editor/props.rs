use super::{LongFormEditorAction, LongFormEditorChange};
use gpui::{px, rgb, Hsla, Pixels, SharedString};

#[derive(Clone)]
pub struct LongFormEditorStyle {
    pub height: Pixels,
    pub min_height: Pixels,
    pub max_height: Option<Pixels>,
    pub padding_x: Pixels,
    pub padding_y: Pixels,
    pub radius: Pixels,
    pub background: Hsla,
    pub border: Hsla,
    pub focused_border: Hsla,
    pub text: Hsla,
    pub placeholder: Hsla,
    pub selection: Hsla,
    pub caret: Hsla,
    pub font_size: Pixels,
    pub line_height: Pixels,
    pub font_family: Option<SharedString>,
}

impl Default for LongFormEditorStyle {
    fn default() -> Self {
        Self {
            height: px(160.0),
            min_height: px(96.0),
            max_height: None,
            padding_x: px(0.0),
            padding_y: px(0.0),
            radius: px(0.0),
            background: Hsla::from(rgb(0x000000)).opacity(0.0),
            border: Hsla::from(rgb(0x000000)).opacity(0.0),
            focused_border: Hsla::from(rgb(0x000000)).opacity(0.0),
            text: rgb(0xf0efed).into(),
            placeholder: rgb(0x65645e).into(),
            selection: Hsla::from(rgb(0x78bced)).opacity(0.28),
            caret: rgb(0x78bced).into(),
            font_size: px(14.0),
            line_height: px(20.0),
            font_family: None,
        }
    }
}

#[derive(Clone)]
pub struct LongFormEditorProps {
    pub value: SharedString,
    pub placeholder: SharedString,
    pub style: LongFormEditorStyle,
    pub disabled: bool,
    pub request_focus: bool,
    pub tab_text: SharedString,
    pub on_change: Option<LongFormEditorChange>,
    pub on_submit: Option<LongFormEditorAction>,
    pub on_escape: Option<LongFormEditorAction>,
    pub on_tab: Option<LongFormEditorAction>,
    pub on_focus: Option<LongFormEditorAction>,
}

impl LongFormEditorProps {
    pub fn new(value: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            placeholder: SharedString::default(),
            style: LongFormEditorStyle::default(),
            disabled: false,
            request_focus: false,
            tab_text: "    ".into(),
            on_change: None,
            on_submit: None,
            on_escape: None,
            on_tab: None,
            on_focus: None,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn style(mut self, style: LongFormEditorStyle) -> Self {
        self.style = style;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn request_focus(mut self, request_focus: bool) -> Self {
        self.request_focus = request_focus;
        self
    }

    pub fn tab_text(mut self, tab_text: impl Into<SharedString>) -> Self {
        self.tab_text = tab_text.into();
        self
    }

    pub fn on_change(mut self, on_change: LongFormEditorChange) -> Self {
        self.on_change = Some(on_change);
        self
    }

    pub fn on_submit(mut self, on_submit: LongFormEditorAction) -> Self {
        self.on_submit = Some(on_submit);
        self
    }

    pub fn on_escape(mut self, on_escape: LongFormEditorAction) -> Self {
        self.on_escape = Some(on_escape);
        self
    }

    pub fn on_tab(mut self, on_tab: LongFormEditorAction) -> Self {
        self.on_tab = Some(on_tab);
        self
    }

    pub fn on_focus(mut self, on_focus: LongFormEditorAction) -> Self {
        self.on_focus = Some(on_focus);
        self
    }
}
