use super::text_style::{
    MailDirection, MailFontFamily, MailFontStyle, MailFontWeight, MailInlineStyle, MailLineHeight,
    MailTextAlign, MailTextDecoration, MailTextTransform, MailWordBreak,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MailDisplay {
    #[default]
    Unspecified,
    None,
    Inline,
    InlineBlock,
    Block,
    Table,
    TableCell,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MailVerticalAlign {
    Top,
    Middle,
    Bottom,
    #[default]
    Baseline,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MailBackgroundSize {
    #[default]
    Auto,
    Contain,
    Cover,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MailBackgroundPosition {
    #[default]
    Initial,
    Center,
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MailBackgroundRepeat {
    #[default]
    Repeat,
    NoRepeat,
    RepeatX,
    RepeatY,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MailFloat {
    #[default]
    None,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MailBorderCollapse {
    #[default]
    Separate,
    Collapse,
}

/// Which of CSS's two table layout algorithms sizes a table's columns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MailTableLayout {
    #[default]
    Auto,
    Fixed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MailBoxSizing {
    #[default]
    ContentBox,
    BorderBox,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MailOverflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
    Auto,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MailPosition {
    Absolute,
    Relative,
    #[default]
    Static,
    Fixed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MailEdgeInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl MailEdgeInsets {
    /// The two horizontal edges combined.
    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    pub fn any(self) -> bool {
        self.top > 0.0 || self.right > 0.0 || self.bottom > 0.0 || self.left > 0.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MailBorderSide {
    pub color: Option<u32>,
    pub width: Option<f32>,
}

impl MailBorderSide {
    pub fn paints(self) -> bool {
        self.color.is_some() || self.width.unwrap_or_default() > 0.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MailBorderEdges {
    pub top: MailBorderSide,
    pub right: MailBorderSide,
    pub bottom: MailBorderSide,
    pub left: MailBorderSide,
}

impl MailBorderEdges {
    pub fn any(self) -> bool {
        self.top.paints() || self.right.paints() || self.bottom.paints() || self.left.paints()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MailStyle {
    pub color: Option<u32>,
    pub background_color: Option<u32>,
    pub background_image_url: Option<String>,
    pub background_size: MailBackgroundSize,
    pub background_position: MailBackgroundPosition,
    pub background_repeat: MailBackgroundRepeat,
    pub font_family: Option<MailFontFamily>,
    pub font_style: MailFontStyle,
    pub font_size: Option<f32>,
    pub font_weight: Option<MailFontWeight>,
    pub line_height: Option<MailLineHeight>,
    pub text_align: MailTextAlign,
    /// The `text-align` of the line this box sits in — its parent's. An
    /// inline-level box is placed by it, while its own `text-align` only
    /// aligns what is inside it.
    pub line_align: MailTextAlign,
    pub text_align_from_table_cell_attr: bool,
    pub text_decoration: MailTextDecoration,
    pub text_transform: MailTextTransform,
    pub word_break: MailWordBreak,
    /// `letter-spacing`: extra advance after every character. Inherited.
    pub letter_spacing: Option<f32>,
    /// `white-space` forbids wrapping (`nowrap`, `pre`); inherited like other text.
    pub nowrap: bool,
    pub direction: MailDirection,
    pub preserve_whitespace: bool,
    pub padding: MailEdgeInsets,
    pub margin: MailEdgeInsets,
    pub border_color: Option<u32>,
    pub border_width: Option<f32>,
    pub border_edges: MailBorderEdges,
    pub border_radius: Option<f32>,
    pub border_collapse: MailBorderCollapse,
    pub table_layout: MailTableLayout,
    pub box_sizing: MailBoxSizing,
    pub width: Option<f32>,
    pub width_percent: Option<f32>,
    pub min_width: Option<f32>,
    pub max_width: Option<f32>,
    pub max_width_percent: Option<f32>,
    pub height: Option<f32>,
    pub max_height: Option<f32>,
    pub margin_left_auto: bool,
    pub margin_right_auto: bool,
    pub overflow: MailOverflow,
    pub clip: Option<String>,
    pub position: MailPosition,
    pub border_spacing: Option<MailTableSpacing>,
    pub display: MailDisplay,
    pub vertical_align: MailVerticalAlign,
    pub float: MailFloat,
    pub display_none: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MailTableSpacing {
    pub horizontal: f32,
    pub vertical: f32,
}

impl MailStyle {
    pub fn html_initial() -> Self {
        Self {
            font_size: Some(16.0),
            ..Default::default()
        }
    }

    pub fn inherit_text(parent: &Self) -> Self {
        Self {
            color: parent.color,
            font_family: parent.font_family,
            font_style: parent.font_style,
            font_size: parent.font_size,
            font_weight: parent.font_weight,
            line_height: parent
                .line_height
                .map(|line_height| line_height.inherited(parent.font_size.unwrap_or(16.0))),
            text_align: parent.text_align,
            text_align_from_table_cell_attr: parent.text_align_from_table_cell_attr,
            text_decoration: parent.text_decoration,
            text_transform: parent.text_transform,
            word_break: parent.word_break,
            letter_spacing: parent.letter_spacing,
            nowrap: parent.nowrap,
            direction: parent.direction,
            preserve_whitespace: parent.preserve_whitespace,
            ..Default::default()
        }
    }

    pub fn inline_style(&self) -> MailInlineStyle {
        MailInlineStyle {
            color: self.color,
            font_family: self.font_family,
            font_weight: self.font_weight,
            italic: self.font_style == MailFontStyle::Italic,
            font_size: self.font_size,
            line_height: self.line_height,
            text_decoration: self.text_decoration,
            text_transform: self.text_transform,
            preserve_whitespace: self.preserve_whitespace,
            letter_spacing: self.letter_spacing,
        }
    }

    /// Whether this box participates in an inline formatting context. CSS makes a
    /// container's children lay out in a row only when every one of them is
    /// inline-level; anything block-level forces block layout instead.
    pub fn is_inline_level(&self) -> bool {
        matches!(self.display, MailDisplay::Inline | MailDisplay::InlineBlock)
    }

    pub fn has_box_style(&self) -> bool {
        self.background_color.is_some()
            || self.background_image_url.is_some()
            || self.padding.any()
            || self.margin.any()
            || self.border_color.is_some()
            || self.border_width.unwrap_or_default() > 0.0
            || self.border_edges.any()
            || self.border_radius.unwrap_or_default() > 0.0
            || self.width.is_some()
            || self.width_percent.is_some()
            || self.min_width.is_some()
            || self.max_width.is_some()
            || self.max_width_percent.is_some()
            || self.height.is_some()
            || self.max_height.is_some()
            || self.margin_left_auto
            || self.margin_right_auto
            // Every element now resolves a used display, so only a display that
            // actually changes how the box behaves marks it as styled; plain
            // block or inline flow does not.
            || !matches!(
                self.display,
                MailDisplay::Unspecified | MailDisplay::Block | MailDisplay::Inline
            )
            || self.vertical_align != MailVerticalAlign::Baseline
            || self.direction == MailDirection::Rtl
            || self.float != MailFloat::None
            || self.overflow != MailOverflow::Visible
            || self.position != MailPosition::Static
    }
}
