use crate::AppearanceMode;

pub const DARK_APP_BACKGROUND: u32 = 0x27292d;

pub const fn if_light(appearance_mode: AppearanceMode, light: f32, dark: f32) -> f32 {
    match appearance_mode {
        AppearanceMode::Light => light,
        AppearanceMode::Dark => dark,
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct SurfaceColorSpec {
    pub hex: u32,
    pub opacity: f32,
}

impl SurfaceColorSpec {
    pub const fn new(hex: u32, opacity: f32) -> Self {
        Self { hex, opacity }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct SurfaceTheme {
    pub app_bg: u32,
    pub elevated_surface_bg: u32,
    pub surface_border: SurfaceColorSpec,
    pub text_primary: u32,
    pub text_secondary: u32,
    pub text_muted: u32,
    pub text_hint: u32,
    pub command_menu_active_bg: SurfaceColorSpec,
    pub card_fill: u32,
    pub card_border: SurfaceColorSpec,
    pub card_shadow: SurfaceColorSpec,
    pub favorite_active_bg: SurfaceColorSpec,
    pub page_icon_bg: SurfaceColorSpec,
}

impl SurfaceTheme {
    pub const fn for_appearance_mode(appearance_mode: AppearanceMode) -> Self {
        match appearance_mode {
            AppearanceMode::Light => Self {
                app_bg: 0xffffff,
                elevated_surface_bg: 0xffffff,
                surface_border: SurfaceColorSpec::new(0x37352f, 0.075),
                text_primary: 0x2c2c2b,
                text_secondary: 0x4a4947,
                text_muted: 0x9b9a97,
                text_hint: 0x9b9a97,
                command_menu_active_bg: SurfaceColorSpec::new(0x37352f, 0.08),
                card_fill: 0xffffff,
                card_border: SurfaceColorSpec::new(0x37352f, 0.05),
                card_shadow: SurfaceColorSpec::new(0x37352f, 0.014),
                favorite_active_bg: SurfaceColorSpec::new(0x2a1c00, 0.07),
                page_icon_bg: SurfaceColorSpec::new(0x37352f, 0.04),
            },
            AppearanceMode::Dark => Self {
                app_bg: DARK_APP_BACKGROUND,
                elevated_surface_bg: 0x252525,
                surface_border: SurfaceColorSpec::new(0xffffff, 0.08),
                text_primary: 0xf0efed,
                text_secondary: 0xd8d6d1,
                text_muted: 0xada9a3,
                text_hint: 0x65645e,
                command_menu_active_bg: SurfaceColorSpec::new(0xffffff, 0.06),
                card_fill: 0x2c2c2b,
                card_border: SurfaceColorSpec::new(0xfffff3, 0.082),
                card_shadow: SurfaceColorSpec::new(0x191919, 0.08),
                favorite_active_bg: SurfaceColorSpec::new(0xffffff, 0.08),
                page_icon_bg: SurfaceColorSpec::new(0xffffff, 0.04),
            },
        }
    }
}

impl Default for SurfaceTheme {
    fn default() -> Self {
        Self::for_appearance_mode(AppearanceMode::Dark)
    }
}
