//! Styled buttons, icon actions, segmented icon controls, tooltips, and external links.

use nsh_base::{button as base_button, link, theme, tooltip};
use nsh_core::element::IntoTooltip;
use nsh_core::prelude::*;

use super::icons::svg_icon;
use super::{color, focus_ring, radius, MOTION_FAST_MS, MOTION_INSTANT_MS};

/// Semantic button emphasis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    /// The one committal action of a region.
    Primary,
    /// An ordinary visible action.
    Secondary,
    /// A quiet toolbar or inline action.
    Ghost,
    /// An irreversible destructive action (e.g. confirming deletion/uninstall).
    Destructive,
}

/// Visual emphasis for non-text icon buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
    /// Solid primary accent icon CTA (30pt toolbar scale).
    Primary,
    /// Bordered toolbar icon button (30pt scale).
    Secondary,
    /// Bordered destructive toolbar icon button (30pt scale).
    ToolbarDanger,
    /// Bordered toolbar launcher button (30pt scale).
    ToolbarLaunch,
    /// Compact bordered launcher button (26pt row scale).
    Launch,
    /// Borderless utility action (export, refresh, dismiss).
    Subtle,
    /// Borderless destructive trigger.
    Danger,
}

/// Convenience alias for [`action`].
pub fn button(
    kind: ButtonKind,
    label: impl IntoReactive<SharedString>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    action(kind, label, on_press)
}

/// A labeled action button (32pt desktop scale), enabled.
pub fn action(
    kind: ButtonKind,
    label: impl IntoReactive<SharedString>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    build_action(kind, label, None, false, || false, || false, on_press)
}

/// A labeled action button (32pt desktop scale), disabled while `disabled` reads true.
pub fn action_disabled(
    kind: ButtonKind,
    label: impl IntoReactive<SharedString>,
    disabled: impl Fn() -> bool + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    build_action(kind, label, None, false, disabled, || false, on_press)
}

/// A labeled action button (32pt desktop scale) that marks itself busy for screen readers
/// and disables activation while `busy` reads true.
pub fn action_busy(
    kind: ButtonKind,
    label: impl IntoReactive<SharedString>,
    busy: impl Fn() -> bool + Copy + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    build_action(kind, label, None, false, busy, busy, on_press)
}

/// A compact labeled action button (26pt row scale), enabled.
pub fn action_sm(
    kind: ButtonKind,
    label: impl IntoReactive<SharedString>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    build_action(kind, label, None, true, || false, || false, on_press)
}

/// A compact labeled action button (26pt row scale), disabled while `disabled` reads true.
pub fn action_sm_disabled(
    kind: ButtonKind,
    label: impl IntoReactive<SharedString>,
    disabled: impl Fn() -> bool + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    build_action(kind, label, None, true, disabled, || false, on_press)
}

/// A labeled action button with a leading 14pt SVG icon.
pub fn action_with_icon(
    kind: ButtonKind,
    icon_markup: &'static str,
    label: impl IntoReactive<SharedString>,
    disabled: impl Fn() -> bool + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    build_action(
        kind,
        label,
        Some(icon_markup),
        false,
        disabled,
        || false,
        on_press,
    )
}

fn build_action(
    kind: ButtonKind,
    label: impl IntoReactive<SharedString>,
    icon_markup: Option<&'static str>,
    compact: bool,
    disabled: impl Fn() -> bool + 'static,
    busy: impl Fn() -> bool + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let element = base_button()
        .on_press(on_press)
        .disabled(disabled)
        .busy(busy)
        .h(px(if compact { 26.0 } else { 32.0 }))
        .px(px(if compact { 10.0 } else { 12.0 }))
        .flex_row()
        .items_center()
        .justify_center()
        .gap(px(if compact { 4.0 } else { 6.0 }))
        .rounded(radius)
        .transition_transform()
        .duration(MOTION_FAST_MS)
        .active(|s| s.scale(0.97))
        .focus_visible(focus_ring)
        .disabled_style(|s| s.opacity(0.4));

    let label_view = |fg: fn() -> AdaptiveColor, medium: bool| {
        let t = text(label).text_color(fg);
        let t = if compact { t.text_xs() } else { t.text_sm() };
        if medium {
            t.font_weight(FontWeight::MEDIUM)
        } else {
            t
        }
    };
    let icon_size = if compact { 12.0 } else { 14.0 };

    match kind {
        ButtonKind::Primary => element
            .bg(move || theme().primary)
            .hover(|s| s.opacity(0.9))
            .active(|s| s.opacity(0.85))
            .child(icon_markup.map(|m| svg_icon(m, icon_size, || theme().primary_foreground)))
            .child(label_view(|| theme().primary_foreground, true)),
        ButtonKind::Secondary => element
            .border_1()
            .border_color(move || theme().border)
            .bg(move || theme().background)
            .hover(|s| s.bg(move || theme().accent))
            .active(|s| s.bg(move || theme().muted))
            .child(icon_markup.map(|m| svg_icon(m, icon_size, || theme().foreground)))
            .child(label_view(|| theme().foreground, false)),
        ButtonKind::Ghost => element
            .hover(|s| s.bg(move || theme().accent))
            .active(|s| s.bg(move || theme().muted))
            .child(icon_markup.map(|m| svg_icon(m, icon_size, || theme().muted_foreground)))
            .child(label_view(|| theme().muted_foreground, false)),
        ButtonKind::Destructive => element
            .bg(move || theme().destructive)
            .hover(|s| s.opacity(0.9))
            .active(|s| s.opacity(0.85))
            .child(icon_markup.map(|m| svg_icon(m, icon_size, || color("destructive-foreground"))))
            .child(label_view(|| color("destructive-foreground"), true)),
    }
}

/// A desktop hover/focus tooltip following the popover surface tokens and
/// subtle fade/scale entrance/exit motion (automatically skipped when `reduce_motion` is set).
pub fn hint_tooltip(label: impl IntoReactive<SharedString>) -> impl IntoTooltip {
    tooltip()
        .delay(250.0)
        .transition_all()
        .duration(MOTION_INSTANT_MS)
        .starting_style(|s| s.opacity(0.0).scale(0.95))
        .ending_style(|s| s.opacity(0.0).scale(0.95))
        .child(
            view()
                .px_2()
                .py_1()
                .rounded(radius)
                .border_1()
                .border_color(move || theme().border)
                .bg(move || theme().popover)
                .shadow_sm()
                .child(
                    text(label)
                        .text_xs()
                        .text_color(move || theme().popover_foreground),
                ),
        )
}

/// A non-text vector icon action with screen-reader label and hover tooltip.
pub fn icon_action(
    kind: IconKind,
    markup: &'static str,
    label: &'static str,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    icon_action_with_tip(kind, markup, label, label, || false, on_press)
}

/// A non-text vector icon action that disables cleanly while `disabled` reads true.
pub fn icon_action_disabled(
    kind: IconKind,
    markup: &'static str,
    label: &'static str,
    disabled: impl Fn() -> bool + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    icon_action_with_tip(kind, markup, label, label, disabled, on_press)
}

/// A non-text vector icon action with separate reactive tooltip text.
pub fn icon_action_with_tip<M>(
    kind: IconKind,
    markup: &'static str,
    a11y_label: impl nsh_core::element::IntoProp<Option<SharedString>, M>,
    tooltip_label: impl IntoReactive<SharedString>,
    disabled: impl Fn() -> bool + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    icon_action_spinning_with_tip(
        kind,
        markup,
        a11y_label,
        tooltip_label,
        disabled,
        || false,
        on_press,
    )
}

/// A non-text vector icon action whose inner icon spins while `spinning` reads true.
pub fn icon_action_spinning_with_tip<M>(
    kind: IconKind,
    markup: &'static str,
    a11y_label: impl nsh_core::element::IntoProp<Option<SharedString>, M>,
    tooltip_label: impl IntoReactive<SharedString>,
    disabled: impl Fn() -> bool + 'static,
    spinning: impl Fn() -> bool + Copy + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let size = match kind {
        IconKind::Primary
        | IconKind::Secondary
        | IconKind::ToolbarDanger
        | IconKind::ToolbarLaunch => 30.0,
        IconKind::Launch | IconKind::Subtle | IconKind::Danger => 26.0,
    };
    let icon_size = match kind {
        IconKind::Primary
        | IconKind::Secondary
        | IconKind::ToolbarDanger
        | IconKind::ToolbarLaunch => 15.0,
        IconKind::Launch | IconKind::Subtle | IconKind::Danger => 14.0,
    };
    let render_icon = move |color_fn: fn() -> AdaptiveColor| {
        view()
            .size(px(icon_size))
            .items_center()
            .justify_center()
            .style_when(spinning, |s| s.animate_spin())
            .child(svg_icon(markup, icon_size, color_fn))
    };
    let base = base_button()
        .on_press(on_press)
        .disabled(disabled)
        .busy(spinning)
        .accessibility_label(a11y_label)
        .w(px(size))
        .h(px(size))
        .items_center()
        .justify_center()
        .rounded(radius)
        .transition_transform()
        .duration(MOTION_FAST_MS)
        .active(|s| s.scale(0.94))
        .focus_visible(focus_ring)
        .disabled_style(|s| s.opacity(0.4));

    let styled = match kind {
        IconKind::Primary => base
            .bg(move || theme().primary)
            .hover(|s| s.opacity(0.9))
            .active(|s| s.opacity(0.85))
            .child(render_icon(|| theme().primary_foreground)),
        IconKind::Secondary => base
            .border_1()
            .border_color(move || theme().border)
            .bg(move || theme().background)
            .hover(|s| s.bg(move || theme().accent).border_color(move || theme().ring))
            .active(|s| s.bg(move || theme().muted))
            .child(render_icon(|| theme().foreground)),
        IconKind::ToolbarDanger => base
            .border_1()
            .border_color(move || theme().border)
            .bg(move || theme().background)
            .hover(|s| {
                s.bg(move || theme().accent)
                    .border_color(move || theme().destructive)
            })
            .active(|s| s.bg(move || theme().muted))
            .child(render_icon(|| theme().destructive)),
        IconKind::ToolbarLaunch | IconKind::Launch => base
            .border_1()
            .border_color(move || theme().border)
            .bg(move || theme().secondary)
            .hover(|s| s.bg(move || theme().accent).border_color(move || theme().ring))
            .active(|s| s.bg(move || theme().muted))
            .child(render_icon(|| theme().foreground)),
        IconKind::Subtle => base
            .hover(|s| s.bg(move || theme().accent))
            .active(|s| s.bg(move || theme().muted))
            .child(render_icon(|| theme().muted_foreground)),
        IconKind::Danger => base
            .hover(|s| s.bg(move || theme().muted))
            .active(|s| s.bg(move || theme().accent))
            .child(render_icon(|| theme().destructive)),
    };

    view()
        .flex_shrink_0()
        .tooltip(hint_tooltip(tooltip_label))
        .child(styled)
}

/// One segment of a compact non-text segmented control (e.g. appearance modes).
pub fn segmented_icon(
    active: impl Fn() -> bool + Copy + 'static,
    markup: &'static str,
    label: &'static str,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    view()
        .flex_shrink_0()
        .tooltip(hint_tooltip(label))
        .child(
            base_button()
                .on_press(on_press)
                .selected(active)
                .accessibility_label(label)
                .w(px(26.0))
                .h(px(24.0))
                .items_center()
                .justify_center()
                .rounded(px(6.0))
                .style_when(active, |s| s.bg(move || theme().accent))
                .hover(|s| s.bg(move || theme().secondary))
                .active(|s| s.scale(0.95))
                .transition_transform()
                .duration(MOTION_FAST_MS)
                .focus_visible(focus_ring)
                .child(svg_icon(markup, 14.0, move || {
                    if active() {
                        theme().accent_foreground
                    } else {
                        theme().muted_foreground
                    }
                })),
        )
}

/// An external hyperlink styled with the primary accent and focus ring.
pub fn external_link<M>(
    label: impl IntoReactive<SharedString>,
    href: impl nsh_core::element::IntoProp<Option<SharedString>, M>,
) -> impl IntoElement {
    link()
        .href(href)
        .flex_row()
        .items_center()
        .gap_1()
        .rounded(px(4.0))
        .focus_visible(focus_ring)
        .hover(|s| s.opacity(0.8))
        .active(|s| s.opacity(0.65))
        .child(
            text(label)
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || theme().primary),
        )
        .child(svg_icon(super::icons::EXTERNAL_LINK, 12.0, move || {
            theme().primary
        }))
}

/// A button-styled external hyperlink (e.g. primary release download CTA).
pub fn external_link_button<M>(
    kind: ButtonKind,
    label: impl IntoReactive<SharedString>,
    href: impl nsh_core::element::IntoProp<Option<SharedString>, M>,
) -> impl IntoElement {
    let base = link()
        .href(href)
        .h_8()
        .px_3()
        .flex_row()
        .items_center()
        .justify_center()
        .gap_1p5()
        .rounded(radius)
        .transition_transform()
        .duration(MOTION_FAST_MS)
        .active(|s| s.scale(0.97))
        .focus_visible(focus_ring);
    match kind {
        ButtonKind::Primary => base
            .bg(move || theme().primary)
            .hover(|s| s.opacity(0.9))
            .active(|s| s.opacity(0.85))
            .child(
                text(label)
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(move || theme().primary_foreground),
            )
            .child(svg_icon(super::icons::EXTERNAL_LINK, 13.0, move || {
                theme().primary_foreground
            })),
        _ => base
            .border_1()
            .border_color(move || theme().border)
            .bg(move || theme().background)
            .hover(|s| s.bg(move || theme().accent))
            .active(|s| s.bg(move || theme().muted))
            .child(text(label).text_sm().text_color(move || theme().foreground))
            .child(svg_icon(super::icons::EXTERNAL_LINK, 13.0, move || {
                theme().foreground
            })),
    }
}
