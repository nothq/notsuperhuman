use std::ops::Range;

use gpui::{
    fill, point, px, size, Bounds, Hsla, PaintQuad, Pixels, TextRun, UnderlineStyle, Window,
};

use super::{EditorLayoutLine, LongFormEditor};

pub(super) fn sanitize_text(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

pub(super) fn shaped_layout_lines(
    source: &str,
    font_size: Pixels,
    runs: Vec<TextRun>,
    wrap_width: Option<Pixels>,
    window: &mut Window,
) -> Vec<EditorLayoutLine> {
    let Some(lines) = window
        .text_system()
        .shape_text(
            source.to_string().into(),
            font_size,
            &runs,
            wrap_width,
            None,
        )
        .ok()
        .map(|lines| lines.into_vec())
    else {
        return Vec::new();
    };
    let mut start = 0usize;
    lines
        .into_iter()
        .map(|line| {
            let end = start + line.len();
            let layout_line = EditorLayoutLine { start, end, line };
            start = end.saturating_add(1);
            layout_line
        })
        .collect()
}

pub(super) fn layout_content_height(lines: &[EditorLayoutLine], line_height: Pixels) -> Pixels {
    lines
        .iter()
        .map(|line| line.line.size(line_height).height)
        .fold(px(0.0), |total, height| total + height)
}

pub(super) fn text_runs_for_editor(
    text: &str,
    color: Hsla,
    selected_range: Option<Range<usize>>,
    marked_range: Option<Range<usize>>,
    selection: Hsla,
) -> Vec<TextRun> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut cut_points = vec![0, text.len()];
    if let Some(range) = selected_range.as_ref() {
        cut_points.push(range.start.min(text.len()));
        cut_points.push(range.end.min(text.len()));
    }
    if let Some(range) = marked_range.as_ref() {
        cut_points.push(range.start.min(text.len()));
        cut_points.push(range.end.min(text.len()));
    }
    cut_points.sort_unstable();
    cut_points.dedup();

    cut_points
        .windows(2)
        .filter_map(|window| {
            let start = window[0];
            let end = window[1];
            if start == end {
                return None;
            }
            let selected = selected_range
                .as_ref()
                .is_some_and(|range| start < range.end && end > range.start && !range.is_empty());
            let marked = marked_range
                .as_ref()
                .is_some_and(|range| start < range.end && end > range.start && !range.is_empty());
            Some(TextRun {
                len: end - start,
                font: Default::default(),
                color,
                background_color: selected.then_some(selection),
                underline: marked.then_some(UnderlineStyle {
                    color: Some(color),
                    thickness: px(1.0),
                    wavy: false,
                }),
                strikethrough: None,
            })
        })
        .collect()
}

pub(super) fn cursor_quad(
    editor: &LongFormEditor,
    lines: &[EditorLayoutLine],
    bounds: Bounds<Pixels>,
) -> Option<PaintQuad> {
    let mut line_top = px(0.0);
    for line in lines {
        let line_height_px = line.line.size(editor.style.line_height).height;
        let offset = editor.cursor_offset();
        if offset >= line.start && offset <= line.end {
            let local = offset.saturating_sub(line.start).min(line.end - line.start);
            let position = line
                .line
                .position_for_index(local, editor.style.line_height)?;
            return Some(fill(
                Bounds::new(
                    point(
                        bounds.left() + position.x,
                        bounds.top() + line_top + position.y - editor.scroll_y,
                    ),
                    size(px(1.5), editor.style.line_height),
                ),
                editor.style.caret,
            ));
        }
        line_top += line_height_px;
    }
    None
}
