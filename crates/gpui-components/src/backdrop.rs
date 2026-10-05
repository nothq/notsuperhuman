use std::rc::Rc;

use gpui::{
    App, ClickEvent, Context, Div, ElementId, InteractiveElement, MouseButton, Stateful,
    StatefulInteractiveElement, Window,
};

type BackdropClickHandler<V> =
    Rc<dyn Fn(&mut V, &ClickEvent, &mut Window, &mut Context<V>) + 'static>;
type AppBackdropClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// A stable backdrop identity paired with the click that dismisses it.
///
/// Primary clicks dismiss by default. Additional mouse buttons can be enabled
/// explicitly while preserving GPUI's matched press-and-release semantics.
pub struct BackdropDismissal<V: 'static> {
    id: ElementId,
    mouse_buttons: Vec<MouseButton>,
    handler: BackdropClickHandler<V>,
}

impl<V: 'static> BackdropDismissal<V> {
    /// Creates a dismissal that responds to matched primary clicks.
    ///
    /// The caller location provides an identity that is stable across redraws.
    /// Repeated elements rendered from one callsite must use [`Self::keyed`].
    #[track_caller]
    pub fn new(
        handler: impl Fn(&mut V, &ClickEvent, &mut Window, &mut Context<V>) + 'static,
    ) -> Self {
        Self::with_id(
            ElementId::CodeLocation(*core::panic::Location::caller()),
            handler,
        )
    }

    /// Creates a dismissal for a repeated or dynamically identified backdrop.
    pub fn keyed(
        key: ElementId,
        handler: impl Fn(&mut V, &ClickEvent, &mut Window, &mut Context<V>) + 'static,
    ) -> Self {
        Self::with_id(key, handler)
    }

    fn with_id(
        id: ElementId,
        handler: impl Fn(&mut V, &ClickEvent, &mut Window, &mut Context<V>) + 'static,
    ) -> Self {
        Self {
            id,
            mouse_buttons: vec![MouseButton::Left],
            handler: Rc::new(handler),
        }
    }
}

/// Applies a blocking hitbox and dismisses only from a matched GPUI click.
///
/// Press and release events for every configured button are consumed. The
/// dismissal is deferred until `on_click`/`on_aux_click`, so the backdrop stays
/// mounted through the release that completes the gesture.
pub fn dismissible_backdrop<V: 'static>(
    backdrop: Div,
    dismissal: BackdropDismissal<V>,
    cx: &mut Context<V>,
) -> Stateful<Div> {
    let BackdropDismissal {
        id,
        mouse_buttons,
        handler,
    } = dismissal;
    let handler = cx.listener(move |this, event, window, cx| handler(this, event, window, cx));
    dismissible_backdrop_with_id(backdrop, id, mouse_buttons, Rc::new(handler))
}

fn dismissible_backdrop_with_id(
    backdrop: Div,
    id: ElementId,
    mouse_buttons: Vec<MouseButton>,
    handler: AppBackdropClickHandler,
) -> Stateful<Div> {
    let mut backdrop = backdrop.occlude().id(id);

    for button in mouse_buttons.iter().copied() {
        backdrop = backdrop
            .on_mouse_down(button, |_, window, cx| {
                window.prevent_default();
                cx.stop_propagation();
            })
            .on_mouse_up(button, |_, window, cx| {
                window.prevent_default();
                cx.stop_propagation();
            });
    }

    let primary_handler = Rc::clone(&handler);
    backdrop = backdrop.on_click(move |event, window, cx| {
        if !is_matched_mouse_click(event, MouseButton::Left) {
            return;
        }
        cx.stop_propagation();
        primary_handler(event, window, cx);
        cx.stop_propagation();
    });

    let auxiliary_buttons = mouse_buttons
        .into_iter()
        .filter(|button| *button != MouseButton::Left)
        .collect::<Vec<_>>();
    if !auxiliary_buttons.is_empty() {
        backdrop = backdrop.on_aux_click(move |event, window, cx| {
            let Some(button) = matched_mouse_button(event) else {
                return;
            };
            if !auxiliary_buttons.contains(&button) {
                return;
            }
            cx.stop_propagation();
            handler(event, window, cx);
            cx.stop_propagation();
        });
    }

    backdrop
}

fn matched_mouse_button(event: &ClickEvent) -> Option<MouseButton> {
    let ClickEvent::Mouse(event) = event else {
        return None;
    };
    (event.down.button == event.up.button).then_some(event.down.button)
}

fn is_matched_mouse_click(event: &ClickEvent, button: MouseButton) -> bool {
    matched_mouse_button(event) == Some(button)
}
