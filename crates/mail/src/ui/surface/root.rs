use crate::ui::*;

mod compose;
mod construction;

pub type MailRemoteImages = Vec<(String, Arc<Image>)>;

#[derive(Clone)]
pub struct SurfaceInput {
    pub workspace: Option<MailWorkspace>,
    pub workspace_api: Option<Arc<dyn crate::model::MailWorkspaceApi>>,
    pub local_file_api: Arc<dyn crate::model::MailLocalFileApi>,
}

#[cfg(any(test, feature = "test-support"))]
impl Default for SurfaceInput {
    fn default() -> Self {
        Self {
            workspace: None,
            workspace_api: None,
            local_file_api: crate::ui::test_support::mail_test_local_file_api(),
        }
    }
}

#[derive(Clone)]
pub(crate) enum MailStartup {
    #[cfg(any(test, feature = "test-support"))]
    Fixture,
    Loading {
        bootstrap_api: Arc<dyn crate::model::MailBootstrapApi>,
        request_started: bool,
    },
    Ready(Box<crate::model::MailWorkspaceBootstrap>),
    Error {
        bootstrap_api: Arc<dyn crate::model::MailBootstrapApi>,
        error: String,
    },
    /// No account is signed in yet; the surface shows the sign-in screen.
    SignedOut,
}

pub struct SurfaceRoot {
    #[cfg(any(test, feature = "test-support"))]
    input_dirty: bool,
    #[cfg(any(test, feature = "test-support"))]
    input: SurfaceInput,
    local_file_api: Arc<dyn crate::model::MailLocalFileApi>,
    startup: MailStartup,
    theme: SurfaceTheme,
    preview_width: f32,
    viewport_height: f32,
    window_controls_visible: bool,
    chrome_top_inset: f32,
    pub(crate) surface: Option<gpui::Entity<SurfaceState>>,
}

impl SurfaceRoot {
    pub fn set_theme(&mut self, theme: SurfaceTheme) {
        self.theme = theme;
    }

    pub fn set_preview_width(&mut self, preview_width: f32) {
        self.preview_width = preview_width;
    }

    pub fn set_viewport_height(&mut self, viewport_height: f32) {
        self.viewport_height = viewport_height;
    }

    pub fn set_window_controls_visible(&mut self, visible: bool) {
        self.window_controls_visible = visible;
    }

    pub fn set_chrome_top_inset(&mut self, chrome_top_inset: f32) {
        self.chrome_top_inset = chrome_top_inset.max(0.0);
    }

    pub fn ensure_surface_state(&mut self, active: bool, cx: &mut App) {
        if !active {
            return;
        }
        let surface = self.ensure_surface(cx);
        #[cfg(any(test, feature = "test-support"))]
        let input = self.take_dirty_input();
        let theme = self.theme;
        let preview_width = self.preview_width;
        let viewport_height = self.viewport_height;
        let window_controls_visible = self.window_controls_visible;
        let chrome_top_inset = self.chrome_top_inset;
        surface.update(cx, |surface, cx| {
            #[cfg(any(test, feature = "test-support"))]
            if let Some((input, startup)) = input {
                surface.apply_input(input, startup);
            }
            surface.theme = theme;
            surface.appearance_mode = AppearanceMode::current(cx);
            surface.preview_width = preview_width;
            surface.viewport_height = viewport_height;
            surface.window_controls_visible = window_controls_visible;
            surface.chrome_top_inset = chrome_top_inset;
            surface.ensure_mail_surface_state(cx);
        });
    }

    pub fn handle_key_down(&mut self, event: &KeyDownEvent, cx: &mut App) -> bool {
        let Some(surface) = self.surface.as_ref() else {
            return false;
        };
        surface.update(cx, |surface, cx| surface.handle_mail_key_down(event, cx))
    }

    pub fn render_preview(&mut self, cx: &mut App) -> AnyElement {
        let surface = self.ensure_surface(cx);
        #[cfg(any(test, feature = "test-support"))]
        let input = self.take_dirty_input();
        let theme = self.theme;
        let preview_width = self.preview_width;
        let viewport_height = self.viewport_height;
        let window_controls_visible = self.window_controls_visible;
        let chrome_top_inset = self.chrome_top_inset;
        surface.update(cx, |surface, cx| {
            #[cfg(any(test, feature = "test-support"))]
            if let Some((input, startup)) = input {
                surface.apply_input(input, startup);
            }
            surface.theme = theme;
            surface.appearance_mode = AppearanceMode::current(cx);
            surface.preview_width = preview_width;
            surface.viewport_height = viewport_height;
            surface.window_controls_visible = window_controls_visible;
            surface.chrome_top_inset = chrome_top_inset;
            surface.ensure_mail_surface_state(cx);
        });
        surface.into_any_element()
    }

    fn ensure_surface(&mut self, cx: &mut App) -> gpui::Entity<SurfaceState> {
        if let Some(surface) = self.surface.as_ref() {
            let surface = surface.clone();
            #[cfg(any(test, feature = "test-support"))]
            if let Some((input, startup)) = self.take_dirty_input() {
                surface.update(cx, |surface, _cx| {
                    surface.apply_input(input, startup);
                });
            }
            return surface;
        }
        let surface = cx.new(|cx| {
            SurfaceState::new(
                MailSurfaceStateConfig {
                    #[cfg(any(test, feature = "test-support"))]
                    input: self.input.clone(),
                    local_file_api: self.local_file_api.clone(),
                    startup: self.startup.clone(),
                    theme: self.theme,
                    preview_width: self.preview_width,
                    viewport_height: self.viewport_height,
                    window_controls_visible: self.window_controls_visible,
                    chrome_top_inset: self.chrome_top_inset,
                },
                cx,
            )
        });
        #[cfg(any(test, feature = "test-support"))]
        {
            self.input_dirty = false;
        }
        self.surface = Some(surface.clone());
        surface
    }

    fn ensure_surface_app(&mut self, cx: &mut App) -> gpui::Entity<SurfaceState> {
        if let Some(surface) = self.surface.as_ref() {
            let surface = surface.clone();
            #[cfg(any(test, feature = "test-support"))]
            if let Some((input, startup)) = self.take_dirty_input() {
                surface.update(cx, |surface, _cx| {
                    surface.apply_input(input, startup);
                });
            }
            return surface;
        }
        let surface = cx.new(|cx| {
            SurfaceState::new(
                MailSurfaceStateConfig {
                    #[cfg(any(test, feature = "test-support"))]
                    input: self.input.clone(),
                    local_file_api: self.local_file_api.clone(),
                    startup: self.startup.clone(),
                    theme: self.theme,
                    preview_width: self.preview_width,
                    viewport_height: self.viewport_height,
                    window_controls_visible: self.window_controls_visible,
                    chrome_top_inset: self.chrome_top_inset,
                },
                cx,
            )
        });
        #[cfg(any(test, feature = "test-support"))]
        {
            self.input_dirty = false;
        }
        self.surface = Some(surface.clone());
        surface
    }

    #[cfg(any(test, feature = "test-support"))]
    fn take_dirty_input(&mut self) -> Option<(SurfaceInput, MailStartup)> {
        if !self.input_dirty {
            return None;
        }
        self.input_dirty = false;
        Some((self.input.clone(), self.startup.clone()))
    }
}

#[cfg(test)]
mod tests;
