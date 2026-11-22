//! Linear progress bar with theme tokens and smooth width transition.

use nsh_base::{progress, theme};
use nsh_core::prelude::*;

use super::MOTION_BASE_MS;

/// A horizontal determinate progress bar (`0.0..=100.0`).
pub fn progress_bar<M>(
    value: impl Fn() -> f32 + Copy + 'static,
    accessibility_label: impl nsh_core::element::IntoProp<Option<SharedString>, M>,
) -> impl IntoElement {
    progress()
        .value(value)
        .accessibility_label(accessibility_label)
        .w_full()
        .track_style(|s| {
            s.w_full()
                .h(px(6.0))
                .rounded_full()
                .overflow_hidden()
                .bg(move || theme().muted)
        })
        .indicator_style(|s| {
            s.h_full()
                .rounded_full()
                .bg(move || theme().primary)
                .transition_all()
                .duration(MOTION_BASE_MS)
        })
}
