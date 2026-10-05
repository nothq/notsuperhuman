use gpui::{
    div, fill, point, px, relative, size, App, Bounds, Context, CursorStyle, Element, ElementId,
    ElementInputHandler, Entity, FocusHandle, Focusable, GlobalElementId, InteractiveElement,
    IntoElement, LayoutId, MouseButton, PaintQuad, ParentElement, Pixels, Render, Style, Styled,
    TextAlign, Window,
};

use super::{
    layout::{cursor_quad, layout_content_height, shaped_layout_lines, text_runs_for_editor},
    EditorLayoutLine, LongFormEditor,
};

impl Render for LongFormEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus_handle.is_focused(window);
        if self.request_focus && !focused && !self.disabled {
            window.focus(&self.focus_handle, cx);
        }
        let focused = self.focus_handle.is_focused(window);
        self.focused_last_render = focused;
        if focused {
            self.schedule_blink(cx);
        } else {
            self.caret_visible = true;
            self.blink_scheduled = false;
        }

        let style = self.style.clone();
        let border = if focused {
            style.focused_border
        } else {
            style.border
        };
        let content_height = self.content_height + style.padding_y * 2.0;
        let mut height = style.height.max(style.min_height).max(content_height);
        if let Some(max_height) = style.max_height {
            height = height.min(max_height);
        }
        let mut root = div()
            .h(height)
            .w_full()
            .min_w(px(0.0))
            .rounded(style.radius)
            .border_1()
            .border_color(border)
            .bg(style.background)
            .px(style.padding_x)
            .py(style.padding_y)
            .flex()
            .items_start()
            .overflow_hidden()
            .text_size(style.font_size)
            .line_height(style.line_height)
            .text_color(style.text)
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_key_down(cx.listener(Self::on_key_down))
            .on_scroll_wheel(cx.listener(Self::on_scroll_wheel))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move));
        if let Some(font_family) = style.font_family.as_ref() {
            root = root.font_family(font_family.clone());
        }
        root.child(EditorTextElement {
            editor: cx.entity(),
        })
    }
}

impl Focusable for LongFormEditor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

struct EditorTextElement {
    editor: Entity<LongFormEditor>,
}

struct EditorPrepaintState {
    lines: Vec<EditorLayoutLine>,
    cursor: Option<PaintQuad>,
    content_height: Pixels,
    viewport_height: Pixels,
    render_placeholder: bool,
}

impl IntoElement for EditorTextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for EditorTextElement {
    type RequestLayoutState = ();
    type PrepaintState = EditorPrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let editor = self.editor.read(cx);
        let render_placeholder = editor.content.is_empty() && !editor.placeholder.is_empty();
        let source = editor_prepaint_source(editor, render_placeholder);
        let lines = editor_prepaint_lines(editor, source, render_placeholder, bounds, window);
        let content_height =
            layout_content_height(&lines, editor.style.line_height).max(editor.style.line_height);
        let cursor = editor_prepaint_cursor(editor, &lines, bounds);
        EditorPrepaintState {
            lines,
            cursor,
            content_height,
            viewport_height: bounds.size.height,
            render_placeholder,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.editor.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.editor.clone()),
            cx,
        );

        let (line_height, scroll_y, caret_visible) = {
            let editor = self.editor.read(cx);
            (
                editor.style.line_height,
                editor.scroll_y,
                editor.caret_visible,
            )
        };

        paint_editor_lines(
            EditorLinePaintRequest {
                lines: &prepaint.lines,
                render_placeholder: prepaint.render_placeholder,
                bounds,
                line_height,
                scroll_y,
            },
            window,
            cx,
        );

        let should_paint_cursor = caret_visible && focus_handle.is_focused(window);
        if should_paint_cursor {
            if let Some(cursor) = prepaint.cursor.take() {
                window.paint_quad(cursor);
            }
        }

        sync_editor_prepaint_state(&self.editor, prepaint, bounds, cx);
    }
}

struct EditorLinePaintRequest<'a> {
    lines: &'a [EditorLayoutLine],
    render_placeholder: bool,
    bounds: Bounds<Pixels>,
    line_height: Pixels,
    scroll_y: Pixels,
}

fn editor_prepaint_source(editor: &LongFormEditor, render_placeholder: bool) -> &str {
    if render_placeholder {
        editor.placeholder.as_ref()
    } else {
        editor.content.as_ref()
    }
}

fn editor_prepaint_lines(
    editor: &LongFormEditor,
    source: &str,
    render_placeholder: bool,
    bounds: Bounds<Pixels>,
    window: &mut Window,
) -> Vec<EditorLayoutLine> {
    if source.is_empty() {
        return Vec::new();
    }
    let text_color = if render_placeholder {
        editor.style.placeholder
    } else {
        editor.style.text
    };
    let runs = text_runs_for_editor(
        source,
        text_color,
        (!render_placeholder).then_some(editor.selected_range.clone()),
        (!render_placeholder)
            .then(|| editor.marked_range.clone())
            .flatten(),
        editor.style.selection,
    );
    shaped_layout_lines(
        source,
        editor.style.font_size,
        runs,
        Some(bounds.size.width.max(px(0.0))),
        window,
    )
}

fn editor_prepaint_cursor(
    editor: &LongFormEditor,
    lines: &[EditorLayoutLine],
    bounds: Bounds<Pixels>,
) -> Option<PaintQuad> {
    if !editor.selected_range.is_empty() {
        return None;
    }
    if editor.content.is_empty() {
        return Some(fill(
            Bounds::new(
                point(bounds.left(), bounds.top() - editor.scroll_y),
                size(px(1.5), editor.style.line_height),
            ),
            editor.style.caret,
        ));
    }
    cursor_quad(editor, lines, bounds)
}

fn paint_editor_lines(request: EditorLinePaintRequest<'_>, window: &mut Window, cx: &mut App) {
    let mut line_origin = point(
        request.bounds.left(),
        request.bounds.top() - request.scroll_y,
    );
    for line in request.lines {
        let line_height_px = line.line.size(request.line_height).height;
        let line_bottom = line_origin.y + line_height_px;
        if line_bottom >= request.bounds.top() && line_origin.y <= request.bounds.bottom() {
            if !request.render_placeholder {
                let _ = line.line.paint_background(
                    line_origin,
                    request.line_height,
                    TextAlign::Left,
                    Some(request.bounds),
                    window,
                    cx,
                );
            }
            let _ = line.line.paint(
                line_origin,
                request.line_height,
                TextAlign::Left,
                Some(request.bounds),
                window,
                cx,
            );
        }
        line_origin.y += line_height_px;
    }
}

fn sync_editor_prepaint_state(
    editor_entity: &Entity<LongFormEditor>,
    prepaint: &mut EditorPrepaintState,
    bounds: Bounds<Pixels>,
    cx: &mut App,
) {
    let lines = std::mem::take(&mut prepaint.lines);
    editor_entity.update(cx, |editor, _cx| {
        editor.last_layout = lines;
        editor.last_bounds = Some(bounds);
        editor.content_height = prepaint.content_height;
        editor.viewport_height = prepaint.viewport_height;
        editor.clamp_scroll();
    });
}
