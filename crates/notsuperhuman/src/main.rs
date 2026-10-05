use std::{borrow::Cow, ffi::c_void, time::Duration};

use app_model::{AppearanceMode, SurfaceFrame, SurfaceRoot as _, SurfaceTheme, Viewport};
use gpui::{
    actions, div, point, prelude::*, px, rgb, size, App, Bounds, Context, FocusHandle, KeyBinding,
    KeyDownEvent, Menu, MenuItem, TitlebarOptions, Window, WindowAppearance, WindowBounds,
    WindowOptions,
};

actions!(notsuperhuman, [Quit]);

#[cfg(target_os = "macos")]
extern "C" {
    fn malloc_zone_pressure_relief(zone: *mut c_void, goal: usize) -> usize;
}

/// Mail arrives as large JSON documents that are parsed and then dropped.
/// macOS keeps the freed pages charged to the process until the system runs
/// short, so hand them back once the app goes quiet.
fn release_freed_memory(cx: &mut App) {
    let executor = cx.background_executor().clone();
    cx.background_spawn(async move {
        loop {
            executor.timer(Duration::from_secs(10)).await;
            #[cfg(target_os = "macos")]
            // SAFETY: a null zone asks every malloc zone to return free pages.
            unsafe {
                malloc_zone_pressure_relief(std::ptr::null_mut(), 0)
            };
        }
    })
    .detach();
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        cx.text_system()
            .add_fonts(vec![
                Cow::Borrowed(include_bytes!("../assets/fonts/Lato-Regular.ttf").as_slice()),
                Cow::Borrowed(include_bytes!("../assets/fonts/Lato-Bold.ttf").as_slice()),
                Cow::Borrowed(include_bytes!("../assets/fonts/Lato-Black.ttf").as_slice()),
            ])
            .expect("bundled Lato fonts load");
        theme::init(theme::LoadThemes::JustBase, cx);
        cx.set_global(appearance_mode(cx.window_appearance()));
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.set_menus([
            Menu::new("notsuperhuman").items([MenuItem::action("Quit notsuperhuman", Quit)])
        ]);
        cx.on_window_closed(|cx, _| cx.quit()).detach();
        release_freed_memory(cx);

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1320.0), px(860.0)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("notsuperhuman".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(14.0), px(14.0))),
            }),
            window_min_size: Some(size(px(854.0), px(480.0))),
            ..Default::default()
        };
        cx.open_window(options, |window, cx| {
            cx.new(|cx| Notsuperhuman::new(window, cx))
        })
        .expect("open the notsuperhuman window");
        cx.activate(true);
    });
}

struct Notsuperhuman {
    root: mail::ui::SurfaceRoot,
    focus: FocusHandle,
}

impl Notsuperhuman {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        window
            .observe_window_appearance(|window, cx| {
                cx.set_global(appearance_mode(window.appearance()));
            })
            .detach();
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self {
            root: mail::production_root(),
            focus,
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.root.allows_workspace_shell_key_down(window, cx)
            && self.root.handle_workspace_key_down(event, cx)
        {
            cx.stop_propagation();
        }
    }
}

fn appearance_mode(appearance: WindowAppearance) -> AppearanceMode {
    match appearance {
        WindowAppearance::Light | WindowAppearance::VibrantLight => AppearanceMode::Light,
        WindowAppearance::Dark | WindowAppearance::VibrantDark => AppearanceMode::Dark,
    }
}

impl Render for Notsuperhuman {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bounds = window.bounds();
        let viewport = Viewport {
            logical_width: f32::from(bounds.size.width).round() as u32,
            logical_height: f32::from(bounds.size.height).round() as u32,
            scale_factor: f64::from(window.scale_factor()),
            ..Viewport::default()
        };
        let theme = SurfaceTheme::for_appearance_mode(*cx.global::<AppearanceMode>());
        let frame = SurfaceFrame {
            theme,
            viewport,
            preview_width: viewport.app_width(),
            viewport_height: viewport.app_height(),
            active: true,
            window_controls_visible: true,
            chrome_top_inset: 0.0,
            ..SurfaceFrame::default()
        };
        self.root.prepare_surface_window_frame(frame, window, cx);
        div()
            .size_full()
            .flex()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .bg(rgb(theme.app_bg))
            .child(self.root.render_surface(window, cx))
    }
}
