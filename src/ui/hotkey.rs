//! Reusable global hotkey display badge and interactive shortcut editor controls.

use std::rc::Rc;

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::hotkey::{HotkeyBinding, HotkeyStatus};

use super::button::{action_busy, ButtonKind};
use super::input::{text_field, toggle_switch};
use super::nav::filter_chip;
use super::surface::status_badge;
use super::{mono_font, ValueTone};

/// Built-in global hotkey preset combinations offered by [`hotkey_editor`].
pub const DEFAULT_HOTKEY_PRESETS: &[&str] = &["alt+space", "ctrl+shift+space", "ctrl+alt+f"];

/// Compact monospace pill displaying a formatted shortcut (e.g. `"Alt+Space"`).
pub fn hotkey_badge(label: impl IntoReactive<SharedString>) -> impl IntoElement {
    view()
        .flex_shrink_0()
        .px_2()
        .h(px(20.0))
        .items_center()
        .justify_center()
        .rounded_full()
        .border_1()
        .border_color(move || theme().border)
        .bg(move || theme().muted)
        .child(
            text(label)
                .text_xs()
                .font_family(mono_font)
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || theme().muted_foreground),
        )
}

/// Reusable global hotkey configuration control with enable/disable switch, preset chips,
/// custom keystroke input + Apply button, and a live [`status_badge`].
pub fn hotkey_editor(
    binding: impl Fn() -> HotkeyBinding + Clone + 'static,
    status: impl Fn() -> HotkeyStatus + Clone + 'static,
    on_change: impl Fn(HotkeyBinding) + Clone + 'static,
) -> impl IntoElement {
    hotkey_editor_with_presets(binding, status, DEFAULT_HOTKEY_PRESETS, on_change)
}

/// Variant of [`hotkey_editor`] accepting a custom list of preset keystroke strings.
pub fn hotkey_editor_with_presets(
    binding: impl Fn() -> HotkeyBinding + Clone + 'static,
    status: impl Fn() -> HotkeyStatus + Clone + 'static,
    presets: &'static [&'static str],
    on_change: impl Fn(HotkeyBinding) + Clone + 'static,
) -> impl IntoElement {
    let binding_fn = Rc::new(binding);
    let status_fn = Rc::new(status);
    let on_change = Rc::new(on_change);

    let binding_memo = computed(move || binding_fn());
    let status_memo = computed(move || status_fn());

    let initial = binding_memo.get_untracked();
    let draft = signal(initial.keystroke.clone());
    let last_committed = stored(initial.keystroke);

    effect(move || {
        let next = binding_memo.get().keystroke;
        if last_committed.replace(next.clone()) != next {
            untracked(|| draft.set(next));
        }
    });

    let on_toggle = {
        let on_change = Rc::clone(&on_change);
        move |enabled: bool| {
            let current = binding_memo.get_untracked();
            on_change(HotkeyBinding::new(enabled, current.keystroke));
        }
    };

    let apply_custom = {
        let on_change = Rc::clone(&on_change);
        move || {
            let trimmed = draft.get_untracked().trim().to_owned();
            draft.set(trimmed.clone());
            on_change(HotkeyBinding::new(true, trimmed));
        }
    };
    let apply_submit = apply_custom.clone();

    let header_row = view()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .child(toggle_switch(
            move || binding_memo.get().enabled,
            "启用全局快捷键",
            Some("启用全局快捷键"),
            on_toggle,
        ))
        .child(status_badge(
            move || status_memo.get().label(),
            move || status_memo.get().tone(),
        ));

    let presets_row = (!presets.is_empty()).then(|| {
        view()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap_1p5()
            .child(
                text("预设")
                    .text_xs()
                    .text_color(move || theme().muted_foreground),
            )
            .children(presets.iter().copied().map(|preset| {
                let chip_label = HotkeyBinding::new(true, preset).display_label();
                let on_change = Rc::clone(&on_change);
                filter_chip(
                    move || {
                        binding_memo
                            .get()
                            .keystroke
                            .trim()
                            .eq_ignore_ascii_case(preset)
                    },
                    chip_label,
                    move || {
                        draft.set(preset.to_owned());
                        on_change(HotkeyBinding::new(true, preset));
                    },
                )
                .into_element()
            }))
    });

    let input_row = view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(
            text_field(
                move || draft.get(),
                "例如 alt+space 或 ctrl+shift+space",
                move |t| draft.set(t.to_owned()),
            )
            .flex_1()
            .min_w(px(0.0))
            .font_family(mono_font)
            .label("自定义快捷键")
            .on_submit(move |_: &str| apply_submit()),
        )
        .child(action_busy(
            ButtonKind::Secondary,
            "应用",
            move || matches!(status_memo.get(), HotkeyStatus::Registering),
            apply_custom,
        ));

    let feedback_row = view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(
            text(move || match status_memo.get() {
                HotkeyStatus::Invalid(err) => err,
                HotkeyStatus::Conflict(label) => {
                    format!("{label} 已被其他程序占用，请更换快捷键")
                }
                HotkeyStatus::Registered(_) => "当前生效：".to_owned(),
                HotkeyStatus::Registering => "正在向系统注册全局快捷键…".to_owned(),
                HotkeyStatus::Disabled => {
                    "支持 alt / ctrl / shift / mod / win + 按键（如 alt+space）".to_owned()
                }
            })
            .min_w(px(0.0))
            .flex_shrink(1.0)
            .text_xs()
            .text_color(move || match status_memo.get() {
                HotkeyStatus::Invalid(_) => ValueTone::Bad.color(),
                HotkeyStatus::Conflict(_) => ValueTone::Warn.color(),
                _ => theme().muted_foreground,
            }),
        )
        .child(when(
            move || matches!(status_memo.get(), HotkeyStatus::Registered(_)),
            move || {
                hotkey_badge(move || match status_memo.get() {
                    HotkeyStatus::Registered(label) => label,
                    _ => String::new(),
                })
            },
        ));

    let root = view().gap_2p5().child(header_row);
    let root = match presets_row {
        Some(row) => root.child(row),
        None => root,
    };
    root.child(input_row).child(feedback_row)
}
