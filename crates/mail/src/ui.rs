mod parse;
mod render;
mod surface;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
mod types;

use base64::prelude::Engine as _;
use gpui::prelude::FluentBuilder;

pub use parse::{parse_mail_html, parse_plain_mail_body};
pub use render::{
    render_mail_document, render_mail_document_block, render_mail_document_clipped_marker,
    MailImageResolver, MailRenderOptions, MailTextFont, MailTextMeasure,
    MAIL_MESSAGE_CARD_MAX_WIDTH, MAIL_RICH_BODY_SIDE_PADDING, MAIL_RICH_BODY_WIDTH,
};
pub(crate) use surface::mail_palette;
pub use surface::{
    default_mail_selected_tab_id, default_mail_selected_thread_id, MailActionPaletteState,
    MailFooterAction, MailListThread, MailRemoteImages, MailRowAction,
    SurfaceInput, SurfaceRoot, SurfaceState,
};
pub use types::{
    MailBlock, MailContainer, MailDocument, MailEdgeInsets, MailFontWeight, MailImage,
    MailInlineStyle, MailParagraph, MailStyle, MailTable, MailTableCell, MailTableRow,
    MailTextAlign, MailTextRun,
};

use crate::model::{
    MailAddress, MailAttachment, MailContactAddress, MailContactHistoryPage,
    MailContactHistoryRequest, MailDraftRequest, MailIdentity, MailMessage, MailMessagePage,
    MailReadState, MailSearchPage, MailSearchQuery, MailSearchSnippet, MailSendResult,
    MailSplitDefinition, MailSplitDraft, MailSplitEnabled, MailSplitId, MailSplitMove,
    MailSplitMutation, MailStarState, MailThread, MailThreadTriageChange, MailThreadTriageRequest,
    MailThreadTriageResult, MailUploadFile, MailWorkspace, Mailbox,
};

pub(crate) use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
    time::Duration,
};

pub(crate) use app_model::if_light;
pub(crate) use app_model::{AppearanceMode, SurfaceTheme};
pub(crate) use gpui::{
    div, img, list, point, px, rgb, AnyElement, App, AppContext, BoxShadow, Context, Div, Entity,
    FocusHandle, Focusable, FontWeight, Image, ImageFormat, InteractiveElement, IntoElement,
    KeyDownEvent, ListAlignment, ListOffset, ListScrollEvent, ListSizingBehavior, ListState,
    MouseButton, MouseDownEvent, ParentElement, Render, RenderImage, ScrollHandle,
    StatefulInteractiveElement, Styled, Window,
};
pub(crate) use gpui_components::long_form_editor::{
    LongFormEditor, LongFormEditorAction, LongFormEditorChange, LongFormEditorProps,
    LongFormEditorStyle,
};
pub(crate) use gpui_components::text_input::{
    TextInput, TextInputAction, TextInputChange, TextInputMode, TextInputProps, TextInputStyle,
};
pub(crate) use gpui_components::{
    alpha, command_menu_empty_state, command_menu_panel_shell, command_menu_row_shell,
    command_menu_section_title, keystroke_input_text, render_svg_image,
    spawn_background_task_for_entity, spawn_timer_task_for_entity, svg_from_body,
};

mod support;
pub(crate) use support::*;
pub(crate) use surface::MailSurfaceStateConfig;
#[cfg(test)]
pub(crate) use surface::MailStartup;

const MAIL_PREVIEW_MIN_WIDTH: f32 = 360.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MailComposeMode {
    #[default]
    Closed,
    New,
    Reply,
    ReplyAll,
    Forward,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MailComposeField {
    To,
    Cc,
    Subject,
    Body,
}

pub(crate) struct MailComposeTextInputRequest {
    pub(crate) field: MailComposeField,
    pub(crate) value: String,
    pub(crate) placeholder: String,
    pub(crate) mode: TextInputMode,
    pub(crate) style: TextInputStyle,
}
