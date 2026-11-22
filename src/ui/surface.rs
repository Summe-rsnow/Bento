//! Surfaces, badges, hairlines, empty states, floating toasts, and modal dialogs.

use nsh_base::{dialog, theme};
use nsh_core::prelude::*;

use super::button::{action, icon_action, ButtonKind, IconKind};
use super::{color, icons, radius, ValueTone};

/// A grouped flat card surface with a hairline border and zero base shadow.
pub fn card() -> View {
    view()
        .px_3()
        .py(px(10.0))
        .gap_2()
        .rounded(radius)
        .border_1()
        .border_color(move || theme().border)
        .bg(move || theme().card)
}

/// A horizontal 1pt hairline divider between regions.
pub fn divider() -> impl IntoElement {
    view().h(px(1.0)).bg(move || theme().border)
}

/// A vertical 1pt hairline separator for toolbars and title bars.
pub fn toolbar_separator() -> impl IntoElement {
    view()
        .w(px(1.0))
        .h(px(16.0))
        .mx(px(2.0))
        .bg(move || theme().border)
}

/// A section heading with `AccessibilityRole::Header` for screen-reader navigation.
pub fn section_header(title: impl IntoReactive<SharedString>) -> impl IntoElement {
    text(title)
        .role(AccessibilityRole::Header)
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(move || theme().foreground)
}

/// Centered empty state describing what the view is waiting for and the next step.
pub fn empty_state(
    title: impl IntoReactive<SharedString>,
    hint: impl IntoReactive<SharedString>,
) -> impl IntoElement {
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .gap_2()
        .p_8()
        .child(
            text(title)
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || theme().foreground),
        )
        .child(
            text(hint)
                .text_xs()
                .text_color(move || theme().muted_foreground),
        )
}

/// A short semantic status pill: state is always named in words, never color alone.
pub fn status_badge(
    label: impl IntoReactive<SharedString>,
    tone: ValueTone,
) -> impl IntoElement {
    view()
        .flex_shrink_0()
        .px_2()
        .h(px(20.0))
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(move || tone.color().opacity(0.16))
        .child(text(label).text_xs().text_color(move || tone.color()))
}

/// A floating bottom feedback toast that occupies zero layout space in the main column.
pub fn notice_toast(
    message: impl Fn() -> String + 'static,
    dot_color: impl Fn() -> AdaptiveColor + Copy + 'static,
    on_close: impl Fn() + 'static,
) -> impl IntoElement {
    view()
        .pointer_events(PointerEvents::Auto)
        .flex_row()
        .items_center()
        .gap_2()
        .pl_3()
        .pr_1()
        .py_1()
        .max_w(relative(1.0))
        .rounded(radius)
        .border_1()
        .border_color(move || theme().border)
        .bg(move || theme().card)
        .shadow_md()
        .child(view().size_2().flex_shrink_0().rounded_full().bg(dot_color))
        .child(
            text(message)
                .min_w(px(0.0))
                .flex_shrink(1.0)
                .text_xs()
                .text_color(move || theme().foreground),
        )
        .child(icon_action(IconKind::Subtle, icons::CLOSE, "关闭提示", on_close))
}

/// Shared modal dialog frame: theme scrim backdrop + centered card with
/// consistent border, radius, elevation, Escape/backdrop cancel, and FadeInUp motion.
pub fn modal_dialog(
    max_width: f32,
    on_cancel: impl Fn() + 'static,
    content: impl IntoElement,
) -> impl IntoElement {
    let enter = motion().entering(MotionPreset::FadeInUp);
    let card_view = view()
        .bind_motion(&enter)
        .w_full()
        .max_w(px(max_width))
        .mx_4()
        .gap_4()
        .p_6()
        .rounded(radius)
        .border_1()
        .border_color(move || theme().border)
        .bg(move || theme().card)
        .shadow_lg()
        .child(content);

    dialog()
        .on_cancel(on_cancel)
        .flex_1()
        .items_center()
        .justify_center()
        .bg(move || color("scrim"))
        .child(card_view)
}

/// Non-dismissible modal dialog frame for in-flight progress (e.g. batch import).
pub fn progress_modal_dialog(max_width: f32, content: impl IntoElement) -> impl IntoElement {
    let enter = motion().entering(MotionPreset::FadeInUp);
    let card_view = view()
        .bind_motion(&enter)
        .w_full()
        .max_w(px(max_width))
        .mx_4()
        .gap_4()
        .p_6()
        .rounded(radius)
        .border_1()
        .border_color(move || theme().border)
        .bg(move || theme().card)
        .shadow_lg()
        .child(content);

    dialog()
        .flex_1()
        .items_center()
        .justify_center()
        .bg(move || color("scrim"))
        .child(card_view)
}

/// A two-column metadata row with a fixed muted label on the left and a value element on the right.
pub fn meta_row(label: &'static str, value: impl IntoElement) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            text(label)
                .flex_shrink_0()
                .text_xs()
                .text_color(move || theme().muted_foreground),
        )
        .child(value)
}

/// Standard confirmation modal dialog with optional warning icon, description, custom content slot,
/// and Cancel / Confirm buttons.
#[allow(clippy::too_many_arguments)]
pub fn confirm_dialog(
    max_width: f32,
    danger_icon: bool,
    title: impl IntoReactive<SharedString>,
    description: impl IntoReactive<SharedString>,
    extra: Option<AnyElement>,
    cancel_label: &'static str,
    confirm_label: &'static str,
    confirm_kind: ButtonKind,
    on_cancel: impl Fn() + Clone + 'static,
    on_confirm: impl Fn() + 'static,
) -> impl IntoElement {
    let title_view = text(title)
        .text_base()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(move || theme().foreground);

    let header = if danger_icon {
        view()
            .flex_row()
            .items_center()
            .gap_2p5()
            .child(icons::svg_icon(icons::ALERT_TRIANGLE, 18.0, move || {
                theme().destructive
            }))
            .child(title_view)
            .into_element()
    } else {
        title_view.into_element()
    };

    let body = view()
        .gap_4()
        .child(header)
        .child(
            text(description)
                .text_xs()
                .text_color(move || theme().muted_foreground),
        );
    let body = if let Some(extra_el) = extra {
        body.child(extra_el)
    } else {
        body
    };

    modal_dialog(
        max_width,
        on_cancel.clone(),
        body.child(
            view()
                .flex_row()
                .justify_end()
                .gap_2()
                .child(action(ButtonKind::Secondary, cancel_label, on_cancel))
                .child(action(confirm_kind, confirm_label, on_confirm)),
        ),
    )
}
