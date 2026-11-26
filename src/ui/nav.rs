//! Navigation controls: sidebar navigation rows with dynamic count badges and filter chips.

use nsh_base::{button, theme};
use nsh_core::prelude::*;

use super::icons::svg_icon;
use super::{focus_ring, radius, MOTION_FAST_MS};

/// A desktop sidebar navigation item with icon, label, and optional reactive count badge.
pub fn sidebar_nav_item(
    selected: impl Fn() -> bool + Copy + 'static,
    icon_markup: &'static str,
    label: &'static str,
    badge_count: impl Fn() -> Option<usize> + Copy + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    sidebar_nav_item_reactive(selected, icon_markup, label, badge_count, on_press)
}

/// A desktop sidebar navigation item accepting a reactive `label` (e.g. for live locale changes).
pub fn sidebar_nav_item_reactive(
    selected: impl Fn() -> bool + Copy + 'static,
    icon_markup: &'static str,
    label: impl IntoReactive<SharedString>,
    badge_count: impl Fn() -> Option<usize> + Copy + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let label = label.into_reactive();
    let a11y_label = label.clone().map(Some);
    button()
        .on_press(on_press)
        .selected(selected)
        .accessibility_label(a11y_label)
        .w_full()
        .h(px(34.0))
        .px(px(10.0))
        .flex_row()
        .items_center()
        .gap_2()
        .rounded(radius)
        .style_when(selected, |s| s.bg(move || theme().accent))
        .hover(move |s| {
            s.bg(move || {
                if selected() {
                    theme().accent
                } else {
                    theme().muted
                }
            })
        })
        .active(|s| s.scale(0.98))
        .transition_transform()
        .duration(MOTION_FAST_MS)
        .focus_visible(focus_ring)
        .child(svg_icon(icon_markup, 15.0, move || {
            if selected() {
                theme().accent_foreground
            } else {
                theme().muted_foreground
            }
        }))
        .child(
            text(label)
                .flex_1()
                .min_w(px(0.0))
                .number_of_lines(1)
                .text_sm()
                .style_with(move |s| {
                    if selected() {
                        s.font_weight(FontWeight::MEDIUM)
                            .text_color(move || theme().accent_foreground)
                    } else {
                        s.text_color(move || theme().sidebar_foreground)
                    }
                }),
        )
        .child(when(
            move || badge_count().is_some_and(|n| n > 0),
            move || {
                view()
                    .flex_shrink_0()
                    .px(px(6.0))
                    .h(px(18.0))
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(move || {
                        if selected() {
                            theme().primary.opacity(0.16)
                        } else {
                            theme().muted
                        }
                    })
                    .child(
                        text(move || {
                            badge_count()
                                .map(|n| n.to_string())
                                .unwrap_or_default()
                        })
                        .text_xs()
                        .tabular_nums()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(move || {
                            if selected() {
                                theme().primary
                            } else {
                                theme().muted_foreground
                            }
                        }),
                    )
            },
        ))
}

/// A selectable pill-shaped filter chip (e.g. bucket filter in Discover view).
pub fn filter_chip(
    selected: impl Fn() -> bool + Copy + 'static,
    label: impl IntoReactive<SharedString>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    button()
        .on_press(on_press)
        .selected(selected)
        .h(px(26.0))
        .px(px(10.0))
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .rounded_full()
        .border_1()
        .border_color(move || if selected() { theme().primary } else { theme().border })
        .bg(move || {
            if selected() {
                theme().accent
            } else {
                theme().background
            }
        })
        .hover(move |s| {
            s.bg(move || {
                if selected() {
                    theme().accent
                } else {
                    theme().muted
                }
            })
        })
        .active(|s| s.scale(0.97))
        .transition_transform()
        .duration(MOTION_FAST_MS)
        .focus_visible(focus_ring)
        .child(
            text(label)
                .text_xs()
                .style_with(move |s| {
                    if selected() {
                        s.font_weight(FontWeight::MEDIUM)
                            .text_color(move || theme().accent_foreground)
                    } else {
                        s.text_color(move || theme().muted_foreground)
                    }
                }),
        )
}
