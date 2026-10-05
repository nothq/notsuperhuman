use app_model::{SurfaceFrame, SurfaceRoot as AppSurfaceRoot};
use gpui::{AnyElement, App, IntoElement, KeyDownEvent, Window};

use super::SurfaceRoot;

impl AppSurfaceRoot for SurfaceRoot {
    fn set_surface_frame(&mut self, frame: SurfaceFrame) {
        self.set_theme(frame.theme);
        self.set_preview_width(frame.preview_width);
        self.set_viewport_height(frame.viewport_height);
        self.set_window_controls_visible(frame.window_controls_visible);
        self.set_chrome_top_inset(frame.chrome_top_inset);
    }

    fn activate_surface(&mut self, frame: SurfaceFrame, cx: &mut App) {
        self.set_surface_frame(frame);
        self.ensure_surface_state(frame.active, cx);
    }

    fn render_surface_preview(&mut self, frame: SurfaceFrame, cx: &mut App) -> Option<AnyElement> {
        self.set_surface_frame(frame);
        frame.active.then(|| self.render_preview(cx))
    }

    fn handle_workspace_key_down(&mut self, event: &KeyDownEvent, cx: &mut App) -> bool {
        self.handle_key_down(event, cx)
    }

    fn render_surface(&mut self, _window: &mut Window, cx: &mut App) -> AnyElement {
        self.ensure_surface_app(cx).into_any_element()
    }
}
