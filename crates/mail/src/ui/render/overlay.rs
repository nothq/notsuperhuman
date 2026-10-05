//! Paint order for atomic inline boxes.
//!
//! CSS 2.1 Appendix E paints every in-flow block's background before any inline
//! content, so an `inline-block` badge that overflows its 15px wrapper lands
//! *on top of* the photo in the cell after it. GPUI paints in tree order, which
//! put the photo over the badge. Deferring the badge's paint restores the CSS
//! order; the clip mask is captured at prepaint so a deferred badge still stops
//! at the edge of a scrolled message like everything else does.

use gpui::{
    AnyElement, App, Bounds, Element, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    Pixels, Window,
};

pub(super) fn overlay(child: impl IntoElement) -> Overlay {
    Overlay {
        child: Some(child.into_any_element()),
    }
}

pub(super) struct Overlay {
    child: Option<AnyElement>,
}

impl Element for Overlay {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.as_mut().unwrap().request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let child = self.child.take().unwrap();
        let offset = window.element_offset();
        let mask = window.content_mask();
        window.defer_draw(child, offset, 0, Some(mask));
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        _window: &mut Window,
        _cx: &mut App,
    ) {
    }
}

impl IntoElement for Overlay {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}
