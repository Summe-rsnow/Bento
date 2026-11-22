//! Desktop shell infrastructure: hidden title bar window setup, appearance
//! switching, close-to-tray handling, system tray helpers, and global notice toasts.

use std::time::Duration;

use nsh_base::theme;
use nsh_core::desktop::{TitleBarOptions, TitleBarStyle, WindowAttributes};
use nsh_core::element::desktop::{current_window, quit, title_bar, MenuItem, WindowRef};
use nsh_core::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ui::{
    action, checkbox, icons, modal_dialog, notice_toast, radio_option, radius, segmented_icon,
    ButtonKind, ValueTone,
};

/// Configures the current desktop window with a hidden native title bar,
/// minimum size bounds, and intercepted close button (`close_on_press = false`).
pub fn init_desktop_window(min_width: f32, min_height: f32, title_bar_height: f32) {
    let mut tb = TitleBarOptions::default();
    tb.style = TitleBarStyle::Hidden;
    tb.height = Some(px(title_bar_height));
    let mut attrs = WindowAttributes::default();
    attrs.title_bar = Some(tb);
    attrs.min_size = Some(Some(size(px(min_width), px(min_height))));
    attrs.close_on_press = Some(false);
    current_window().set(attrs);
}

/// Builds a draggable hidden-title-bar frame reserving native window-button insets.
pub fn title_bar_shell(
    height: f32,
    left: impl IntoElement,
    center: impl IntoElement,
    right: impl IntoElement,
) -> View {
    view()
        .window_drag(true)
        .flex_row()
        .items_center()
        .gap_3()
        .h(move || px(title_bar().top.0.max(height)))
        .pl(move || px(title_bar().left.0 + 16.0))
        .pr(move || px(title_bar().right.0 + 16.0))
        .border_b_1()
        .border_color(move || theme().border)
        .child(view().flex_shrink_0().flex_row().items_center().gap_2().child(left))
        .child(view().flex_1().min_w(px(0.0)).flex_row().items_center().child(center))
        .child(
            view()
                .window_drag(false)
                .flex_shrink_0()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(right),
        )
}

/// User-selected color scheme (`system`, `light`, or `dark`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    /// Converts this preference to NSH's [`Appearance`].
    pub fn to_appearance(self) -> Appearance {
        match self {
            Self::System => Appearance::System,
            Self::Light => Appearance::Light,
            Self::Dark => Appearance::Dark,
        }
    }

    /// Applies this color scheme to the active NSH surface.
    pub fn apply(self) {
        set_color_scheme(self.to_appearance());
    }

    /// Human-readable Chinese label used for tooltips and accessibility.
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "跟随系统",
            Self::Light => "浅色模式",
            Self::Dark => "深色模式",
        }
    }
}

/// Standard 3-segment appearance switcher (`System` / `Light` / `Dark`).
pub fn theme_mode_segmented(
    current: impl Fn() -> ThemeMode + Copy + 'static,
    on_change: impl Fn(ThemeMode) + Copy + 'static,
) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(2.0))
        .p(px(2.0))
        .rounded(radius)
        .border_1()
        .border_color(move || theme().border)
        .bg(move || theme().muted)
        .child(segmented_icon(
            move || current() == ThemeMode::System,
            icons::MONITOR,
            ThemeMode::System.label(),
            move || on_change(ThemeMode::System),
        ))
        .child(segmented_icon(
            move || current() == ThemeMode::Light,
            icons::SUN,
            ThemeMode::Light.label(),
            move || on_change(ThemeMode::Light),
        ))
        .child(segmented_icon(
            move || current() == ThemeMode::Dark,
            icons::MOON,
            ThemeMode::Dark.label(),
            move || on_change(ThemeMode::Dark),
        ))
}

/// Action taken when the user clicks the main window's close button.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseBehavior {
    /// Prompt the user each time (default).
    #[default]
    Ask,
    /// Hide the main window to the system tray.
    Tray,
    /// Quit the application cleanly.
    Exit,
}

impl CloseBehavior {
    /// Human-readable label for tray menus and settings.
    pub fn label(self) -> &'static str {
        match self {
            Self::Ask => "每次询问",
            Self::Tray => "最小化到托盘",
            Self::Exit => "直接退出应用",
        }
    }
}

/// Installs the standard close-button handler on `window`.
pub fn bind_close_handler(
    window: WindowRef,
    get_behavior: impl Fn() -> CloseBehavior + 'static,
    on_ask: impl Fn() + 'static,
) {
    let win = window.clone();
    window.on_close_press(move |_reason| match get_behavior() {
        CloseBehavior::Ask => {
            win.focus();
            on_ask();
        }
        CloseBehavior::Tray => win.hide(),
        CloseBehavior::Exit => quit(),
    });
}

/// Builds the standard system tray submenu for switching [`CloseBehavior`].
pub fn close_behavior_submenu(
    current: CloseBehavior,
    on_change: impl Fn(CloseBehavior) + Copy + 'static,
) -> MenuItem {
    MenuItem::submenu(
        "点击关闭按钮时",
        [
            MenuItem::new(CloseBehavior::Ask.label())
                .checked(current == CloseBehavior::Ask)
                .on_select(move || on_change(CloseBehavior::Ask)),
            MenuItem::new(CloseBehavior::Tray.label())
                .checked(current == CloseBehavior::Tray)
                .on_select(move || on_change(CloseBehavior::Tray)),
            MenuItem::new(CloseBehavior::Exit.label())
                .checked(current == CloseBehavior::Exit)
                .on_select(move || on_change(CloseBehavior::Exit)),
        ],
    )
}

/// Modal dialog asking whether clicking close should minimize to tray or quit,
/// with an optional "不再提示" checkbox that persists the chosen [`CloseBehavior`].
pub fn confirm_close_dialog(
    open_sig: Signal<bool>,
    window: WindowRef,
    app_name: &'static str,
    on_choose_behavior: impl Fn(CloseBehavior) + 'static,
) -> impl IntoElement {
    let chosen_tray = signal(true);
    let remember = signal(false);
    let cancel = move || open_sig.set(false);
    let confirm = move || {
        let to_tray = chosen_tray.get_untracked();
        let save_choice = remember.get_untracked();
        open_sig.set(false);
        if save_choice {
            on_choose_behavior(if to_tray {
                CloseBehavior::Tray
            } else {
                CloseBehavior::Exit
            });
        }
        if to_tray {
            window.hide();
        } else {
            quit();
        }
    };

    modal_dialog(
        400.0,
        cancel,
        view()
            .gap_4()
            .child(
                view()
                    .gap_1()
                    .child(
                        text(format!("关闭 {app_name}"))
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(move || theme().foreground),
                    )
                    .child(
                        text("点击关闭按钮时希望如何处理？")
                            .text_xs()
                            .text_color(move || theme().muted_foreground),
                    ),
            )
            .child(
                view()
                    .gap_2()
                    .child(radio_option(
                        move || chosen_tray.get(),
                        "最小化到系统托盘",
                        "隐藏主窗口，可从右下角托盘图标随时恢复",
                        "最小化到系统托盘",
                        move || chosen_tray.set(true),
                    ))
                    .child(radio_option(
                        move || !chosen_tray.get(),
                        "退出应用",
                        format!("完全关闭 {app_name} 主窗口及系统托盘"),
                        "退出应用",
                        move || chosen_tray.set(false),
                    )),
            )
            .child(checkbox(
                move || remember.get(),
                || false,
                "不再提示",
                Some("不再提示（可在托盘右键菜单随时修改）"),
                move |next| remember.set(next),
            ))
            .child(
                view()
                    .flex_row()
                    .justify_end()
                    .gap_2()
                    .child(action(ButtonKind::Secondary, "取消", cancel))
                    .child(action(ButtonKind::Primary, "确定", confirm)),
            ),
    )
}

/// Severity tone of a transient toast notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeTone {
    Info,
    Ok,
    Warn,
    Bad,
}

impl NoticeTone {
    /// Maps this tone to its semantic indicator color.
    pub fn color(self) -> AdaptiveColor {
        match self {
            Self::Info => theme().muted_foreground,
            Self::Ok => ValueTone::Ok.color(),
            Self::Warn => ValueTone::Warn.color(),
            Self::Bad => ValueTone::Bad.color(),
        }
    }

    /// Default auto-dismiss lifetime (`6s` for errors, `3s` otherwise).
    pub fn ttl(self) -> Duration {
        match self {
            Self::Bad => Duration::from_secs(6),
            _ => Duration::from_secs(3),
        }
    }
}

/// A single transient feedback notice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub text: String,
    pub tone: NoticeTone,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct NoticeState {
    notice: Option<Notice>,
    seq: u64,
}

/// Reactive controller for global or component-scoped floating notice toasts.
#[derive(Clone, Copy)]
pub struct NoticeCenter {
    state: Signal<NoticeState>,
}

impl NoticeCenter {
    /// Creates a new locally owned notice center for the current component scope.
    pub fn new() -> Self {
        Self {
            state: signal(NoticeState::default()),
        }
    }

    /// Returns the surface-global notice center (safe to call from stores, listeners, and tasks).
    pub fn global() -> Self {
        Self {
            state: global::<NoticeState>(),
        }
    }

    /// Shows a notice with the given tone, incrementing the sequence counter.
    pub fn notify(&self, text: impl Into<String>, tone: NoticeTone) {
        let notice = Notice {
            text: text.into(),
            tone,
        };
        self.state.update(|s| {
            s.notice = Some(notice);
            s.seq = s.seq.wrapping_add(1);
        });
    }

    /// Shows an informational notice.
    pub fn notify_info(&self, text: impl Into<String>) {
        self.notify(text, NoticeTone::Info);
    }

    /// Shows a success notice.
    pub fn notify_ok(&self, text: impl Into<String>) {
        self.notify(text, NoticeTone::Ok);
    }

    /// Shows a warning notice.
    pub fn notify_warn(&self, text: impl Into<String>) {
        self.notify(text, NoticeTone::Warn);
    }

    /// Shows an error notice (auto-dismisses after 6s).
    pub fn notify_bad(&self, text: impl Into<String>) {
        self.notify(text, NoticeTone::Bad);
    }

    /// Returns the currently active notice (tracked where read).
    pub fn current(&self) -> Option<Notice> {
        self.state.with(|s| s.notice.clone())
    }

    /// Returns the monotonic sequence number of the latest notice.
    pub fn seq(&self) -> u64 {
        self.state.with(|s| s.seq)
    }

    /// Clears the active notice immediately.
    pub fn clear(&self) {
        self.state.update(|s| s.notice = None);
    }

    /// Clears the active notice only if `seq` still matches the latest notice.
    pub fn clear_if_seq(&self, seq: u64) {
        self.state.update(|s| {
            if s.seq == seq {
                s.notice = None;
            }
        });
    }

    /// Installs the reactive auto-dismiss effect (`3s` for ok/info/warn, `6s` for bad).
    pub fn install_auto_dismiss(&self) {
        let center = *self;
        effect(move || {
            if let Some(notice) = center.current() {
                let seq = center.seq();
                let task = set_timeout(notice.tone.ttl(), move || center.clear_if_seq(seq));
                cleanup(move || task.cancel());
            }
        });
    }

    /// Renders the bottom-centered floating [`notice_toast`] overlay when `visible()` is true
    /// and a notice is active.
    pub fn overlay(&self, visible: impl Fn() -> bool + Copy + 'static) -> impl IntoElement {
        let center = *self;
        when(
            move || visible() && center.current().is_some(),
            move || {
                view()
                    .absolute()
                    .bottom(px(16.0))
                    .left(px(16.0))
                    .right(px(16.0))
                    .items_center()
                    .z_index(10)
                    .pointer_events(PointerEvents::BoxNone)
                    .child(notice_toast(
                        move || center.current().map(|n| n.text).unwrap_or_default(),
                        move || {
                            center
                                .current()
                                .map(|n| n.tone.color())
                                .unwrap_or_else(|| theme().muted_foreground)
                        },
                        move || center.clear(),
                    ))
            },
        )
    }
}

impl Default for NoticeCenter {
    fn default() -> Self {
        Self::new()
    }
}
