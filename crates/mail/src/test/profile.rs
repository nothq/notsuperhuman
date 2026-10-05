use std::time::{Duration, Instant};

use gpui::{px, AppContext, TestAppContext};

mod fixtures;

use crate::model::{MailAddress, MailMessage, MailThread, MailWorkspace};
use crate::ui::{
    AppearanceMode, MailStartup, MailSurfaceStateConfig, SurfaceInput, SurfaceState, SurfaceTheme,
};
use fixtures::test_workspace;

const SIXTY_HZ_FRAME_BUDGET_MS: f64 = 16.7;
const THIRTY_HZ_FRAME_BUDGET_MS: f64 = 33.3;

#[gpui::test]
#[ignore]
fn profile_mail_list_scroll_update_loop(cx: &mut TestAppContext) {
    let thread_count = crate::test::mail_profile_count("NOTSUPERHUMAN_MAIL_PROFILE_THREADS", 2_000);
    let frame_count = crate::test::mail_profile_count("NOTSUPERHUMAN_MAIL_PROFILE_FRAMES", 120);
    let workspace = mail_profile_workspace(thread_count);

    eprintln!("profiling mail list with {thread_count} threads");

    let (surface, cx) =
        cx.add_window_view(|_, cx| mail_profile_surface_state(workspace.clone(), false, cx));
    cx.refresh().expect("refresh mail list profile window");

    let start = Instant::now();
    let mut frame_durations = Vec::with_capacity(frame_count);
    for step in 0..frame_count {
        let direction = if step % 40 < 20 { 1.0 } else { -1.0 };
        let frame_start = Instant::now();
        cx.update_entity(&surface, |surface, cx| {
            surface.mail_list_state.scroll_by(px(96.0 * direction));
            cx.notify();
        });
        cx.run_until_parked();
        frame_durations.push(frame_start.elapsed());
    }
    let elapsed = start.elapsed();

    eprintln!(
        "{}",
        summarize_profile_frames("mail-list-scroll-update", &frame_durations, elapsed)
    );
}

#[gpui::test]
#[ignore]
fn profile_mail_open_thread_body_scroll_update_loop(cx: &mut TestAppContext) {
    let section_count =
        crate::test::mail_profile_count("NOTSUPERHUMAN_MAIL_PROFILE_BODY_SECTIONS", 120);
    let frame_count = crate::test::mail_profile_count("NOTSUPERHUMAN_MAIL_PROFILE_FRAMES", 120);
    let workspace = mail_open_thread_profile_workspace(section_count);

    eprintln!("profiling open mail body scroll with {section_count} rich sections");

    let (surface, cx) =
        cx.add_window_view(|_, cx| mail_profile_surface_state(workspace.clone(), true, cx));
    cx.refresh().expect("refresh open mail body profile window");

    let start = Instant::now();
    let mut frame_durations = Vec::with_capacity(frame_count);
    for step in 0..frame_count {
        let direction = if step % 40 < 20 { 1.0 } else { -1.0 };
        let frame_start = Instant::now();
        cx.update_entity(&surface, |surface, cx| {
            surface
                .mail_open_thread_body_list_state
                .scroll_by(px(160.0 * direction));
            cx.notify();
        });
        cx.run_until_parked();
        frame_durations.push(frame_start.elapsed());
    }
    let elapsed = start.elapsed();

    eprintln!(
        "{}",
        summarize_profile_frames("mail-open-body-scroll-update", &frame_durations, elapsed)
    );
}

#[gpui::test]
#[ignore]
fn profile_mail_open_to_list_render(cx: &mut TestAppContext) {
    let thread_count = crate::test::mail_profile_count("NOTSUPERHUMAN_MAIL_PROFILE_THREADS", 2_000);
    let run_count = crate::test::mail_profile_count("NOTSUPERHUMAN_MAIL_OPEN_PROFILE_RUNS", 20);
    let workspace = mail_profile_workspace(thread_count);
    let mut durations = Vec::with_capacity(run_count);

    eprintln!("profiling mail open-to-list render with {thread_count} threads");

    for _ in 0..run_count {
        let start = Instant::now();
        let (_surface, window_cx) =
            cx.add_window_view(|_, cx| mail_profile_surface_state(workspace.clone(), false, cx));
        window_cx
            .refresh()
            .expect("refresh mail open-to-list profile window");
        window_cx.run_until_parked();
        durations.push(start.elapsed());
    }

    eprintln!(
        "{}",
        summarize_profile_samples("mail-open-to-list-render", &durations)
    );
}

fn mail_profile_surface_state(
    workspace: MailWorkspace,
    open_thread: bool,
    cx: &mut gpui::Context<SurfaceState>,
) -> SurfaceState {
    cx.set_global(AppearanceMode::Dark);
    let selected_thread_id = workspace
        .messages
        .first()
        .map(|message| message.thread_id.clone());
    let mut surface = SurfaceState::new(
        MailSurfaceStateConfig {
            input: SurfaceInput {
                workspace: Some(workspace.clone()),
                workspace_api: None,
                local_file_api: crate::ui::test_support::mail_test_local_file_api(),
            },
            local_file_api: crate::ui::test_support::mail_test_local_file_api(),
            startup: MailStartup::Fixture,
            theme: SurfaceTheme::default(),
            preview_width: 1280.0,
            viewport_height: 800.0,
            window_controls_visible: false,
            chrome_top_inset: 0.0,
        },
        cx,
    );
    surface.mail_thread_detail_cache.clear();
    surface.mail_open_thread_body_list_thread_id = None;
    surface.mail_selected_tab_id = workspace.selected_mailbox_id.clone();
    surface.sync_mail_list_rows();
    surface.mail_selected_thread_id = selected_thread_id.clone();
    surface.mail_open_thread_id = open_thread.then_some(selected_thread_id.clone()).flatten();
    if open_thread {
        let thread_id = selected_thread_id
            .clone()
            .expect("open thread profile requires a selected thread");
        let message = workspace
            .messages
            .iter()
            .find(|message| message.thread_id == thread_id)
            .cloned()
            .expect("open thread profile requires a message");
        surface.mail_thread_cache.insert(
            thread_id.clone(),
            MailThread {
                id: thread_id,
                messages: vec![message],
            },
        );
    }
    surface.ensure_mail_active_thread_detail_cached();
    surface.sync_mail_open_thread_body_list_state();
    surface
}

fn mail_profile_workspace(thread_count: usize) -> MailWorkspace {
    let mut workspace = test_workspace("inbox");
    workspace.messages = (0..thread_count)
        .map(profile_mail_message)
        .collect::<Vec<_>>();
    let inbox = workspace
        .mailboxes
        .iter_mut()
        .find(|mailbox| mailbox.id == "inbox")
        .expect("profile workspace should include inbox mailbox");
    inbox.total_emails = thread_count as u64;
    inbox.unread_emails = thread_count.min(99) as u64;
    workspace
}

fn mail_open_thread_profile_workspace(section_count: usize) -> MailWorkspace {
    let mut workspace = test_workspace("inbox");
    let mut message = profile_mail_message(0);
    message.subject = "Rich invoice body scroll profile".to_string();
    message.preview = "Synthetic rich HTML mail body used for GPUI scroll profiling.".to_string();
    message.body_text = "Synthetic rich HTML mail body used for GPUI scroll profiling.".to_string();
    message.body_html = Some(rich_profile_mail_html(section_count));
    workspace.messages = vec![message];
    let inbox = workspace
        .mailboxes
        .iter_mut()
        .find(|mailbox| mailbox.id == "inbox")
        .expect("profile workspace should include inbox mailbox");
    inbox.total_emails = 1;
    inbox.unread_emails = 0;
    workspace
}

fn profile_mail_message(index: usize) -> MailMessage {
    let mut message = test_workspace("inbox")
        .messages
        .into_iter()
        .next()
        .expect("mail profile seed message should exist");
    let from = MailAddress {
        name: "notsuperhuman".to_string(),
        email: "ops@example.com".to_string(),
    };
    let to = MailAddress {
        name: "Alex Rivera".to_string(),
        email: "alex@example.com".to_string(),
    };
    message.id = format!("profile-message-{index}");
    message.thread_id = format!("profile-thread-{index}");
    message.message_id = vec![format!("<profile-message-{index}@example.com>")];
    message.subject = format!("Profile thread {index}");
    message.preview =
        format!("Synthetic preview body for mail thread {index} used in GPUI profiling.");
    message.received_at = "2026-04-16T15:05:00Z".to_string();
    message.sender = vec![from.clone()];
    message.from = vec![from];
    message.to = vec![to];
    message.mailbox_ids = vec!["inbox".to_string()];
    message.has_attachment = false;
    message.is_unread = true;
    message.is_draft = false;
    message.body_text = message.preview.clone();
    message.body_html = Some(format!("<p>{}</p>", message.preview));
    message.body_loaded = true;
    message
}

fn rich_profile_mail_html(section_count: usize) -> String {
    let mut html = String::from(
        r#"<div style="width:640px;background:#ffffff;padding:28px;font-family:Arial;color:#242832;">
        <table width="100%" cellpadding="0" cellspacing="0" style="border-collapse:collapse;">
        <tr><td style="font-size:22px;line-height:28px;font-weight:700;">Invoice profile body</td></tr>
        <tr><td style="height:18px;"></td></tr>"#,
    );
    for index in 0..section_count {
        html.push_str(&format!(
            r#"<tr>
                <td style="padding:12px 0;border-top:1px solid #e7ebf0;">
                    <table width="100%" cellpadding="0" cellspacing="0" style="border-collapse:collapse;">
                        <tr>
                            <td width="72" style="font-size:12px;line-height:18px;color:#667085;">#{index:03}</td>
                            <td style="font-size:14px;line-height:20px;color:#242832;">Render and scroll section {index}</td>
                            <td width="96" align="right" style="font-size:14px;line-height:20px;color:#242832;">${index}.00</td>
                        </tr>
                        <tr>
                            <td></td>
                            <td colspan="2" style="font-size:12px;line-height:18px;color:#667085;padding-top:4px;">
                                Nested table text, inherited styles, and row spacing exercise the rich mail renderer.
                            </td>
                        </tr>
                    </table>
                </td>
            </tr>"#,
        ));
    }
    html.push_str("</table></div>");
    html
}

fn summarize_profile_frames(
    label: &str,
    frame_durations: &[Duration],
    total_elapsed: Duration,
) -> String {
    let mut frame_millis = frame_durations
        .iter()
        .map(Duration::as_secs_f64)
        .map(|seconds| seconds * 1000.0)
        .collect::<Vec<_>>();
    frame_millis.sort_by(|left, right| left.total_cmp(right));
    let frame_count = frame_millis.len();
    let mean_ms = if frame_count == 0 {
        0.0
    } else {
        frame_millis.iter().sum::<f64>() / frame_count as f64
    };
    let spikes_over_60hz_budget = frame_millis
        .iter()
        .filter(|millis| **millis > SIXTY_HZ_FRAME_BUDGET_MS)
        .count();
    let spikes_over_30hz_budget = frame_millis
        .iter()
        .filter(|millis| **millis > THIRTY_HZ_FRAME_BUDGET_MS)
        .count();
    format!(
        "captured {frame_count} {label} frames in {:?} (mean {:.2} ms, p50 {:.2} ms, p95 {:.2} ms, p99 {:.2} ms, max {:.2} ms, spikes >{:.1} ms: {}, >{:.1} ms: {})",
        total_elapsed,
        mean_ms,
        percentile_millis(&frame_millis, 0.50),
        percentile_millis(&frame_millis, 0.95),
        percentile_millis(&frame_millis, 0.99),
        frame_millis.last().copied().unwrap_or(0.0),
        SIXTY_HZ_FRAME_BUDGET_MS,
        spikes_over_60hz_budget,
        THIRTY_HZ_FRAME_BUDGET_MS,
        spikes_over_30hz_budget,
    )
}

fn summarize_profile_samples(label: &str, durations: &[Duration]) -> String {
    let mut millis = durations
        .iter()
        .map(Duration::as_secs_f64)
        .map(|seconds| seconds * 1000.0)
        .collect::<Vec<_>>();
    millis.sort_by(|left, right| left.total_cmp(right));
    let sample_count = millis.len();
    let mean_ms = if sample_count == 0 {
        0.0
    } else {
        millis.iter().sum::<f64>() / sample_count as f64
    };
    format!(
        "captured {sample_count} {label} samples (mean {:.2} ms, p50 {:.2} ms, p95 {:.2} ms, max {:.2} ms)",
        mean_ms,
        percentile_millis(&millis, 0.50),
        percentile_millis(&millis, 0.95),
        millis.last().copied().unwrap_or(0.0),
    )
}

fn percentile_millis(sorted_millis: &[f64], percentile: f64) -> f64 {
    if sorted_millis.is_empty() {
        return 0.0;
    }
    let rank = (percentile.clamp(0.0, 1.0) * sorted_millis.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted_millis.len() - 1);
    sorted_millis[index]
}
