use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bento::hotkey::{
    bind_global_hotkey, toggle_window_visibility, HotkeyBinding, HotkeyStatus,
};
use bento::shell::{
    autostart_enabled, autostart_tray_item, bind_close_handler, clear_window_progress,
    desktop_window, init_autostart_for_test, refresh_autostart, remember_window_placement_with_key,
    save_window_placement, set_autostart_enabled, set_window_progress, title_bar_autostart_toggle,
    title_bar_right_with_theme, title_bar_shell, CloseBehavior, NoticeCenter, NoticeTone,
    ThemeMode, WindowProgressState,
};
use bento::ui::{action_busy, hotkey_badge, hotkey_editor, icon_action_spinning_with_tip, icons, ButtonKind, IconKind};
use bento::DEFAULT_THEME_JSON;
use nsh_base::theme;
use nsh_core::desktop::{
    CloseReason, DesktopCommand, Shortcut, WindowEvent, WindowMode, WindowPlacement, WindowRequest,
    WindowState,
};
use nsh_core::element::desktop::current_window;
use nsh_core::element::{Environment, Root};
use nsh_core::platform::{Idiom, Os};
use nsh_core::prelude::*;
use nsh_core::storage::{StorageBackend, StorageChange};
use nsh_core::{
    ApproxTextEngine, Command, Core, EventKind, EventPayload, MountBatch, MountItem, NodeId,
    PayloadValue, PressEvent, ROOT_ID,
};

#[derive(Default)]
struct MemoryBackend {
    entries: Mutex<BTreeMap<String, String>>,
}

impl MemoryBackend {
    fn seeded(pairs: impl IntoIterator<Item = (&'static str, String)>) -> Arc<Self> {
        let map = pairs
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect();
        Arc::new(Self {
            entries: Mutex::new(map),
        })
    }

    fn get(&self, key: &str) -> Option<String> {
        self.entries.lock().unwrap().get(key).cloned()
    }
}

impl StorageBackend for MemoryBackend {
    fn load(&self) -> Result<BTreeMap<String, String>, String> {
        Ok(self.entries.lock().unwrap().clone())
    }

    fn write(&self, change: StorageChange) -> Result<(), String> {
        let mut entries = self.entries.lock().unwrap();
        match change {
            StorageChange::Set(k, v) => {
                entries.insert(k, v);
            }
            StorageChange::Remove(k) => {
                entries.remove(&k);
            }
            StorageChange::Clear => entries.clear(),
        }
        Ok(())
    }
}

fn make_placement(x: f32, y: f32, w: f32, h: f32) -> WindowPlacement {
    WindowPlacement::new(
        WindowMode::Normal,
        Bounds::new(point(px(x), px(y)), size(px(w), px(h))),
    )
}

fn make_window_state(placement: WindowPlacement, visible: bool, minimized: bool, focused: bool) -> WindowState {
    let mut st = WindowState::new(placement.bounds, placement);
    st.visible = visible;
    st.minimized = minimized;
    st.focused = focused;
    st
}

fn desktop_env(state: Option<WindowState>) -> Environment {
    let mut env = Environment::default();
    env.os = Os::Windows;
    env.idiom = Idiom::Desktop;
    env.window = size(px(960.0), px(640.0));
    env.window_state = state;
    env
}

fn setup_core(storage: Arc<MemoryBackend>, env: Environment) -> Core<ApproxTextEngine> {
    let mut core = Core::new(ApproxTextEngine, size(px(960.0), px(640.0)));
    core.set_components(nsh_base::registry());
    if let Some(tokens) = core.theme_tokens().of(DEFAULT_THEME_JSON) {
        core.set_tokens(tokens);
    }
    core.set_storage(Some(storage));
    core.set_environment(env);
    core
}

fn desktop_commands(batch: &MountBatch) -> Vec<&DesktopCommand> {
    batch
        .items
        .iter()
        .filter_map(|item| match item {
            MountItem::Command {
                id: ROOT_ID,
                command: Command::Desktop(cmd),
            } => Some(cmd.as_ref()),
            _ => None,
        })
        .collect()
}

fn open_window_id(batch: &MountBatch, window_id: &str) -> NodeId {
    batch
        .items
        .iter()
        .find_map(|item| match item {
            MountItem::Command {
                id,
                command: Command::Desktop(cmd),
            } => match cmd.as_ref() {
                DesktopCommand::OpenWindow(open) if open.options.id == window_id => Some(*id),
                _ => None,
            },
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing OpenWindow for `{window_id}` in mount batch"))
}

fn window_commands(batch: &MountBatch, win_id: NodeId) -> Vec<&DesktopCommand> {
    batch
        .items
        .iter()
        .filter_map(|item| match item {
            MountItem::Command {
                id,
                command: Command::Desktop(cmd),
            } if *id == win_id => Some(cmd.as_ref()),
            _ => None,
        })
        .collect()
}

fn send_window_state(core: &mut Core<ApproxTextEngine>, win_id: NodeId, state: WindowState) {
    let _ = core.component_event(
        win_id,
        EventKind::Window,
        &EventPayload::Window(Box::new(WindowEvent::State(Box::new(state)))),
    );
}

fn texts_of(batch: &MountBatch) -> Vec<String> {
    batch
        .items
        .iter()
        .filter_map(|item| match item {
            MountItem::Text { text, .. } => Some(text.text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn window_placement_restores_debounces_ignores_minimized_and_flushes_on_quit() {
    let saved = make_placement(120.0, 90.0, 1024.0, 700.0);
    let saved_json = serde_json::to_string(&saved).unwrap();
    let key = "test.window.placement.v1";
    let storage = MemoryBackend::seeded([(key, saved_json.clone())]);

    let mut core = setup_core(storage.clone(), desktop_env(None));

    #[component]
    fn PlacementApp() -> impl IntoElement {
        desktop_window("PlacementTest", 820.0, 540.0, 42.0, || {
            let win = current_window();
            remember_window_placement_with_key(win, "test.window.placement.v1", 820.0, 540.0);
            view().flex_1()
        })
    }

    core.mount_root(&Root::of::<PlacementApp>());
    let batch = core.commit().unwrap();
    let win_id = open_window_id(&batch, "main");
    assert!(
        window_commands(&batch, win_id).iter().any(|cmd| matches!(
            cmd,
            DesktopCommand::Request(WindowRequest::SetPlacement(p)) if *p == saved
        )),
        "expected SetPlacement on startup, got {:?}",
        window_commands(&batch, win_id)
    );

    // Seed initial window state matching `saved`: no redundant write is scheduled.
    send_window_state(&mut core, win_id, make_window_state(saved, true, false, true));
    let _ = core.commit();
    core.run_component_tasks();
    assert_eq!(core.next_task_timer(), None);

    // Move window to B: before 250ms debounce expires, storage still holds A.
    let moved_b = make_placement(200.0, 150.0, 1100.0, 720.0);
    send_window_state(
        &mut core,
        win_id,
        make_window_state(moved_b, true, false, true),
    );
    let _ = core.commit();
    core.run_component_tasks();
    let due_b = core
        .next_task_timer()
        .expect("expected 250ms debounce timer after move B");
    assert_eq!(storage.get(key).as_deref(), Some(saved_json.as_str()));

    // Wait briefly (< 250ms) then move window to C -> resets 250ms timer cleanly.
    std::thread::sleep(Duration::from_millis(80));
    let moved_c = make_placement(240.0, 180.0, 1180.0, 760.0);
    send_window_state(
        &mut core,
        win_id,
        make_window_state(moved_c, true, false, true),
    );
    let _ = core.commit();
    core.run_component_tasks();
    let due_c = core
        .next_task_timer()
        .expect("expected rescheduled 250ms debounce timer after move C");
    assert!(due_c > due_b, "rescheduled timer should move deadline forward");

    // Advance past B's original deadline while still before C's deadline -> B must have been cancelled.
    let after_b_before_c = due_b + Duration::from_millis(15);
    if after_b_before_c < due_c {
        std::thread::sleep(after_b_before_c.saturating_duration_since(Instant::now()));
        core.run_component_tasks();
        assert_eq!(storage.get(key).as_deref(), Some(saved_json.as_str()));
    }

    // Complete the 250ms quiet period after C -> C is persisted.
    std::thread::sleep(
        (due_c + Duration::from_millis(15)).saturating_duration_since(Instant::now()),
    );
    core.run_component_tasks();
    let expected_c = serde_json::to_string(&moved_c).unwrap();
    assert_eq!(storage.get(key).as_deref(), Some(expected_c.as_str()));
    assert_eq!(core.next_task_timer(), None);

    // Move to valid D, then minimize immediately -> minimized state must NOT overwrite
    // and cancels D's pending timer, while cached valid D flushes synchronously on quit.
    let moved_d = make_placement(300.0, 220.0, 1200.0, 800.0);
    send_window_state(
        &mut core,
        win_id,
        make_window_state(moved_d, true, false, true),
    );
    let _ = core.commit();
    core.run_component_tasks();
    assert!(core.next_task_timer().is_some());

    let minimized_bad = make_placement(-32000.0, -32000.0, 160.0, 28.0);
    send_window_state(
        &mut core,
        win_id,
        make_window_state(minimized_bad, false, true, false),
    );
    let _ = core.commit();
    core.run_component_tasks();
    assert_eq!(core.next_task_timer(), None);
    assert_eq!(storage.get(key).as_deref(), Some(expected_c.as_str()));

    // Quit synchronously flushes cached valid placement D.
    let _ = core.start_quit();
    let expected_d = serde_json::to_string(&moved_d).unwrap();
    assert_eq!(storage.get(key).as_deref(), Some(expected_d.as_str()));
}

#[test]
fn bind_close_handler_flushes_placement_before_tray_hide_reopens_and_filters_invalid() {
    let storage = Arc::new(MemoryBackend::default());
    let initial = make_placement(50.0, 60.0, 900.0, 600.0);
    let mut core = setup_core(storage.clone(), desktop_env(None));

    #[component]
    fn CloseApp() -> impl IntoElement {
        desktop_window("CloseTest", 820.0, 540.0, 42.0, || {
            let win = current_window();
            remember_window_placement_with_key(win.clone(), "close.key", 820.0, 540.0);
            bind_close_handler(win.clone(), || CloseBehavior::Tray, || {});
            view().flex_1().child(
                view().test_id("save-direct").on_press(move || {
                    // Direct save_window_placement with too-high min_width does not save.
                    save_window_placement(&win, "direct.too_small", 2000.0, 540.0);
                    save_window_placement(&win, "direct.ok", 820.0, 540.0);
                }),
            )
        })
    }

    core.mount_root(&Root::of::<CloseApp>());
    let batch = core.commit().unwrap();
    let win_id = open_window_id(&batch, "main");
    let save_btn = find_test_id(&batch, "save-direct");

    send_window_state(&mut core, win_id, make_window_state(initial, true, false, true));
    let _ = core.commit();
    let _ = press_node(&mut core, save_btn);

    assert!(storage.get("direct.too_small").is_none());
    assert_eq!(
        storage.get("direct.ok").as_deref(),
        Some(serde_json::to_string(&initial).unwrap().as_str())
    );

    // Move window and immediately press close BEFORE 250ms debounce expires.
    let updated = make_placement(310.0, 210.0, 1080.0, 680.0);
    send_window_state(&mut core, win_id, make_window_state(updated, true, false, true));
    let _ = core.commit();
    assert!(storage.get("close.key").is_none());

    let _ = core.component_event(
        win_id,
        EventKind::Window,
        &EventPayload::Window(Box::new(WindowEvent::ClosePress {
            reason: CloseReason::CloseButton,
        })),
    );
    let batch = core.commit().unwrap();
    assert!(window_commands(&batch, win_id)
        .iter()
        .any(|cmd| matches!(cmd, DesktopCommand::Request(WindowRequest::Hide))));
    assert_eq!(
        storage.get("close.key").as_deref(),
        Some(serde_json::to_string(&updated).unwrap().as_str())
    );

    // Single-instance reopen restores, shows, and focuses the window exactly once.
    core.reopened();
    let batch = core.commit().unwrap();
    let reqs: Vec<WindowRequest> = window_commands(&batch, win_id)
        .into_iter()
        .filter_map(|cmd| match cmd {
            DesktopCommand::Request(req) => Some(req.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        reqs,
        vec![
            WindowRequest::Restore,
            WindowRequest::Show,
            WindowRequest::Focus
        ]
    );
}

fn find_test_id(batch: &MountBatch, test_id: &str) -> NodeId {
    batch
        .items
        .iter()
        .find_map(|item| match item {
            MountItem::ViewProps { id, props } if props.test_id.as_deref() == Some(test_id) => {
                Some(*id)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing test_id `{test_id}` in mount batch"))
}

fn press_node(core: &mut Core<ApproxTextEngine>, id: NodeId) -> MountBatch {
    assert!(
        core.component_event(
            id,
            EventKind::Press,
            &EventPayload::Press(PressEvent::default()),
        ),
        "expected Press event on node {id:?} to be handled"
    );
    core.run_component_tasks();
    core.commit().unwrap_or_default()
}

fn register_commands(batch: &MountBatch) -> Vec<(u32, Shortcut)> {
    desktop_commands(batch)
        .into_iter()
        .filter_map(|cmd| match cmd {
            DesktopCommand::RegisterGlobalShortcut { id, shortcut } => Some((*id, *shortcut)),
            _ => None,
        })
        .collect()
}

fn unregister_commands(batch: &MountBatch) -> Vec<u32> {
    desktop_commands(batch)
        .into_iter()
        .filter_map(|cmd| match cmd {
            DesktopCommand::UnregisterGlobalShortcut(id) => Some(*id),
            _ => None,
        })
        .collect()
}

fn send_shortcut_answer(core: &mut Core<ApproxTextEngine>, id: u32, registered: bool) -> MountBatch {
    let _ = core.component_event(
        ROOT_ID,
        EventKind::Menu,
        &EventPayload::Map(vec![
            ("id", PayloadValue::Number(f64::from(id))),
            ("registered", PayloadValue::Bool(registered)),
        ]),
    );
    core.run_component_tasks();
    core.commit().unwrap_or_default()
}

fn send_shortcut_trigger(core: &mut Core<ApproxTextEngine>, id: u32) -> MountBatch {
    let _ = core.component_event(
        ROOT_ID,
        EventKind::Menu,
        &EventPayload::Map(vec![("id", PayloadValue::Number(f64::from(id)))]),
    );
    core.run_component_tasks();
    core.commit().unwrap_or_default()
}

#[test]
fn bind_global_hotkey_validates_registers_handles_stale_generations_and_triggers() {
    thread_local! {
        static QUEUE: RefCell<VecDeque<HotkeyBinding>> = const { RefCell::new(VecDeque::new()) };
        static OUTPUTS: RefCell<Option<(Rc<RefCell<Vec<HotkeyStatus>>>, Rc<RefCell<usize>>)>> = const { RefCell::new(None) };
    }

    QUEUE.with(|q| {
        q.borrow_mut().extend([
            HotkeyBinding::new(true, "not++valid"),
            HotkeyBinding::new(true, "alt+space"),
            HotkeyBinding::new(true, "ctrl+shift+space"),
            HotkeyBinding::new(true, "ctrl+alt+f"),
            HotkeyBinding::new(true, "alt+space"),
            HotkeyBinding::disabled(),
        ]);
    });

    let storage = Arc::new(MemoryBackend::default());
    let mut core = setup_core(storage, desktop_env(None));

    #[component]
    fn HotkeyApp() -> impl IntoElement {
        let binding = signal(HotkeyBinding::disabled());
        let statuses = Rc::new(RefCell::new(Vec::new()));
        let triggers = Rc::new(RefCell::new(0_usize));

        let statuses_cb = Rc::clone(&statuses);
        let triggers_cb = Rc::clone(&triggers);
        bind_global_hotkey(
            move || binding.get(),
            move |st| statuses_cb.borrow_mut().push(st),
            move || *triggers_cb.borrow_mut() += 1,
        );

        OUTPUTS.with(|slot| *slot.borrow_mut() = Some((statuses, triggers)));
        view().flex_1().child(
            view().test_id("hotkey-driver").on_press(move || {
                if let Some(next) = QUEUE.with(|q| q.borrow_mut().pop_front()) {
                    binding.set(next);
                }
            }),
        )
    }

    core.mount_root(&Root::of::<HotkeyApp>());
    let batch = core.commit().unwrap();
    assert!(
        register_commands(&batch).is_empty() && unregister_commands(&batch).is_empty(),
        "expected no global shortcut commands on initial disabled mount, got {:?}",
        desktop_commands(&batch)
    );
    let driver_id = find_test_id(&batch, "hotkey-driver");
    let (statuses, triggers) = OUTPUTS.with(|slot| slot.borrow_mut().take().unwrap());
    assert_eq!(statuses.borrow().as_slice(), &[HotkeyStatus::Disabled]);

    // 1. Invalid syntax emits HotkeyStatus::Invalid without panicking or issuing desktop commands.
    statuses.borrow_mut().clear();
    let batch = press_node(&mut core, driver_id);
    assert!(register_commands(&batch).is_empty());
    assert!(unregister_commands(&batch).is_empty());
    assert!(
        matches!(statuses.borrow().as_slice(), [HotkeyStatus::Invalid(msg)] if !msg.is_empty()),
        "expected Invalid status, got {:?}",
        statuses.borrow()
    );

    // 2. Valid shortcut registers and enters Registering state before answer.
    statuses.borrow_mut().clear();
    let batch = press_node(&mut core, driver_id);
    let regs = register_commands(&batch);
    assert_eq!(regs.len(), 1);
    let (id1, sc1) = regs[0];
    assert_eq!(sc1, Shortcut::parse("alt+space", false).unwrap());
    assert_eq!(statuses.borrow().as_slice(), &[HotkeyStatus::Registering]);

    // 3. Change shortcut BEFORE id1's answer arrives: unregisters id1 and requests id2.
    statuses.borrow_mut().clear();
    let batch = press_node(&mut core, driver_id);
    assert_eq!(unregister_commands(&batch), vec![id1]);
    let regs = register_commands(&batch);
    assert_eq!(regs.len(), 1);
    let (id2, sc2) = regs[0];
    assert_ne!(id1, id2);
    assert_eq!(sc2, Shortcut::parse("ctrl+shift+space", false).unwrap());
    assert_eq!(statuses.borrow().as_slice(), &[HotkeyStatus::Registering]);

    // Stale answer for id1 must be ignored because generation moved to id2.
    statuses.borrow_mut().clear();
    let _ = send_shortcut_answer(&mut core, id1, true);
    assert!(
        statuses.borrow().is_empty(),
        "stale answer should be ignored, got {:?}",
        statuses.borrow()
    );

    // Answer for active id2 succeeds and activates the trigger callback.
    let _ = send_shortcut_answer(&mut core, id2, true);
    assert_eq!(
        statuses.borrow().as_slice(),
        &[HotkeyStatus::Registered("Ctrl+Shift+Space".to_owned())]
    );

    let _ = send_shortcut_trigger(&mut core, id2);
    assert_eq!(*triggers.borrow(), 1);

    // 4. Switch to ctrl+alt+f and simulate OS conflict (registered = false).
    statuses.borrow_mut().clear();
    let batch = press_node(&mut core, driver_id);
    assert_eq!(unregister_commands(&batch), vec![id2]);
    let regs = register_commands(&batch);
    assert_eq!(regs.len(), 1);
    let (id3, sc3) = regs[0];
    assert_eq!(sc3, Shortcut::parse("ctrl+alt+f", false).unwrap());

    let _ = send_shortcut_answer(&mut core, id3, false);
    assert_eq!(
        statuses.borrow().as_slice(),
        &[
            HotkeyStatus::Registering,
            HotkeyStatus::Conflict("Ctrl+Alt+F".to_owned()),
        ]
    );

    // 5. Re-register after conflict succeeds without spuriously unregistering rejected id3.
    statuses.borrow_mut().clear();
    let batch = press_node(&mut core, driver_id);
    assert!(unregister_commands(&batch).is_empty());
    let regs = register_commands(&batch);
    assert_eq!(regs.len(), 1);
    let (id4, _) = regs[0];
    let _ = send_shortcut_answer(&mut core, id4, true);
    assert_eq!(
        statuses.borrow().as_slice(),
        &[
            HotkeyStatus::Registering,
            HotkeyStatus::Registered("Alt+Space".to_owned()),
        ]
    );

    // 6. Disable binding: unregisters active id4 and reports Disabled.
    statuses.borrow_mut().clear();
    let batch = press_node(&mut core, driver_id);
    assert_eq!(unregister_commands(&batch), vec![id4]);
    assert!(register_commands(&batch).is_empty());
    assert_eq!(statuses.borrow().as_slice(), &[HotkeyStatus::Disabled]);
}

#[test]
fn toggle_window_visibility_respects_focus_visibility_and_minimized_state() {
    let storage = Arc::new(MemoryBackend::default());
    let mut core = setup_core(storage.clone(), desktop_env(None));

    #[component]
    fn ToggleApp() -> impl IntoElement {
        desktop_window("ToggleTest", 820.0, 540.0, 42.0, || {
            let win = current_window();
            remember_window_placement_with_key(win.clone(), "toggle.placement.v1", 820.0, 540.0);
            view().flex_1().child(
                view().test_id("toggle-btn").on_press(move || {
                    toggle_window_visibility(&win);
                }),
            )
        })
    }

    core.mount_root(&Root::of::<ToggleApp>());
    let batch = core.commit().unwrap();
    let win_id = open_window_id(&batch, "main");
    let btn_id = find_test_id(&batch, "toggle-btn");

    let window_requests = |batch: &MountBatch| -> Vec<WindowRequest> {
        window_commands(batch, win_id)
            .into_iter()
            .filter_map(|cmd| match cmd {
                DesktopCommand::Request(req) => Some(req.clone()),
                _ => None,
            })
            .collect()
    };

    // Case 1: visible && focused && !minimized -> flush placement + Hide.
    let updated = make_placement(220.0, 140.0, 1040.0, 680.0);
    send_window_state(&mut core, win_id, make_window_state(updated, true, false, true));
    let _ = core.commit();
    assert!(storage.get("toggle.placement.v1").is_none());

    let batch = press_node(&mut core, btn_id);
    assert_eq!(window_requests(&batch), vec![WindowRequest::Hide]);
    assert_eq!(
        storage.get("toggle.placement.v1").as_deref(),
        Some(serde_json::to_string(&updated).unwrap().as_str())
    );

    // Case 2: hidden -> Restore + Show + Focus.
    send_window_state(&mut core, win_id, make_window_state(updated, false, false, false));
    let _ = core.commit();
    let batch = press_node(&mut core, btn_id);
    assert_eq!(
        window_requests(&batch),
        vec![WindowRequest::Restore, WindowRequest::Show, WindowRequest::Focus]
    );

    // Case 3: visible but unfocused -> Restore + Show + Focus.
    send_window_state(&mut core, win_id, make_window_state(updated, true, false, false));
    let _ = core.commit();
    let batch = press_node(&mut core, btn_id);
    assert_eq!(
        window_requests(&batch),
        vec![WindowRequest::Restore, WindowRequest::Show, WindowRequest::Focus]
    );

    // Case 4: visible && focused but minimized -> Restore + Show + Focus.
    send_window_state(&mut core, win_id, make_window_state(updated, true, true, true));
    let _ = core.commit();
    let batch = press_node(&mut core, btn_id);
    assert_eq!(
        window_requests(&batch),
        vec![WindowRequest::Restore, WindowRequest::Show, WindowRequest::Focus]
    );
}

#[test]
fn window_progress_sets_clamps_and_clears_progress_attributes() {
    let storage = Arc::new(MemoryBackend::default());
    let mut core = setup_core(storage, desktop_env(None));

    #[component]
    fn ProgressApp() -> impl IntoElement {
        desktop_window("ProgressTest", 820.0, 540.0, 42.0, || {
            let win = current_window();
            let win_set = win.clone();
            let win_clear = win;
            view()
                .flex_1()
                .child(view().test_id("set-progress").on_press(move || {
                    set_window_progress(&win_set, WindowProgressState::Normal, Some(1.5));
                }))
                .child(view().test_id("clear-progress").on_press(move || {
                    clear_window_progress(&win_clear);
                }))
        })
    }

    core.mount_root(&Root::of::<ProgressApp>());
    let batch = core.commit().unwrap();
    let win_id = open_window_id(&batch, "main");
    let set_id = find_test_id(&batch, "set-progress");
    let clear_id = find_test_id(&batch, "clear-progress");

    let batch = press_node(&mut core, set_id);
    let attrs = window_commands(&batch, win_id)
        .into_iter()
        .find_map(|cmd| match cmd {
            DesktopCommand::SetAttributes(a) => Some(a),
            _ => None,
        })
        .expect("expected SetAttributes with clamped progress");
    let wp = attrs.progress.flatten().expect("expected Some(WindowProgress)");
    assert_eq!(wp.state, WindowProgressState::Normal);
    assert_eq!(wp.value, Some(Percentage(1.0)));

    let batch = press_node(&mut core, clear_id);
    let attrs = window_commands(&batch, win_id)
        .into_iter()
        .find_map(|cmd| match cmd {
            DesktopCommand::SetAttributes(a) => Some(a),
            _ => None,
        })
        .expect("expected SetAttributes clearing progress");
    assert_eq!(attrs.progress, Some(None));
}

#[test]
fn busy_actions_expose_accessibility_busy_and_disabled_state() {
    let storage = Arc::new(MemoryBackend::default());
    let mut core = setup_core(storage, desktop_env(None));

    #[component]
    fn BusyApp() -> impl IntoElement {
        let is_busy = signal(false);
        let clicks = signal(0_usize);
        view()
            .flex_1()
            .child(action_busy(
                ButtonKind::Primary,
                move || {
                    if is_busy.get() {
                        "同步中…"
                    } else {
                        "立即同步"
                    }
                },
                move || is_busy.get(),
                move || {
                    clicks.update(|n| *n += 1);
                    is_busy.set(true);
                },
            ))
            .child(icon_action_spinning_with_tip(
                IconKind::Secondary,
                icons::REFRESH,
                "刷新",
                "刷新列表",
                move || is_busy.get(),
                move || is_busy.get(),
                move || {},
            ))
            .child(text(move || format!("clicks:{}", clicks.get())))
    }

    core.mount_root(&Root::of::<BusyApp>());
    let batch = core.commit().unwrap();
    let texts = texts_of(&batch);
    assert!(texts.iter().any(|t| t == "立即同步"));
    assert!(texts.iter().any(|t| t == "clicks:0"));

    // Find the button node carrying label text "立即同步" and press it.
    let btn_id = batch
        .items
        .iter()
        .find_map(|item| match item {
            MountItem::ViewProps { id, props }
                if props.accessibility.role == AccessibilityRole::Button =>
            {
                Some(*id)
            }
            _ => None,
        })
        .expect("expected button node");
    let batch = press_node(&mut core, btn_id);
    let texts = texts_of(&batch);
    assert!(texts.iter().any(|t| t == "同步中…"));
    assert!(texts.iter().any(|t| t == "clicks:1"));
    assert!(batch.items.iter().any(|item| match item {
        MountItem::ViewProps { props, .. } => {
            props.accessibility.state.busy && props.accessibility.state.disabled
        }
        _ => false,
    }));
}

#[test]
fn ui_controls_render_title_bar_hotkey_badge_and_editor_across_statuses() {
    thread_local! {
        static UI_STEPS: RefCell<VecDeque<(ThemeMode, HotkeyBinding, HotkeyStatus)>> = const { RefCell::new(VecDeque::new()) };
    }

    UI_STEPS.with(|q| {
        q.borrow_mut().extend([
            (
                ThemeMode::Dark,
                HotkeyBinding::new(true, "ctrl+shift+space"),
                HotkeyStatus::Registering,
            ),
            (
                ThemeMode::Dark,
                HotkeyBinding::new(true, "ctrl+shift+space"),
                HotkeyStatus::Registered("Ctrl+Shift+Space".to_owned()),
            ),
            (
                ThemeMode::Light,
                HotkeyBinding::new(true, "ctrl+alt+f"),
                HotkeyStatus::Conflict("Ctrl+Alt+F".to_owned()),
            ),
            (
                ThemeMode::Light,
                HotkeyBinding::new(true, "bad++key"),
                HotkeyStatus::Invalid("invalid keystroke syntax".to_owned()),
            ),
        ]);
    });

    let storage = Arc::new(MemoryBackend::default());
    let mut core = setup_core(storage, desktop_env(None));

    #[component]
    fn ControlsApp() -> impl IntoElement {
        let mode = signal(ThemeMode::Light);
        let binding = signal(HotkeyBinding::disabled());
        let status = signal(HotkeyStatus::Disabled);

        let driver = view().test_id("ui-driver").on_press(move || {
            if let Some((next_mode, next_binding, next_status)) =
                UI_STEPS.with(|q| q.borrow_mut().pop_front())
            {
                mode.set(next_mode);
                binding.set(next_binding);
                status.set(next_status);
            }
        });

        view()
            .flex_1()
            .flex_col()
            .gap_3()
            .bg(move || theme().background)
            .child(title_bar_shell(
                42.0,
                text("TestApp"),
                view(),
                title_bar_right_with_theme(
                    text("Actions"),
                    move || mode.get(),
                    move |next| mode.set(next),
                ),
            ))
            .child(hotkey_badge("Alt+Space"))
            .child(hotkey_editor(
                move || binding.get(),
                move || status.get(),
                move |next| binding.set(next),
            ))
            .child(driver)
    }

    core.mount_root(&Root::of::<ControlsApp>());
    let batch = core.commit().unwrap();
    let driver_id = find_test_id(&batch, "ui-driver");
    let initial_texts = texts_of(&batch);
    for expected in [
        "TestApp",
        "Actions",
        "Alt+Space",
        "Ctrl+Shift+Space",
        "Ctrl+Alt+F",
        "未启用",
        "应用",
    ] {
        assert!(
            initial_texts.iter().any(|t| t == expected),
            "expected `{expected}` in initial texts: {initial_texts:?}"
        );
    }

    let batch = press_node(&mut core, driver_id);
    let registering_texts = texts_of(&batch);
    assert!(registering_texts.iter().any(|t| t == "正在注册…"));
    assert!(registering_texts
        .iter()
        .any(|t| t == "正在向系统注册全局快捷键…"));

    let batch = press_node(&mut core, driver_id);
    let registered_texts = texts_of(&batch);
    assert!(registered_texts
        .iter()
        .any(|t| t == "已启用 (Ctrl+Shift+Space)"));
    assert!(registered_texts.iter().any(|t| t == "当前生效："));

    let batch = press_node(&mut core, driver_id);
    let conflict_texts = texts_of(&batch);
    assert!(conflict_texts.iter().any(|t| t == "热键冲突 (Ctrl+Alt+F)"));
    assert!(conflict_texts
        .iter()
        .any(|t| t.contains("已被其他程序占用")));

    let batch = press_node(&mut core, driver_id);
    let invalid_texts = texts_of(&batch);
    assert!(invalid_texts.iter().any(|t| t == "快捷键无效"));
    assert!(invalid_texts
        .iter()
        .any(|t| t == "invalid keystroke syntax"));
}

#[test]
fn autostart_controller_test_mode_toggles_state_and_emits_notices_without_os_mutation() {
    thread_local! {
        static STATE_LOG: RefCell<Vec<(bool, Option<(String, NoticeTone)>)>> = const { RefCell::new(Vec::new()) };
    }
    STATE_LOG.with(|log| log.borrow_mut().clear());

    let storage = Arc::new(MemoryBackend::default());
    let mut core = setup_core(storage, desktop_env(None));

    #[component]
    fn AutoStartApp() -> impl IntoElement {
        init_autostart_for_test(false);
        // `refresh_autostart` must be a no-op in test mode.
        refresh_autostart();
        let _tray_item = autostart_tray_item();

        let toggle_driver = view().test_id("autostart-driver").on_press(move || {
            let next = !autostart_enabled();
            set_autostart_enabled(next);
            refresh_autostart();
            let notice = NoticeCenter::global()
                .current()
                .map(|n| (n.text, n.tone));
            STATE_LOG.with(|log| log.borrow_mut().push((autostart_enabled(), notice)));
        });

        view()
            .flex_1()
            .flex_col()
            .child(title_bar_shell(
                42.0,
                text("AutoStartApp"),
                view(),
                title_bar_autostart_toggle(),
            ))
            .child(toggle_driver)
    }

    core.mount_root(&Root::of::<AutoStartApp>());
    let batch = core.commit().unwrap();
    let texts = texts_of(&batch);
    assert!(
        texts.iter().any(|t| t == "开机自启"),
        "expected title_bar_autostart_toggle to render `开机自启`, got {texts:?}"
    );
    let driver_id = find_test_id(&batch, "autostart-driver");

    // Toggle false -> true
    let _ = press_node(&mut core, driver_id);
    // Toggle true -> false
    let _ = press_node(&mut core, driver_id);

    STATE_LOG.with(|log| {
        let entries = log.borrow();
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0],
            (true, Some(("已开启开机自启".to_owned(), NoticeTone::Ok)))
        );
        assert_eq!(
            entries[1],
            (false, Some(("已关闭开机自启".to_owned(), NoticeTone::Ok)))
        );
    });
}
