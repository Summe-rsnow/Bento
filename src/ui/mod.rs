//! Styled UI layer built on `nsh_base` using 100% semantic `theme()` tokens.
//!
//! Apps compose these controls rather than styling `nsh_base` primitives directly.

pub mod button;
pub mod icons;
pub mod input;
pub mod nav;
pub mod progress;
pub mod surface;

pub use button::{
    action, action_disabled, action_sm, action_sm_disabled, action_with_icon, button,
    external_link, external_link_button, hint_tooltip, icon_action, icon_action_disabled,
    icon_action_spinning_with_tip, icon_action_with_tip, segmented_icon, ButtonKind, IconKind,
};
pub use icons::{png_data_uri, svg_icon};
pub use input::{checkbox, radio_option, search_input, text_field, textarea, toggle_switch};
pub use nav::{filter_chip, sidebar_nav_item};
pub use progress::progress_bar;
pub use surface::{
    card, divider, empty_state, modal_dialog, notice_toast, progress_modal_dialog, section_header,
    status_badge, toolbar_separator,
};

use nsh_base::theme;
use nsh_core::element::StyleRefinement;
use nsh_core::prelude::*;

/// Shared motion duration tokens (in milliseconds), aligned with NSH Design Guides.
pub const MOTION_INSTANT_MS: f32 = 120.0;
pub const MOTION_FAST_MS: f32 = 150.0;
pub const MOTION_BASE_MS: f32 = 240.0;
pub const MOTION_SLOW_MS: f32 = 360.0;

/// Reads a named semantic theme color from `theme.json`, falling back to
/// `theme().foreground` rather than panicking if a custom token is missing.
pub fn color(name: &'static str) -> AdaptiveColor {
    theme().color(name).unwrap_or_else(|| theme().foreground)
}

/// The single shared corner radius for controls and surfaces (`theme().radius`).
pub fn radius() -> Length {
    theme().radius.into()
}

/// Semantic tone for status values, badges, notices, and progress bars.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueTone {
    Ok,
    Warn,
    Bad,
    Info,
    Neutral,
}

impl ValueTone {
    /// Classifies a normalized ratio (`0.0..=1.0`): > 0.5 is healthy, >= 0.2 is warning, else low.
    pub fn of(remaining: f32) -> Self {
        if remaining > 0.5 {
            Self::Ok
        } else if remaining >= 0.2 {
            Self::Warn
        } else {
            Self::Bad
        }
    }

    /// Maps the tone to its semantic theme color.
    pub fn color(self) -> AdaptiveColor {
        match self {
            Self::Ok => color("success"),
            Self::Warn => color("warning"),
            Self::Bad => theme().destructive,
            Self::Info => theme().primary,
            Self::Neutral => theme().muted_foreground,
        }
    }
}

/// Standard outward focus ring shown on keyboard focus (`focus_visible`).
pub fn focus_ring(s: StyleRefinement) -> StyleRefinement {
    s.outline(px(2.0)).outline_color(move || theme().ring)
}
