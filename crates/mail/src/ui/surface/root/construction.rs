#[cfg(any(test, feature = "test-support"))]
use super::SurfaceInput;
use super::{MailStartup, SurfaceRoot};
use crate::ui::{Arc, SurfaceTheme, MAIL_PREVIEW_MIN_WIDTH};

#[cfg(any(test, feature = "test-support"))]
impl Default for SurfaceRoot {
    fn default() -> Self {
        Self::fixture()
    }
}

impl SurfaceRoot {
    pub fn production(
        bootstrap_api: Arc<dyn crate::model::MailBootstrapApi>,
        local_file_api: Arc<dyn crate::model::MailLocalFileApi>,
    ) -> Self {
        Self {
            #[cfg(any(test, feature = "test-support"))]
            input_dirty: true,
            #[cfg(any(test, feature = "test-support"))]
            input: SurfaceInput {
                workspace: None,
                workspace_api: None,
                local_file_api: local_file_api.clone(),
            },
            local_file_api,
            startup: MailStartup::Loading {
                bootstrap_api,
                request_started: false,
            },
            theme: SurfaceTheme::default(),
            preview_width: MAIL_PREVIEW_MIN_WIDTH,
            viewport_height: 800.0,
            window_controls_visible: false,
            chrome_top_inset: 0.0,
            surface: None,
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture() -> Self {
        Self::from_input(SurfaceInput::default())
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn from_input(input: SurfaceInput) -> Self {
        let local_file_api = input.local_file_api.clone();
        Self {
            input,
            local_file_api,
            startup: MailStartup::Fixture,
            input_dirty: true,
            theme: SurfaceTheme::default(),
            preview_width: MAIL_PREVIEW_MIN_WIDTH,
            viewport_height: 800.0,
            window_controls_visible: false,
            chrome_top_inset: 0.0,
            surface: None,
        }
    }
}
