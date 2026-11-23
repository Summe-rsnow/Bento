//! System-wide global hotkey binding, validation, status reporting, and window toggling.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use nsh_core::desktop::Shortcut;
use nsh_core::element::desktop::{
    register_global_shortcut, unregister_global_shortcut, WindowRef,
};
use nsh_core::platform::Os;
use nsh_core::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ui::ValueTone;

/// Persistent configuration for an application-wide global hotkey.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotkeyBinding {
    pub enabled: bool,
    pub keystroke: String,
}

impl Default for HotkeyBinding {
    fn default() -> Self {
        Self {
            enabled: false,
            keystroke: "alt+space".to_owned(),
        }
    }
}

impl HotkeyBinding {
    /// Creates a new hotkey binding with the given enable flag and keystroke string.
    pub fn new(enabled: bool, keystroke: impl Into<String>) -> Self {
        Self {
            enabled,
            keystroke: keystroke.into(),
        }
    }

    /// Returns the default disabled hotkey binding (`enabled = false`, `keystroke = "alt+space"`).
    pub fn disabled() -> Self {
        Self::default()
    }

    /// Validates `keystroke` using NSH's [`Shortcut::parse`] without panicking.
    pub fn validate(&self) -> Result<Shortcut, String> {
        Shortcut::parse(self.keystroke.trim(), current_os() == Os::MacOs).map_err(|e| e.to_string())
    }

    /// Formats `keystroke` for the active OS (e.g. `"Alt+Space"` on Windows/Linux, `"⌥Space"` on macOS),
    /// falling back to the raw trimmed string if invalid.
    pub fn display_label(&self) -> String {
        match self.validate() {
            Ok(shortcut) => shortcut.label_for(current_os()),
            Err(_) => self.keystroke.trim().to_owned(),
        }
    }
}

/// Live registration state of a global hotkey binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyStatus {
    Disabled,
    Registering,
    Registered(String),
    Conflict(String),
    Invalid(String),
}

impl HotkeyStatus {
    /// Concise human-readable status label suitable for badges.
    pub fn label(&self) -> String {
        match self {
            Self::Disabled => "未启用".to_owned(),
            Self::Registering => "正在注册…".to_owned(),
            Self::Registered(label) => format!("已启用 ({label})"),
            Self::Conflict(label) => format!("热键冲突 ({label})"),
            Self::Invalid(_) => "快捷键无效".to_owned(),
        }
    }

    /// Semantic badge tone corresponding to this status.
    pub fn tone(&self) -> ValueTone {
        match self {
            Self::Disabled => ValueTone::Neutral,
            Self::Registering => ValueTone::Info,
            Self::Registered(_) => ValueTone::Ok,
            Self::Conflict(_) => ValueTone::Warn,
            Self::Invalid(_) => ValueTone::Bad,
        }
    }
}

/// Reactively binds `binding()` as a system-wide global shortcut under the current NSH owner.
///
/// Validates syntax with [`Shortcut::parse`] before calling `register_global_shortcut` (which
/// panics on invalid input), unregisters previous keystrokes on change/disable/cleanup, and
/// discards stale async registration results using a monotonic generation counter.
pub fn bind_global_hotkey(
    binding: impl Fn() -> HotkeyBinding + 'static,
    on_status: impl Fn(HotkeyStatus) + Clone + 'static,
    on_trigger: impl Fn() + Clone + 'static,
) {
    let binding_memo = computed(binding);
    let generation = Rc::new(Cell::new(0_u64));
    let active_key: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    {
        let active_key = Rc::clone(&active_key);
        cleanup(move || {
            if let Some(prev) = active_key.borrow_mut().take() {
                unregister_global_shortcut(&prev);
            }
        });
    }

    effect(move || {
        let current_gen = generation.get().wrapping_add(1);
        generation.set(current_gen);

        if let Some(prev) = active_key.borrow_mut().take() {
            unregister_global_shortcut(&prev);
        }

        let current = binding_memo.get();
        if !current.enabled {
            on_status(HotkeyStatus::Disabled);
            return;
        }

        let keystroke = current.keystroke.trim().to_owned();
        let shortcut = match Shortcut::parse(&keystroke, current_os() == Os::MacOs) {
            Ok(shortcut) => shortcut,
            Err(err) => {
                on_status(HotkeyStatus::Invalid(err.to_string()));
                return;
            }
        };

        let label = shortcut.label_for(current_os());
        on_status(HotkeyStatus::Registering);
        *active_key.borrow_mut() = Some(keystroke.clone());

        {
            let active_key_cleanup = Rc::clone(&active_key);
            let key_cleanup = keystroke.clone();
            cleanup(move || {
                unregister_global_shortcut(&key_cleanup);
                let mut slot = active_key_cleanup.borrow_mut();
                if slot.as_deref() == Some(&key_cleanup) {
                    *slot = None;
                }
            });
        }

        let trigger = on_trigger.clone();
        let answer = register_global_shortcut(&keystroke, move || trigger());

        let generation_task = Rc::clone(&generation);
        let on_status_task = on_status.clone();
        let active_key_task = Rc::clone(&active_key);
        let key_task = keystroke;
        let task = spawn(async move {
            let ok = answer.await;
            if generation_task.get() != current_gen {
                return;
            }
            if ok {
                on_status_task(HotkeyStatus::Registered(label));
            } else {
                let mut slot = active_key_task.borrow_mut();
                if slot.as_deref() == Some(&key_task) {
                    *slot = None;
                }
                on_status_task(HotkeyStatus::Conflict(label));
            }
        });
        cleanup(move || task.cancel());
    });
}

/// Shows, restores, and focuses `window`, or hides it when it is already visible, focused,
/// and not minimized.
pub fn toggle_window_visibility(window: &WindowRef) {
    let state = untracked(|| window.state());
    if let Some(st) = state
        && st.visible
        && st.focused
        && !st.minimized
    {
        crate::shell::flush_registered_window_placement(window);
        window.hide();
    } else {
        window.restore();
        window.show();
        window.focus();
    }
}

fn current_os() -> Os {
    try_os().unwrap_or_else(|| {
        if cfg!(target_os = "macos") {
            Os::MacOs
        } else if cfg!(target_os = "windows") {
            Os::Windows
        } else if cfg!(target_os = "linux") {
            Os::Linux
        } else {
            Os::default()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkey_binding_defaults_and_constructors() {
        let def = HotkeyBinding::default();
        assert!(!def.enabled);
        assert_eq!(def.keystroke, "alt+space");
        assert_eq!(HotkeyBinding::disabled(), def);

        let custom = HotkeyBinding::new(true, "ctrl+shift+space");
        assert!(custom.enabled);
        assert_eq!(custom.keystroke, "ctrl+shift+space");
    }

    #[test]
    fn hotkey_validation_accepts_valid_and_rejects_invalid() {
        assert!(HotkeyBinding::new(true, "alt+space").validate().is_ok());
        assert!(HotkeyBinding::new(true, " ctrl+shift+space ").validate().is_ok());
        assert!(HotkeyBinding::new(true, "ctrl+alt+f").validate().is_ok());
        assert!(HotkeyBinding::new(true, "mod+k").validate().is_ok());

        assert!(HotkeyBinding::new(true, "").validate().is_err());
        assert!(HotkeyBinding::new(true, "   ").validate().is_err());
        assert!(HotkeyBinding::new(true, "alt+").validate().is_err());
        assert!(HotkeyBinding::new(true, "bogus+space").validate().is_err());
        assert!(HotkeyBinding::new(true, "mod+k mod+s").validate().is_err());
        assert!(HotkeyBinding::new(true, "ctrl+ctrl+a").validate().is_err());
    }

    #[test]
    fn hotkey_display_label_formats_cleanly_and_falls_back_on_invalid() {
        let label = HotkeyBinding::new(true, "alt+space").display_label();
        if cfg!(target_os = "macos") {
            assert_eq!(label, "⌥Space");
        } else {
            assert_eq!(label, "Alt+Space");
        }

        let combo = HotkeyBinding::new(true, "ctrl+shift+space").display_label();
        if cfg!(target_os = "macos") {
            assert_eq!(combo, "⌃⇧Space");
        } else {
            assert_eq!(combo, "Ctrl+Shift+Space");
        }

        assert_eq!(
            HotkeyBinding::new(true, " not+valid ").display_label(),
            "not+valid"
        );
    }

    #[test]
    fn hotkey_status_labels_and_tones_match_semantics() {
        assert_eq!(HotkeyStatus::Disabled.tone(), ValueTone::Neutral);
        assert_eq!(HotkeyStatus::Registering.tone(), ValueTone::Info);
        assert_eq!(
            HotkeyStatus::Registered("Alt+Space".into()).tone(),
            ValueTone::Ok
        );
        assert_eq!(
            HotkeyStatus::Conflict("Alt+Space".into()).tone(),
            ValueTone::Warn
        );
        assert_eq!(
            HotkeyStatus::Invalid("bad".into()).tone(),
            ValueTone::Bad
        );

        assert!(!HotkeyStatus::Disabled.label().is_empty());
        assert!(!HotkeyStatus::Registering.label().is_empty());
        assert!(HotkeyStatus::Registered("Alt+Space".into())
            .label()
            .contains("Alt+Space"));
        assert!(HotkeyStatus::Conflict("Alt+Space".into())
            .label()
            .contains("Alt+Space"));
        assert!(!HotkeyStatus::Invalid("bad".into()).label().is_empty());
    }
}
