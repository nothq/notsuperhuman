use gpui::{AppContext, TestAppContext};

use super::{mail_surface_state, support::test_api};

#[gpui::test]
fn submitted_mail_search_keeps_input_focus(cx: &mut TestAppContext) {
    let surface = cx.new(|cx| mail_surface_state(Some(test_api()), cx));
    cx.update_entity(&surface, |surface, cx| {
        assert!(surface.open_mail_search(cx));
        surface.set_mail_search_input("no matching conversation".to_string(), cx);
        assert!(surface.submit_mail_search(cx));
        assert!(surface.mail_search_input_focused());
        assert!(!surface.mail_shortcuts_focused);
    });
    cx.run_until_parked();
    cx.read_entity(&surface, |surface, _cx| {
        assert!(surface.mail_search_input_focused());
        assert!(!surface.mail_shortcuts_focused);
    });
}
