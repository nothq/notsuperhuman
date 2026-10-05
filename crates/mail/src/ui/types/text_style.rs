//! Text-level style: fonts, line height, alignment and decoration.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MailInlineStyle {
    pub color: Option<u32>,
    pub font_family: Option<MailFontFamily>,
    pub font_weight: Option<MailFontWeight>,
    pub italic: bool,
    pub font_size: Option<f32>,
    pub line_height: Option<MailLineHeight>,
    pub text_decoration: MailTextDecoration,
    pub text_transform: MailTextTransform,
    pub preserve_whitespace: bool,
    /// Extra advance after every character, from `letter-spacing`.
    pub letter_spacing: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MailLineHeight {
    Pixels(f32),
    Number(f32),
    Percentage(f32),
}

impl MailLineHeight {
    pub fn resolve(self, font_size: f32) -> f32 {
        match self {
            Self::Pixels(value) => value,
            Self::Number(value) | Self::Percentage(value) => value * font_size,
        }
    }

    pub(crate) fn inherited(self, parent_font_size: f32) -> Self {
        match self {
            Self::Percentage(value) => Self::Pixels(value * parent_font_size),
            Self::Pixels(_) | Self::Number(_) => self,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailFontWeight {
    Normal,
    Medium,
    Bold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailFontFamily {
    Arial,
    Helvetica,
    /// The face macOS ships with a medium weight, which `Helvetica` lacks.
    HelveticaNeue,
    /// The web-safe faces macOS ships, which email stacks name first. Each
    /// is far enough from Helvetica in width that substituting it rewraps
    /// every paragraph: Verdana sets a line 16% wider.
    Verdana,
    Geneva,
    Tahoma,
    TrebuchetMs,
    Georgia,
    LucidaGrande,
    CourierNew,
    SansSerif,
    Serif,
    Monospace,
    Other,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MailTextAlign {
    #[default]
    Start,
    End,
    Left,
    Center,
    Right,
}

impl MailTextAlign {
    pub(crate) fn resolve(self, direction: MailDirection) -> Self {
        match (self, direction) {
            (Self::Start, MailDirection::Ltr) | (Self::End, MailDirection::Rtl) => Self::Left,
            (Self::Start, MailDirection::Rtl) | (Self::End, MailDirection::Ltr) => Self::Right,
            (resolved, _) => resolved,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MailDirection {
    #[default]
    Ltr,
    Rtl,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MailFontStyle {
    #[default]
    Normal,
    Italic,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MailTextDecoration {
    pub underline: bool,
    pub line_through: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MailTextTransform {
    #[default]
    None,
    Uppercase,
    Lowercase,
    Capitalize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MailWordBreak {
    #[default]
    Normal,
    BreakWord,
    BreakAll,
}
