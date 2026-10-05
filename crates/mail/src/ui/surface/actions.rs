use super::AppearanceMode;

pub(crate) const MAIL_ACTIVITY_MIN_WIDTH: f32 = 268.0;
pub(crate) const MAIL_ACTIVITY_MAX_WIDTH: f32 = 360.0;
pub(crate) const MAIL_APPBAR_WIDTH: f32 = 40.0;
pub(crate) const MAIL_FONT_FAMILY: &str = "Lato";
pub(crate) const MAIL_OPEN_COMPOSER_HEIGHT: f32 = 74.0;
pub(crate) const MAIL_OPEN_HEADER_HEIGHT: f32 = 136.0;
pub(crate) const MAIL_ROW_HEIGHT: f32 = 36.0;
pub(crate) const MAIL_SEARCH_FONT_FAMILY: &str = "Lato";
pub(crate) const MAIL_SEARCH_TOP_INSET: f32 = 40.0;
pub(crate) const MAIL_SECTION_LABEL_LEFT: f32 = 52.5;
pub(crate) const MAIL_SECTION_LABEL_RIGHT: f32 = 30.0;
pub(crate) const MAIL_TABBAR_HEIGHT: f32 = 76.0;
pub(crate) const MAIL_WINDOW_CHROME_CONTENT_ADJUSTMENT: f32 = 1.0;

#[derive(Clone, Copy)]
pub(crate) struct MailPalette {
    pub surface_bg: u32,
    pub shell_bg: u32,
    pub activity_bg: u32,
    pub activity_border: u32,
    pub viewer_bg: u32,
    pub viewer_border: u32,
    pub message_card_bg: u32,
    pub message_card_border: u32,
    pub composer_bg: u32,
    pub compose_panel_bg: u32,
    pub compose_panel_divider: u32,
    pub compose_caret: u32,
    pub compose_suggestion_bg: u32,
    pub compose_suggestion_selected_bg: u32,
    pub compose_suggestion_border: u32,
    pub compose_suggestion_match: u32,
    pub contact_avatar_bg: u32,
    pub selected_row_bg: u32,
    pub hover_row_bg: u32,
    pub selected_row_accent: u32,
    pub unread_dot: u32,
    pub tooltip_bg: u32,
    pub tooltip_label: u32,
    pub text_rgb: u32,
    pub icon_rgb: u32,
}

pub(crate) const fn mail_palette(appearance_mode: AppearanceMode) -> MailPalette {
    match appearance_mode {
        AppearanceMode::Light => light_mail_palette(),
        AppearanceMode::Dark => dark_mail_palette(),
    }
}

const fn light_mail_palette() -> MailPalette {
    MailPalette {
        surface_bg: 0xffffff,
        shell_bg: 0xffffff,
        activity_bg: 0xfbfcff,
        activity_border: 0xe8edf3,
        viewer_bg: 0xffffff,
        viewer_border: 0xebeff5,
        message_card_bg: 0xffffff,
        message_card_border: 0xc3bff3,
        composer_bg: 0xffffff,
        compose_panel_bg: 0xf4f5fa,
        compose_panel_divider: 0xdce2ea,
        compose_caret: 0x78bced,
        compose_suggestion_bg: 0xffffff,
        compose_suggestion_selected_bg: 0xf0f4fa,
        compose_suggestion_border: 0xd7dde6,
        compose_suggestion_match: 0x17181b,
        contact_avatar_bg: 0xe7ecf3,
        selected_row_bg: 0xf4f5fa,
        hover_row_bg: 0xf7f9fc,
        selected_row_accent: 0xb5b2f0,
        unread_dot: 0x78bced,
        tooltip_bg: 0x45494e,
        tooltip_label: 0xebeef2,
        text_rgb: 0x000000,
        icon_rgb: 0x000000,
    }
}

const fn dark_mail_palette() -> MailPalette {
    MailPalette {
        surface_bg: 0x27292d,
        shell_bg: 0x27292d,
        activity_bg: 0x18191a,
        activity_border: 0x161718,
        viewer_bg: 0x27292d,
        viewer_border: 0x32353a,
        message_card_bg: 0x363a40,
        message_card_border: 0xc2bdf4,
        composer_bg: 0x26282c,
        compose_panel_bg: 0x363a40,
        compose_panel_divider: 0x5a5f68,
        compose_caret: 0x6fbee9,
        compose_suggestion_bg: 0x292c31,
        compose_suggestion_selected_bg: 0x4b515a,
        compose_suggestion_border: 0x23262b,
        compose_suggestion_match: 0xffffff,
        contact_avatar_bg: 0x3a3d43,
        selected_row_bg: 0x464b53,
        hover_row_bg: 0x3b4048,
        selected_row_accent: 0xb5b2f0,
        unread_dot: 0x78bced,
        tooltip_bg: 0x45494e,
        tooltip_label: 0xebeef2,
        text_rgb: 0xffffff,
        icon_rgb: 0xffffff,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailRowAction {
    MarkDone,
    RemindMe,
    Move,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum MailTriageControl {
    Read,
    Star,
}

/// The thread state a triage button reflects.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MailTriageFlags {
    pub(crate) unread: bool,
    pub(crate) starred: bool,
}

impl MailTriageControl {
    pub(crate) const fn ui_segment(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Star => "star",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MailMessageAction {
    Reply,
    Forward,
}

impl MailMessageAction {
    pub(crate) const fn ui_segment(self) -> &'static str {
        match self {
            Self::Reply => "reply",
            Self::Forward => "forward",
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Reply => "Reply",
            Self::Forward => "Forward",
        }
    }

    pub(crate) const fn pop_out_label(self) -> &'static str {
        match self {
            Self::Reply => "Reply, Pop Out Draft",
            Self::Forward => "Forward, Pop Out Draft",
        }
    }

    pub(crate) const fn shortcut(self) -> &'static str {
        match self {
            Self::Reply => "R",
            Self::Forward => "F",
        }
    }

    pub(crate) const fn icon_view_box(self) -> &'static str {
        "-298 390.6 14 12.4"
    }

    pub(crate) fn icon_body(self, appearance_mode: AppearanceMode) -> String {
        let fill = mail_palette(appearance_mode).icon_rgb;
        match self {
            Self::Reply => format!(
                r##"<path fill="#{fill:06X}" d="M-288.5 394h-7.6l2.6-2.6-.6-.8-3.9 3.9 3.9 3.9.7-.8-2.7-2.6h7.6c1.9 0 3.5 1.6 3.5 3.5v4.5h1v-4.5c0-2.5-2-4.5-4.5-4.5z"></path>"##
            ),
            Self::Forward => format!(
                r##"<path fill="#{fill:06X}" d="M-298 398.5v4.5h1v-4.5c0-1.9 1.6-3.5 3.5-3.5h7.6l-2.7 2.6.7.8 3.9-3.9-3.9-3.9-.6.8 2.6 2.6h-7.6c-2.5 0-4.5 2-4.5 4.5z"></path>"##
            ),
        }
    }
}

impl MailRowAction {
    pub(crate) const fn ui_segment(self) -> &'static str {
        match self {
            Self::MarkDone => "mark_done",
            Self::RemindMe => "remind_me",
            Self::Move => "move",
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::MarkDone => "Mark Done",
            Self::RemindMe => "Remind Me",
            Self::Move => "Move",
        }
    }

    pub(crate) const fn shortcut(self) -> &'static str {
        match self {
            Self::MarkDone => "E",
            Self::RemindMe => "H",
            Self::Move => "V",
        }
    }

    pub(crate) const fn icon_view_box(self) -> &'static str {
        match self {
            Self::MarkDone => "0 0 16 14",
            Self::RemindMe | Self::Move => "0 0 14 14",
        }
    }

    pub(crate) const fn icon_size(self) -> (f32, f32) {
        match self {
            Self::MarkDone => (16.0, 14.0),
            Self::RemindMe | Self::Move => (14.0, 14.0),
        }
    }

    pub(crate) fn icon_body(self, appearance_mode: AppearanceMode) -> String {
        let fill = mail_palette(appearance_mode).icon_rgb;
        match self {
            Self::MarkDone => {
                format!(
                    r##"<path d="M1 7.56579L5.68075 11.8339C5.85771 11.9953 6.13152 11.9843 6.29509 11.8094L15 2.5" fill="none" stroke="#{fill:06X}" stroke-linecap="round"></path>"##
                )
            }
            Self::RemindMe => {
                format!(
                    r##"<path d="M7 0.0136719C10.8578 0.0136719 13.9852 3.14124 13.9854 6.99902C13.9854 10.857 10.8579 13.9844 7 13.9844C3.14211 13.9843 0.0146484 10.8569 0.0146484 6.99902C0.0148232 3.14127 3.14222 0.0137204 7 0.0136719ZM7 1.01367C3.6945 1.01372 1.01482 3.69355 1.01465 6.99902C1.01465 10.3046 3.69439 12.9843 7 12.9844C10.3056 12.9844 12.9854 10.3047 12.9854 6.99902C12.9852 3.69352 10.3055 1.01367 7 1.01367ZM6.81543 2.35547C7.09157 2.35547 7.31543 2.57933 7.31543 2.85547V6.92871L9.9375 7.91211C10.1959 8.00902 10.3271 8.29721 10.2305 8.55566C10.1334 8.81383 9.84524 8.94527 9.58691 8.84863L6.63965 7.74316C6.44483 7.66993 6.31559 7.48352 6.31543 7.27539V2.85547C6.31543 2.57939 6.53937 2.35557 6.81543 2.35547Z" fill="#{fill:06X}"></path>"##
                )
            }
            Self::Move => {
                format!(
                    r##"<path fill-rule="evenodd" clip-rule="evenodd" d="M6.41395 0.208715C6.77389 -0.0687834 7.27586 -0.0690576 7.63563 0.208715L12.5565 4.00852C13.073 4.4076 13.075 5.18711 12.5604 5.5886L10.7616 6.98996L12.5497 8.3718C13.0661 8.77082 13.068 9.55034 12.5536 9.95188L7.6327 13.7878L7.4911 13.8806C7.19631 14.0391 6.8397 14.0389 6.54481 13.8806L6.40321 13.7878L1.47938 9.95188C0.964224 9.55036 0.965217 8.76988 1.48231 8.37082L3.27821 6.98411L1.48622 5.5886C0.970917 5.1871 0.971993 4.40663 1.48914 4.00754L6.41395 0.208715ZM2.01747 9.06418C1.95296 9.11396 1.95244 9.21121 2.01649 9.26145L6.94129 13.0984C6.98635 13.133 7.0497 13.1333 7.09461 13.0984L11.5292 9.64036H7.0243C6.99232 9.64036 6.96087 9.63535 6.93055 9.62961C6.79968 9.61729 6.67062 9.58117 6.55164 9.51731L6.41004 9.42453L3.9911 7.53977L2.01747 9.06418ZM8.64539 8.64036H11.4667L10.0487 7.54563L8.64539 8.64036ZM2.10047 4.79953L7.0243 8.63645L11.9452 4.79953L7.0243 0.99973L2.10047 4.79953Z" fill="#{fill:06X}"></path>"##
                )
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailActionPaletteState {
    pub kind: MailActionPaletteKind,
    pub thread_id: String,
    pub selected_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailActionPaletteKind {
    RemindMe,
    Move,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MailActionPaletteOption {
    pub label: String,
    pub detail: String,
    pub action: MailActionPaletteOptionAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MailActionPaletteOptionAction {
    Snooze { preset: MailSnoozePreset },
    Move { mailbox_id: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MailSnoozePreset {
    LaterToday,
    Tomorrow,
    NextWeek,
}

impl MailSnoozePreset {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::LaterToday => "Later Today",
            Self::Tomorrow => "Tomorrow",
            Self::NextWeek => "Next Week",
        }
    }

    pub(crate) const fn detail(self) -> &'static str {
        match self {
            Self::LaterToday => "In 3 hours",
            Self::Tomorrow => "In 1 day",
            Self::NextWeek => "In 7 days",
        }
    }

    pub(crate) fn duration(self) -> time::Duration {
        match self {
            Self::LaterToday => time::Duration::hours(3),
            Self::Tomorrow => time::Duration::days(1),
            Self::NextWeek => time::Duration::days(7),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailFooterAction {
    Invite,
    ContactUs,
    Calendar,
    Settings,
}

impl MailFooterAction {
    pub(crate) const fn ui_segment(self) -> &'static str {
        match self {
            Self::Invite => "invite",
            Self::ContactUs => "contact_us",
            Self::Calendar => "calendar",
            Self::Settings => "settings",
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Invite => "Invite",
            Self::ContactUs => "Contact Us",
            Self::Calendar => "Calendar",
            Self::Settings => "Settings",
        }
    }

    pub(crate) const fn icon_view_box(self) -> &'static str {
        match self {
            Self::Calendar => "0 0 12 12",
            Self::Invite | Self::ContactUs | Self::Settings => "0 0 14 14",
        }
    }

    pub(crate) const fn icon_size(self) -> f32 {
        match self {
            Self::Calendar => 12.0,
            Self::Invite | Self::ContactUs | Self::Settings => 14.0,
        }
    }

    pub(crate) fn icon_body(self, appearance_mode: AppearanceMode) -> String {
        let fill = mail_palette(appearance_mode).icon_rgb;
        match self {
            Self::Invite => {
                format!(
                    r##"<path d="M2.84868 2.24731C2.76789 2.45319 2.7598 2.70089 2.90349 2.98827C3.00769 3.19666 3.20671 3.38655 3.51554 3.55504C3.82429 3.7235 4.2115 3.85322 4.64068 3.95482C5.19758 4.08665 5.75843 4.15905 6.26153 4.21998C5.95898 3.52368 5.436 2.64965 4.81571 1.8743C4.46335 1.43384 3.86473 1.39303 3.35989 1.6853C3.11466 1.82727 2.93409 2.02963 2.84868 2.24731ZM2.85885 0.819877C3.6602 0.355939 4.849 0.315124 5.59658 1.2496C6.12739 1.91311 6.61466 2.67167 6.96796 3.37345C7.27786 2.68022 7.71407 1.92334 8.25306 1.2496C9.00065 0.315124 10.1894 0.355939 10.9908 0.819877C11.3987 1.05601 11.7509 1.42081 11.9319 1.88202C12.1175 2.35501 12.1094 2.8979 11.8406 3.43548C11.6273 3.862 11.2787 4.17523 10.8758 4.40933H14V5.40933H12.5909V12.4548V12.9548H12.0909H1.90906H1.40906V12.4548V5.40933H0V4.40933H2.99412C2.59307 4.18328 2.22717 3.8717 2.00906 3.43548C1.74027 2.8979 1.73218 2.35501 1.91778 1.88202C2.09876 1.42081 2.45098 1.05601 2.85885 0.819877ZM6.34993 5.40933H2.40906V11.9548H6.5V5.40933H6.44066L6.34672 5.42378L6.34993 5.40933ZM7.54624 5.40933H7.5V11.9548H11.5909V5.40933H7.63697L7.64018 5.42378L7.54624 5.40933ZM11.001 2.24731C11.0818 2.45319 11.0898 2.70089 10.9462 2.98827C10.7249 3.43084 10.1323 3.74448 9.2733 3.95585C8.73648 4.08795 8.19217 4.16051 7.69905 4.22174C7.96677 3.51311 8.42488 2.63561 9.03393 1.8743C9.3863 1.43384 9.98491 1.39303 10.4898 1.6853C10.735 1.82727 10.9156 2.02963 11.001 2.24731Z" fill-rule="evenodd" clip-rule="evenodd" fill="#{fill:06X}"/>"##
                )
            }
            Self::ContactUs => {
                format!(
                    r##"<path d="M14 7C14 8.85652 13.2625 10.637 11.9497 11.9497C10.637 13.2625 8.85652 14 7 14C5.14348 14 3.36301 13.2625 2.05025 11.9497C0.737498 10.637 0 8.85652 0 7C0 5.14348 0.737498 3.36301 2.05025 2.05025C3.36301 0.737498 5.14348 0 7 0C8.85652 0 10.637 0.737498 11.9497 2.05025C13.2625 3.36301 14 5.14348 14 7ZM11.1781 11.1781C12.2862 10.07 12.9087 8.56709 12.9087 7C12.9087 5.43291 12.2862 3.93 11.1781 2.8219C10.07 1.7138 8.56709 1.09128 7 1.09128C5.43291 1.09128 3.93 1.7138 2.8219 2.8219C1.7138 3.93 1.09128 5.43291 1.09128 7C1.09128 8.56709 1.7138 10.07 2.8219 11.1781C3.93 12.2862 5.43291 12.9087 7 12.9087C8.56709 12.9087 10.07 12.2862 11.1781 11.1781ZM7.58445 9.42135H6.23534V10.6799H7.58445V9.42135ZM4.79561 4.76992C4.68822 5.07197 4.63452 5.40757 4.63452 5.77673H5.78228C5.78228 5.32031 5.8796 4.95786 6.07425 4.68938C6.27561 4.41419 6.58436 4.27659 7.00051 4.27659C7.08776 4.27659 7.18844 4.29673 7.30255 4.337C7.41665 4.37727 7.52404 4.44104 7.62472 4.52829C7.72541 4.61555 7.80931 4.73301 7.87643 4.88067C7.95026 5.02163 7.98717 5.19278 7.98717 5.39414C7.98717 5.56194 7.96368 5.70625 7.9167 5.82707C7.87643 5.94788 7.81602 6.05863 7.73547 6.15931C7.66164 6.25328 7.57438 6.34054 7.4737 6.42108C7.37974 6.50162 7.27906 6.58888 7.17166 6.68285C7.03742 6.79695 6.91996 6.91441 6.81928 7.03523C6.72531 7.14933 6.64477 7.28357 6.57765 7.43795C6.51053 7.59233 6.46019 7.77355 6.42663 7.98163C6.39978 8.18299 6.38636 8.43133 6.38636 8.72666H7.4737C7.4737 8.48503 7.49384 8.28031 7.53411 8.11251C7.5811 7.94471 7.64486 7.8004 7.7254 7.67958C7.80595 7.55206 7.89992 7.43795 8.00731 7.33727C8.1147 7.23659 8.23216 7.13255 8.35969 7.02516L8.68187 6.74326C8.78926 6.64258 8.88323 6.53183 8.96377 6.41101C9.05103 6.28348 9.11815 6.13582 9.16513 5.96802C9.21883 5.79351 9.24568 5.58879 9.24568 5.35387C9.24568 5.03169 9.18862 4.74308 9.07452 4.48802C8.96042 4.23296 8.80268 4.02153 8.60132 3.85373C8.40667 3.67922 8.17511 3.54834 7.90663 3.46108C7.64486 3.36711 7.3596 3.32013 7.05085 3.32013C6.68169 3.32013 6.34609 3.38054 6.04404 3.50135C5.74872 3.61546 5.49366 3.7799 5.27887 3.99469C5.0708 4.20947 4.90971 4.46788 4.79561 4.76992Z" fill-rule="evenodd" clip-rule="evenodd" fill="#{fill:06X}"/>"##
                )
            }
            Self::Calendar => {
                format!(
                    r##"<g fill="#{fill:06X}" fill-rule="evenodd"><path d="M4.032 6.998v.6c.104 0 .213.003.327.009a.992.992 0 0 1 .315.066.533.533 0 0 1 .237.183c.062.084.093.206.093.366a.615.615 0 0 1-.198.483.703.703 0 0 1-.486.177.724.724 0 0 1-.321-.066.656.656 0 0 1-.225-.177.79.79 0 0 1-.138-.264 1.212 1.212 0 0 1-.054-.321h-.81c-.004.244.031.46.105.648s.179.347.315.477.301.229.495.297c.194.068.409.102.645.102.204 0 .4-.03.588-.09.188-.06.354-.148.498-.264.144-.116.259-.26.345-.432.086-.172.129-.368.129-.588a.985.985 0 0 0-.198-.618.915.915 0 0 0-.546-.336v-.012a.737.737 0 0 0 .441-.318.994.994 0 0 0 .147-.54.97.97 0 0 0-.126-.498 1.218 1.218 0 0 0-.327-.366 1.435 1.435 0 0 0-.453-.225 1.76 1.76 0 0 0-.504-.075c-.216 0-.412.035-.588.105a1.302 1.302 0 0 0-.747.756c-.07.178-.109.375-.117.591h.81a.825.825 0 0 1 .159-.537c.11-.142.273-.213.489-.213.156 0 .294.048.414.144a.5.5 0 0 1 .18.414c0 .12-.029.216-.087.288a.558.558 0 0 1-.222.165.925.925 0 0 1-.291.069 1.802 1.802 0 0 1-.294 0zM8.352 9.5V5.3h-.678a.927.927 0 0 1-.15.402.913.913 0 0 1-.279.261 1.12 1.12 0 0 1-.372.138 2.017 2.017 0 0 1-.429.033v.642H7.5V9.5h.852z"></path><path d="M0 0h12v12H0V0zm1 1h10v10H1V1z"></path><path d="M1 3 H11 v1 H1 v-1 z"></path></g>"##
                )
            }
            Self::Settings => {
                format!(
                    r##"<path fill="#{fill:06X}" d="M4 7a3 3 0 1 0 6 0 3 3 0 0 0-6 0zm5 0a2 2 0 1 1-4 0 2 2 0 0 1 4 0z"></path><path fill="#{fill:06X}" d="M7 0c-.3 0-.627.022-.916.06l-.692.09-.153.68-.237 1.049a5.46 5.46 0 0 0-.21.087l-.909-.574-.59-.372-.552.425A7.498 7.498 0 0 0 1.445 2.74l-.425.553.372.59.574.908c-.03.069-.06.14-.087.21L.83 5.239l-.681.153-.09.692A7.522 7.522 0 0 0 0 7c0 .3.022.627.06.916l.09.692.68.153 1.049.237c.027.07.057.141.087.21l-.574.909-.372.59.425.552c.361.468.828.934 1.296 1.296l.553.425.59-.372.908-.574c.069.03.14.06.21.087l.237 1.048.153.681.692.09c.29.038.617.06.916.06.3 0 .627-.022.916-.06l.692-.09.153-.68.237-1.049a5.46 5.46 0 0 0 .21-.087l.909.574.59.372.552-.425a7.497 7.497 0 0 0 1.296-1.296l.425-.553-.372-.59-.574-.908c.03-.069.06-.14.087-.21l1.048-.237.681-.153.09-.692c.038-.29.06-.617.06-.916 0-.3-.022-.627-.06-.916l-.09-.692-.68-.153-1.049-.237a6.34 6.34 0 0 0-.087-.21l.574-.909.372-.59-.425-.552a7.497 7.497 0 0 0-1.296-1.296l-.553-.425-.59.372-.908.574a5.46 5.46 0 0 0-.21-.087L8.761.83 8.608.15 7.916.06A7.522 7.522 0 0 0 7 0zM2.647 5.853a4.49 4.49 0 0 1 .465-1.12L2.238 3.35c.32-.418.694-.792 1.112-1.112l1.383.874a4.49 4.49 0 0 1 1.12-.465l.361-1.596a6.056 6.056 0 0 1 1.572 0l.36 1.596c.398.105.774.262 1.12.465l1.384-.874c.418.32.792.694 1.112 1.112l-.874 1.383c.203.347.36.723.465 1.12l1.596.361a6.066 6.066 0 0 1 0 1.572l-1.596.36a4.473 4.473 0 0 1-.465 1.12l.874 1.384a6.03 6.03 0 0 1-1.112 1.112l-1.383-.874a4.49 4.49 0 0 1-1.12.465l-.361 1.596a6.066 6.066 0 0 1-1.572 0l-.36-1.596a4.473 4.473 0 0 1-1.12-.465l-1.384.874a6.033 6.033 0 0 1-1.112-1.112l.874-1.383a4.473 4.473 0 0 1-.465-1.12l-1.596-.361a6.056 6.056 0 0 1 0-1.572l1.596-.36z"></path>"##
                )
            }
        }
    }
}
