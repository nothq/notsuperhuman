use super::{SurfaceInput, SurfaceRoot};
use crate::ui::{AppearanceMode, MailMessagePage, MailWorkspace, SurfaceState, SurfaceTheme};
use app_model::{SurfaceFrame, SurfaceRoot as AppSurfaceRoot};
use gpui::{AppContext, InputEvent, Modifiers, MouseButton, MouseDownEvent, TestAppContext};

mod search;
mod support;

use support::{
    paged_test_api, stale_cached_test_api, test_account_loader, test_api, test_message_at,
    test_workspace, uncached_test_api, PageRequests,
};

#[test]
fn mail_surface_frame_tracks_window_controls_visibility() {
    let mut root = SurfaceRoot::from_input(SurfaceInput::default());
    AppSurfaceRoot::set_surface_frame(
        &mut root,
        SurfaceFrame {
            window_controls_visible: true,
            ..SurfaceFrame::default()
        },
    );
    assert!(root.window_controls_visible);
}

#[gpui::test]
fn repeated_render_keeps_live_mail_workspace_after_folder_selection(cx: &mut TestAppContext) {
    let root = cx.new(|_| SurfaceRoot::from_input(SurfaceInput::default()));
    let surface = ensure_mail_surface(&root, cx);
    cx.update_entity(&surface, |surface, cx| {
        surface.apply_mail_workspace_bootstrap(
            crate::model::MailWorkspaceBootstrap::new(
                vec![crate::model::MailAccountInfo {
                    id: "account".to_string(),
                    name: "notsuperhuman".to_string(),
                    address: Some("notsuperhuman@example.test".to_string()),
                    is_primary: true,
                    is_personal: true,
                    is_read_only: false,
                    can_submit: true,
                }],
                test_workspace("inbox"),
                test_api(),
                false,
                test_account_loader(),
            ),
            cx,
        );
        surface.set_mail_selected_tab("drafts", cx);
    });
    cx.update_entity(&root, |root, cx| root.ensure_surface_state(true, cx));
    assert_surface_on_drafts(cx, &surface);
    cx.run_until_parked();
    cx.update_entity(&surface, |surface, cx| surface.open_new_mail_composer(cx));
    assert_new_composer_has_draft(cx, &surface);
}

#[gpui::test]
fn clicking_mail_compose_button_opens_new_composer(cx: &mut TestAppContext) {
    let (surface, cx) = cx.add_window_view(|_window, cx| mail_surface_state(Some(test_api()), cx));
    cx.refresh().expect("refresh mail surface window");
    let compose_button = cx
        .debug_bounds("mail-compose")
        .expect("mail compose button bounds");
    cx.update(|window, app| dispatch_mouse_down(window, app, compose_button.center()));
    assert_new_composer_waiting_for_identity(cx, &surface);
    assert_new_composer_has_draft(cx, &surface);
}

#[gpui::test]
fn pointer_hover_does_not_replace_keyboard_selection(cx: &mut TestAppContext) {
    let surface = cx.new(|cx| mail_surface_state(Some(test_api()), cx));
    cx.update_entity(&surface, |surface, cx| {
        surface.mail_selected_thread_id = Some("inbox-thread".to_string());
        surface.set_mail_thread_hover("drafts-thread", true, cx);
        surface.set_mail_thread_hover("drafts-thread", false, cx);

        assert_eq!(
            surface.mail_selected_thread_id.as_deref(),
            Some("inbox-thread")
        );
        assert_eq!(surface.mail_hovered_thread_id, None);
    });
}

#[gpui::test]
fn opening_mail_modal_clears_underlying_thread_hover(cx: &mut TestAppContext) {
    let surface = cx.new(|cx| mail_surface_state(Some(test_api()), cx));
    cx.update_entity(&surface, |surface, cx| {
        surface.mail_hovered_thread_id = Some("inbox-thread".to_string());

        surface.toggle_mail_folder_drawer(cx);

        assert!(surface.mail_folder_drawer_open);
        assert_eq!(surface.mail_hovered_thread_id, None);

        surface.open_mail_account_palette(cx);

        assert!(surface.mail_account_palette.is_some());
        assert!(!surface.mail_folder_drawer_open);
        assert_eq!(surface.mail_hovered_thread_id, None);
    });
}

#[gpui::test]
fn clicking_mail_header_tab_replaces_uncached_displayed_rows(cx: &mut TestAppContext) {
    let (surface, cx) =
        cx.add_window_view(|_window, cx| mail_surface_state(Some(uncached_test_api()), cx));
    cx.refresh().expect("refresh mail surface window");
    cx.run_until_parked();
    assert_eq!(visible_thread_ids(cx, &surface), vec!["inbox-thread"]);
    let drafts_tab = cx
        .debug_bounds("mail-tab-drafts")
        .expect("drafts tab bounds");
    cx.update(|window, app| dispatch_mouse_down(window, app, drafts_tab.center()));
    cx.read_entity(&surface, |surface, _cx| {
        assert_eq!(surface.mail_selected_tab_id, "drafts");
    });
    assert_eq!(visible_thread_ids(cx, &surface), vec!["drafts-thread"]);
}

#[gpui::test]
fn clicking_sent_mail_header_tab_replaces_stale_cached_rows(cx: &mut TestAppContext) {
    let (surface, cx) =
        cx.add_window_view(|_window, cx| mail_surface_state(Some(stale_cached_test_api()), cx));
    cx.refresh().expect("refresh mail surface window");
    cx.run_until_parked();
    assert_eq!(visible_thread_ids(cx, &surface), vec!["inbox-thread"]);
    let sent_tab = cx.debug_bounds("mail-tab-sent").expect("sent tab bounds");
    cx.update(|window, app| dispatch_mouse_down(window, app, sent_tab.center()));
    cx.read_entity(&surface, |surface, _cx| {
        assert_eq!(surface.mail_selected_tab_id, "sent");
    });
    assert_eq!(visible_thread_ids(cx, &surface), vec!["sent-thread"]);
}

#[gpui::test]
fn open_mail_thread_waits_for_loaded_thread_instead_of_summary_preview(cx: &mut TestAppContext) {
    let root = mail_root_entity(cx, Some(test_api()));
    let surface = ensure_mail_surface(&root, cx);
    cx.update_entity(&surface, |surface, cx| {
        surface.ensure_mail_active_thread_detail_cached();
        assert!(
            !surface
                .mail_thread_detail_cache
                .contains_key("inbox-thread"),
            "selected rows should not precompute open-thread detail"
        );

        surface.open_mail_thread("inbox-thread", cx);

        assert!(
            !surface
                .mail_thread_detail_cache
                .contains_key("inbox-thread"),
            "opening the thread waits for loaded detail"
        );
        assert!(surface.mail_open_thread_detail().is_none());
    });
    cx.run_until_parked();
    cx.update_entity(&surface, |surface, _cx| {
        assert_eq!(
            surface.mail_error.as_deref(),
            Some("unused test API method")
        );
        assert!(surface.mail_open_thread_detail().is_none());
    });
}

#[gpui::test]
fn near_bottom_mail_list_appends_next_message_page(cx: &mut TestAppContext) {
    let (workspace, page) = paged_workspace_and_page();
    let (api, page_requests) = paged_test_api(workspace.clone(), vec![page]);
    let surface = cx.new(|cx| mail_surface_state_with_workspace(workspace, Some(api), cx));

    cx.update_entity(&surface, |surface, cx| {
        surface.maybe_load_more_mail_messages(2, 2, cx);
    });
    cx.run_until_parked();

    assert_eq!(
        recorded_page_requests(&page_requests),
        vec![("inbox".to_string(), 2)]
    );
    assert_eq!(
        visible_thread_ids(cx, &surface),
        vec![
            "inbox-thread-0",
            "inbox-thread-1",
            "inbox-thread-2",
            "inbox-thread-3",
        ]
    );
    cx.read_entity(&surface, |surface, _cx| {
        let workspace = surface.mail_workspace().expect("mail workspace");
        assert_eq!(workspace.messages.len(), 4);
        assert_eq!(workspace.message_next_position, Some(4));
        assert_eq!(surface.mail_list_state.item_count(), 4);
    });
}

#[gpui::test]
fn repeated_near_bottom_events_do_not_duplicate_message_page_load(cx: &mut TestAppContext) {
    let (workspace, page) = paged_workspace_and_page();
    let (api, page_requests) = paged_test_api(workspace.clone(), vec![page]);
    let surface = cx.new(|cx| mail_surface_state_with_workspace(workspace, Some(api), cx));

    cx.update_entity(&surface, |surface, cx| {
        surface.maybe_load_more_mail_messages(2, 2, cx);
        surface.maybe_load_more_mail_messages(2, 2, cx);
    });
    cx.run_until_parked();

    assert_eq!(
        recorded_page_requests(&page_requests),
        vec![("inbox".to_string(), 2)]
    );
    assert_eq!(visible_thread_ids(cx, &surface).len(), 4);
}

#[gpui::test]
fn stale_message_page_after_mailbox_switch_is_dropped(cx: &mut TestAppContext) {
    let (workspace, page) = paged_workspace_and_page();
    let (api, page_requests) = paged_test_api(workspace.clone(), vec![page]);
    let surface = cx.new(|cx| mail_surface_state_with_workspace(workspace, Some(api), cx));

    cx.update_entity(&surface, |surface, cx| {
        surface.maybe_load_more_mail_messages(2, 2, cx);
        surface.set_mail_selected_tab("drafts", cx);
    });
    cx.run_until_parked();

    assert_eq!(
        recorded_page_requests(&page_requests),
        vec![("inbox".to_string(), 2)]
    );
    assert_eq!(visible_thread_ids(cx, &surface), vec!["drafts-thread"]);
    cx.read_entity(&surface, |surface, _cx| {
        assert_eq!(surface.mail_selected_tab_id, "drafts");
        assert_eq!(surface.mail_workspace().unwrap().messages.len(), 1);
        assert_eq!(surface.mail_loading_message_page, None);
    });
}

fn mail_root_entity(
    cx: &mut TestAppContext,
    workspace_api: Option<std::sync::Arc<dyn crate::model::MailWorkspaceApi>>,
) -> gpui::Entity<SurfaceRoot> {
    cx.new(|_| {
        SurfaceRoot::from_input(SurfaceInput {
            workspace: Some(test_workspace("inbox")),
            workspace_api,
            local_file_api: crate::ui::test_support::mail_test_local_file_api(),
        })
    })
}

fn ensure_mail_surface(
    root: &gpui::Entity<SurfaceRoot>,
    cx: &mut TestAppContext,
) -> gpui::Entity<SurfaceState> {
    cx.update_entity(root, |root, cx| {
        cx.set_global(AppearanceMode::Dark);
        root.ensure_surface_state(true, cx);
        root.surface.as_ref().expect("mail surface").clone()
    })
}

fn mail_surface_state(
    workspace_api: Option<std::sync::Arc<dyn crate::model::MailWorkspaceApi>>,
    cx: &mut gpui::Context<SurfaceState>,
) -> SurfaceState {
    mail_surface_state_with_workspace(test_workspace("inbox"), workspace_api, cx)
}

fn mail_surface_state_with_workspace(
    workspace: MailWorkspace,
    workspace_api: Option<std::sync::Arc<dyn crate::model::MailWorkspaceApi>>,
    cx: &mut gpui::Context<SurfaceState>,
) -> SurfaceState {
    cx.set_global(AppearanceMode::Dark);
    let selected_tab_id = workspace.selected_mailbox_id.clone();
    let mut surface = SurfaceState::new(
        crate::ui::MailSurfaceStateConfig {
            input: SurfaceInput {
                workspace: Some(workspace),
                workspace_api,
                local_file_api: crate::ui::test_support::mail_test_local_file_api(),
            },
            local_file_api: crate::ui::test_support::mail_test_local_file_api(),
            startup: crate::ui::surface::MailStartup::Fixture,
            theme: SurfaceTheme::default(),
            preview_width: 720.0,
            viewport_height: 640.0,
            window_controls_visible: false,
            chrome_top_inset: 0.0,
        },
        cx,
    );
    surface.mail_selected_tab_id = selected_tab_id;
    surface
}

fn assert_surface_on_drafts(cx: &TestAppContext, surface: &gpui::Entity<SurfaceState>) {
    cx.read_entity(surface, |surface, _cx| {
        assert_eq!(surface.mail_selected_tab_id, "drafts");
        assert_eq!(
            surface.mail_workspace().unwrap().selected_mailbox_id,
            "drafts"
        );
        assert!(surface.mail_workspace_api().is_some());
        assert_eq!(surface.mail_error, None);
    });
}

fn dispatch_mouse_down(
    window: &mut gpui::Window,
    app: &mut gpui::App,
    position: gpui::Point<gpui::Pixels>,
) {
    window.dispatch_event(
        MouseDownEvent {
            position,
            modifiers: Modifiers::none(),
            button: MouseButton::Left,
            click_count: 1,
            first_mouse: false,
        }
        .to_platform_input(),
        app,
    );
}

fn assert_new_composer_waiting_for_identity(
    cx: &mut TestAppContext,
    surface: &gpui::Entity<SurfaceState>,
) {
    cx.update_entity(surface, |surface, _cx| {
        assert_eq!(surface.mail_compose_mode, crate::ui::MailComposeMode::New);
        assert_eq!(surface.mail_compose_draft_id, None);
        assert_eq!(
            surface.mail_compose_error.as_deref(),
            Some("mail identity is still loading")
        );
    });
}

fn assert_new_composer_has_draft(cx: &mut TestAppContext, surface: &gpui::Entity<SurfaceState>) {
    cx.run_until_parked();
    cx.update_entity(surface, |surface, _cx| {
        assert_eq!(surface.mail_compose_mode, crate::ui::MailComposeMode::New);
        assert_eq!(surface.mail_compose_draft_id.as_deref(), Some("draft-new"));
        assert_eq!(surface.mail_compose_error, None);
    });
}

fn visible_thread_ids(cx: &TestAppContext, surface: &gpui::Entity<SurfaceState>) -> Vec<String> {
    cx.read_entity(surface, |surface, _cx| {
        surface
            .mail_list_threads
            .iter()
            .map(|thread| thread.id.to_string())
            .collect()
    })
}

fn paged_workspace_and_page() -> (MailWorkspace, MailMessagePage) {
    let mut workspace = test_workspace("inbox");
    workspace.messages = vec![test_message_at("inbox", 0), test_message_at("inbox", 1)];
    workspace.message_next_position = Some(2);
    let page = MailMessagePage {
        mailbox_id: "inbox".to_string(),
        position: 2,
        messages: vec![test_message_at("inbox", 2), test_message_at("inbox", 3)],
        next_position: Some(4),
    };
    (workspace, page)
}

fn recorded_page_requests(page_requests: &PageRequests) -> Vec<(String, usize)> {
    page_requests
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}
