use super::{parse_data_image_url, Arc, Context, Duration, Image, SurfaceState};
use crate::ui::mail_image_from_data;
use crate::ui::render_mail_remote_image;

const MAIL_REMOTE_IMAGE_LOAD_CONCURRENCY: usize = 2;
#[cfg(test)]
const MAIL_REMOTE_IMAGE_LOAD_IDLE_DELAY_MS: u64 = 0;
#[cfg(not(test))]
const MAIL_REMOTE_IMAGE_LOAD_IDLE_DELAY_MS: u64 = 80;

impl SurfaceState {
    pub(crate) fn mail_image_resolver(&self) -> crate::ui::MailImageResolver {
        let images = self.mail_remote_images.clone();
        Arc::new(move |url| images.get(url).cloned())
    }

    pub(crate) fn ensure_mail_remote_image_loads(&mut self, cx: &mut Context<Self>) {
        let Some(thread_id) = self.mail_open_thread_id.clone() else {
            self.mail_remote_image_discovery_thread_id = None;
            return;
        };
        if self.mail_remote_image_discovery_thread_id.as_deref() == Some(thread_id.as_str()) {
            return;
        }
        let Some(thread) = self.mail_thread_detail_cache.get(&thread_id).cloned() else {
            return;
        };
        let mut queued = false;
        for message in &thread.message_details {
            for url in message.body.document.image_urls.iter().cloned() {
                queued |= self.enqueue_mail_remote_image_url(url);
            }
        }
        self.mail_remote_image_discovery_thread_id = Some(thread_id);
        if queued {
            self.schedule_mail_remote_image_loads_after_idle(cx);
        }
    }

    fn enqueue_mail_remote_image_url(&mut self, url: String) -> bool {
        if self.mail_remote_images.contains_key(&url)
            || self.mail_active_remote_image_urls.contains(url.as_str())
            || self
                .mail_pending_remote_image_urls
                .iter()
                .any(|pending| pending == &url)
            || self.mail_failed_remote_image_urls.contains(url.as_str())
        {
            return false;
        }
        if url.starts_with("data:") && parse_data_image_url(&url).is_none() {
            self.mail_failed_remote_image_urls.insert(url);
            return false;
        }
        if parse_data_image_url(&url).is_some()
            || url.starts_with("http://")
            || url.starts_with("https://")
        {
            self.mail_pending_remote_image_urls.push_back(url);
            return true;
        }
        false
    }

    fn schedule_mail_remote_image_loads_after_idle(&mut self, cx: &mut Context<Self>) {
        if self.mail_pending_remote_image_urls.is_empty() || self.mail_remote_image_load_scheduled {
            return;
        }
        self.mail_remote_image_load_scheduled = true;
        self.spawn_timer_task(
            (),
            Duration::from_millis(MAIL_REMOTE_IMAGE_LOAD_IDLE_DELAY_MS),
            cx,
            |this, _, cx| {
                this.mail_remote_image_load_scheduled = false;
                this.start_next_mail_remote_image_load(cx);
            },
        );
    }

    fn start_next_mail_remote_image_load(&mut self, cx: &mut Context<Self>) {
        while self.mail_active_remote_image_urls.len() < MAIL_REMOTE_IMAGE_LOAD_CONCURRENCY {
            let Some(url) = self.mail_pending_remote_image_urls.pop_front() else {
                return;
            };
            if self.mail_remote_images.contains_key(&url)
                || self.mail_active_remote_image_urls.contains(url.as_str())
            {
                continue;
            }
            self.mail_active_remote_image_urls.insert(url.clone());
            let generation = self.mail_remote_image_generation;
            if parse_data_image_url(&url).is_some() {
                self.spawn_mail_data_image_load(url, generation, cx);
                continue;
            }
            let Some(workspace_api) = self.mail_workspace_api() else {
                self.mail_active_remote_image_urls.remove(url.as_str());
                continue;
            };
            self.spawn_mail_remote_image_load(url, generation, workspace_api, cx);
        }
    }

    fn spawn_mail_data_image_load(&mut self, url: String, generation: u64, cx: &mut Context<Self>) {
        self.spawn_background_task(
            url.clone(),
            cx,
            move |url| parse_data_image_url(&url).and_then(mail_image_from_data),
            move |this, image, cx| {
                this.finish_mail_remote_image_load(&url, generation, Ok(image), cx);
            },
        );
    }

    fn spawn_mail_remote_image_load(
        &mut self,
        url: String,
        generation: u64,
        workspace_api: Arc<dyn crate::model::MailWorkspaceApi>,
        cx: &mut Context<Self>,
    ) {
        self.spawn_background_task(
            url.clone(),
            cx,
            move |url| {
                let Some(image_data) = workspace_api.load_remote_image(&url)? else {
                    return Ok(None);
                };
                let mimetype = image_data.mimetype.clone();
                mail_image_from_data(image_data).map(Some).ok_or_else(|| {
                    format!("failed to decode mail remote image {url} as {mimetype}")
                })
            },
            move |this, result, cx| {
                this.finish_mail_remote_image_load(&url, generation, result, cx);
            },
        );
    }

    fn finish_mail_remote_image_load(
        &mut self,
        url: &str,
        generation: u64,
        result: Result<Option<Image>, String>,
        cx: &mut Context<Self>,
    ) {
        if generation != self.mail_remote_image_generation {
            return;
        }
        let affects_open_thread = self.mail_open_thread_detail().is_some_and(|thread| {
            thread.message_details.iter().any(|message| {
                message
                    .body
                    .document
                    .image_urls
                    .iter()
                    .any(|image_url| image_url == url)
            })
        });
        self.mail_active_remote_image_urls.remove(url);
        match result {
            Ok(Some(image)) => {
                if let Some(image) = render_mail_remote_image(Arc::new(image), cx) {
                    self.mail_remote_images.insert(url.to_string(), image);
                    if affects_open_thread {
                        self.remeasure_mail_open_thread_body_list_state();
                    }
                    cx.notify();
                } else {
                    println!("[notsuperhuman mail] remote image: failed to render {url}");
                    self.mail_failed_remote_image_urls.insert(url.to_string());
                }
            }
            Ok(None) => {
                println!("[notsuperhuman mail] remote image: no image returned for {url}");
                self.mail_failed_remote_image_urls.insert(url.to_string());
            }
            Err(error) => {
                println!("[notsuperhuman mail] remote image: {error}");
                self.mail_failed_remote_image_urls.insert(url.to_string());
            }
        }
        if !self.mail_pending_remote_image_urls.is_empty() {
            self.schedule_mail_remote_image_loads_after_idle(cx);
        }
    }

    pub(crate) fn clear_mail_remote_images(&mut self) {
        self.mail_remote_images.clear();
        self.mail_active_remote_image_urls.clear();
        self.mail_pending_remote_image_urls.clear();
        self.mail_failed_remote_image_urls.clear();
        self.mail_remote_image_load_scheduled = false;
        self.mail_remote_image_discovery_thread_id = None;
        self.mail_remote_image_generation = self.mail_remote_image_generation.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::{Image, SurfaceState};
    use crate::ui::{
        AppearanceMode, ImageFormat, MailSurfaceStateConfig, SurfaceInput, SurfaceTheme,
    };
    use gpui::{AppContext, TestAppContext};
    use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};

    #[gpui::test]
    fn failed_remote_image_urls_are_not_requeued(cx: &mut TestAppContext) {
        let surface = test_surface(cx);
        cx.update_entity(&surface, |surface, _cx| {
            let url = "https://example.test/missing.png".to_string();

            assert!(surface.enqueue_mail_remote_image_url(url.clone()));
            assert_eq!(surface.mail_pending_remote_image_urls.len(), 1);
            surface.mail_pending_remote_image_urls.clear();
            surface.mail_failed_remote_image_urls.insert(url.clone());

            assert!(!surface.enqueue_mail_remote_image_url(url));
            assert!(surface.mail_pending_remote_image_urls.is_empty());
        });
    }

    #[gpui::test]
    fn invalid_data_image_urls_are_recorded_as_failures(cx: &mut TestAppContext) {
        let surface = test_surface(cx);
        cx.update_entity(&surface, |surface, _cx| {
            let url = "data:image/png,not-base64".to_string();

            assert!(!surface.enqueue_mail_remote_image_url(url.clone()));
            assert!(surface.mail_failed_remote_image_urls.contains(url.as_str()));
            assert!(surface.mail_pending_remote_image_urls.is_empty());
        });
    }

    #[gpui::test]
    fn stale_remote_image_completion_does_not_repopulate_cache(cx: &mut TestAppContext) {
        let surface = test_surface(cx);
        cx.update_entity(&surface, |surface, cx| {
            let url = "https://example.test/logo.png";
            let generation = surface.mail_remote_image_generation;
            let image = Image::from_bytes(ImageFormat::Png, png_bytes(1, 1));
            surface
                .mail_active_remote_image_urls
                .insert(url.to_string());

            surface.clear_mail_remote_images();
            surface.finish_mail_remote_image_load(url, generation, Ok(Some(image)), cx);

            assert!(surface.mail_remote_images.is_empty());
            assert!(surface.mail_failed_remote_image_urls.is_empty());
            assert!(surface.mail_active_remote_image_urls.is_empty());
        });
    }

    fn test_surface(cx: &mut TestAppContext) -> gpui::Entity<SurfaceState> {
        cx.update(|cx| cx.set_global(AppearanceMode::Dark));
        cx.new(|cx| {
            SurfaceState::new(
                MailSurfaceStateConfig {
                    input: SurfaceInput::default(),
                    local_file_api: crate::ui::test_support::mail_test_local_file_api(),
                    startup: crate::ui::surface::MailStartup::Fixture,
                    theme: SurfaceTheme::default(),
                    preview_width: 720.0,
                    viewport_height: 640.0,
                    window_controls_visible: false,
                    chrome_top_inset: 0.0,
                        },
                cx,
            )
        })
    }

    fn png_bytes(width: u32, height: u32) -> Vec<u8> {
        let rgba = vec![0_u8; width as usize * height as usize * 4];
        let mut bytes = Vec::new();
        PngEncoder::new(&mut bytes)
            .write_image(&rgba, width, height, ExtendedColorType::Rgba8)
            .expect("encode test png");
        bytes
    }
}
