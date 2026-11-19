use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::session_cmds::IpcError;

/// The four shortcuts people can change (spec §5). Serialized camelCase so the
/// settings map keys and the IPC values read `drawKey`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "camelCase")]
pub enum ShortcutAction {
    Dictation,
    Session,
    Screenshot,
    DrawKey,
}

/// One row of Settings → Keyboard, in the stored spelling (`Option+Super+K`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutBinding {
    pub action: ShortcutAction,
    pub keys: String,
}

/// Settings → Keyboard refetches its rows on this.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutsChanged {}

impl ShortcutAction {
    pub(crate) const ALL: [ShortcutAction; 4] = [
        ShortcutAction::Dictation,
        ShortcutAction::Session,
        ShortcutAction::Screenshot,
        ShortcutAction::DrawKey,
    ];

    /// The name Settings → Keyboard shows, reused in the conflict copy.
    fn name(self) -> &'static str {
        match self {
            ShortcutAction::Dictation => "Start / stop dictation",
            ShortcutAction::Session => "Start / stop a session",
            ShortcutAction::Screenshot => "Take a screenshot",
            ShortcutAction::DrawKey => "Draw (hold)",
        }
    }

    pub(crate) fn default_keys(self) -> &'static str {
        match self {
            ShortcutAction::Dictation => "Super+Control",
            ShortcutAction::Session => "Option+Super+K",
            ShortcutAction::Screenshot => "Super+Option+S",
            ShortcutAction::DrawKey => "right_option",
        }
    }
}

/// macOS defaults the system takes, in the app's modifier order, with the
/// name the conflict copy shows (spec §5 conflict rules).
const SYSTEM_SHORTCUTS: [(&str, &str); 10] = [
    ("Super+Space", "Spotlight"),
    ("Super+Tab", "the app switcher"),
    ("Super+KeyQ", "Quit"),
    ("Super+KeyW", "Close Window"),
    ("Super+KeyH", "Hide"),
    ("Super+KeyM", "Minimize"),
    ("Control+Space", "Input Source"),
    ("Super+Shift+Digit3", "a screenshot"),
    ("Super+Shift+Digit4", "a screenshot"),
    ("Super+Shift+Digit5", "a screenshot"),
];

/// The spec's `drawKey` value for a UI name U5 could send.
fn canonical_draw_key(keys: &str) -> Option<&'static str> {
    match keys.trim().to_ascii_lowercase().as_str() {
        "right_option" | "rightoption" | "right option" | "right_alt" | "rightalt" => {
            Some("right_option")
        }
        "right_command" | "rightcommand" | "right command" | "right_super" | "rightsuper" => {
            Some("right_command")
        }
        "fn" | "globe" | "globe / fn" => Some("fn"),
        _ => None,
    }
}

fn modifier_order(modifier: &str) -> u8 {
    match modifier {
        "Control" => 0,
        "Option" | "Alt" => 1,
        "Shift" => 2,
        "Super" => 3,
        _ => 4,
    }
}

/// Split one shortcut into its modifiers and its single key. Both
/// `Super+Option+S` and the plugin's `Alt+Super+KeyS` end up `(mods, "S")`.
fn split_keys(keys: &str) -> Option<(Vec<&str>, &str)> {
    let mut parts: Vec<&str> = keys
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    let key = parts.pop()?;
    Some((parts, key))
}

fn canonical_modifier(modifier: &str) -> Option<&'static str> {
    match modifier {
        "Control" | "Ctrl" | "ControlOrCommand" | "CmdOrCtrl" => Some("Control"),
        "Option" | "Alt" => Some("Option"),
        "Shift" => Some("Shift"),
        "Super" | "Command" | "CommandOrControl" | "Meta" => Some("Super"),
        _ => None,
    }
}

/// The stored spelling: modifiers deduplicated in Apple order, then the key.
fn canonical(keys: &str) -> Option<String> {
    let (modifiers, key) = split_keys(keys)?;
    let mut mods: Vec<&'static str> = modifiers
        .iter()
        .map(|m| canonical_modifier(m))
        .collect::<Option<_>>()?;
    mods.sort_by_key(|m| modifier_order(m));
    mods.dedup();
    let key = Some(key).filter(|k| !k.is_empty())?;
    if mods.is_empty() {
        return Some(key.to_string());
    }
    Some(format!("{}+{}", mods.join("+"), key))
}

/// The plugin's spelling of a stored value (`Option+Super+K` →
/// `Alt+Super+KeyK`), or `None` when the plugin cannot register it. The
/// dictation chord is not a plugin shortcut (the tap holds it) and passes as is.
pub(crate) fn spec_keys(keys: &str) -> Option<String> {
    let canonical = canonical(keys)?;
    if canonical == "Super+Control" {
        return Some(canonical);
    }
    let (modifiers, key) = split_keys(&canonical)?;
    if modifiers.is_empty() {
        return Some(key.to_string());
    }
    let plugin_mods: Vec<&str> = modifiers
        .iter()
        .map(|m| match *m {
            "Option" => "Alt",
            other => other,
        })
        .collect();
    let code = if key.len() == 1 && key.chars().all(|c| c.is_ascii_digit()) {
        format!("Digit{key}")
    } else if key.len() == 1 && key.chars().all(|c| c.is_ascii_alphabetic()) {
        format!("Key{}", key.to_ascii_uppercase())
    } else {
        key.to_string()
    };
    let spec = format!("{}+{}", plugin_mods.join("+"), code);
    spec.parse::<tauri_plugin_global_shortcut::Shortcut>()
        .ok()
        .map(|_| spec)
}

/// The value the map and the registry hold for a raw setting.
pub(crate) fn store(action: ShortcutAction, keys: &str) -> Option<String> {
    if action == ShortcutAction::DrawKey {
        return canonical_draw_key(keys).map(str::to_string);
    }
    spec_keys(keys).and_then(|_| canonical(keys))
}

fn invalid(action: ShortcutAction) -> IpcError {
    IpcError::new(
        "invalid",
        if action == ShortcutAction::DrawKey {
            "Choose Right Option, Right Command or Fn."
        } else {
            "That isn't a shortcut SpeakType can use."
        },
    )
}

/// The check before registering: a draw key must be one of the three, a global
/// shortcut must parse, must not be another action's, and — without `force` —
/// must not be a macOS default. Returns the stored form.
pub(crate) fn validate(
    action: ShortcutAction,
    keys: &str,
    force: bool,
    current: &BTreeMap<ShortcutAction, String>,
) -> Result<String, IpcError> {
    let stored = store(action, keys).ok_or_else(|| invalid(action))?;
    for other in ShortcutAction::ALL {
        if other != action && current.get(&other) == Some(&stored) {
            return Err(IpcError::new(
                "conflict",
                format!("Already used for {}", other.name()),
            ));
        }
    }
    let system_name = SYSTEM_SHORTCUTS
        .iter()
        .find(|(system_keys, _)| canonical(system_keys) == Some(stored.clone()))
        .map(|(_, name)| *name);
    if let (false, Some(name)) = (force, system_name) {
        return Err(IpcError::new(
            "systemShortcut",
            format!("macOS uses this for {name}"),
        ));
    }
    Ok(stored)
}

/// The dictation chord rides the CGEventTap; every other shortcut, the plugin.
pub(crate) fn uses_modifier_tap(action: ShortcutAction, keys: &str) -> bool {
    action == ShortcutAction::Dictation && canonical(keys).is_some_and(|k| k == "Super+Control")
}

/// Restore defaults, which the spec says must never be reset to a key another
/// action is holding: a clashing action keeps its current value.
pub(crate) fn default_bindings(
    current: &BTreeMap<ShortcutAction, String>,
) -> BTreeMap<ShortcutAction, String> {
    let mut restored = BTreeMap::new();
    for action in ShortcutAction::ALL {
        let default = default_for(action);
        let already_held = restored.values().any(|held| *held == default);
        let kept = current.get(&action).cloned().unwrap_or(default.clone());
        restored.insert(action, if already_held { kept } else { default });
    }
    restored
}

fn default_for(action: ShortcutAction) -> String {
    canonical(action.default_keys()).expect("defaults are valid shortcuts")
}

/// What Settings → Keyboard shows: every action in menu order.
pub(crate) fn bindings(current: &BTreeMap<ShortcutAction, String>) -> Vec<ShortcutBinding> {
    ShortcutAction::ALL
        .iter()
        .map(|action| ShortcutBinding {
            action: *action,
            keys: current
                .get(action)
                .cloned()
                .unwrap_or_else(|| default_for(*action)),
        })
        .collect()
}

/// The registry's live view: what each action is registered as right now. A
/// failed registration never reaches the map, and a tap action shows `None`,
/// so the next change still knows what to release.
#[derive(Default)]
pub(crate) struct LiveShortcuts(BTreeMap<ShortcutAction, Option<String>>);

impl LiveShortcuts {
    fn get(&self, action: ShortcutAction) -> Option<String> {
        self.0.get(&action).cloned().flatten()
    }

    /// Startup: record what the OS just accepted for an action.
    pub(crate) fn set_armed(&mut self, action: ShortcutAction, keys: &str) {
        self.0.insert(action, Some(keys.to_string()));
    }

    fn set(&mut self, action: ShortcutAction, keys: Option<String>) {
        self.0.insert(action, keys);
    }
}

/// The OS side of the registry, behind a trait so the re-registration rule is
/// testable: arming a shortcut returns whether the OS accepted it.
pub(crate) trait ShortcutsPlatform {
    fn arm(&self, action: ShortcutAction, keys: &str) -> bool;
    fn disarm(&self, keys: &str);
}

/// Move one action to `keys`, touching only its own registration. Validation
/// and conflicts come first, so a refused change neither saves nor registers;
/// a registration the OS rejects rolls back to what that action had.
pub(crate) fn apply(
    platform: &impl ShortcutsPlatform,
    live: &mut LiveShortcuts,
    current: &BTreeMap<ShortcutAction, String>,
    action: ShortcutAction,
    keys: &str,
    force: bool,
) -> Result<String, IpcError> {
    let stored = validate(action, keys, force, current)?;
    let previous = live.get(action);
    if previous.as_deref() == Some(stored.as_str()) {
        return Ok(stored);
    }
    if let Some(old) = &previous {
        platform.disarm(old);
    }
    if !platform.arm(action, &stored) {
        if let Some(old) = &previous {
            platform.arm(action, old);
        }
        return Err(register_failed());
    }
    live.set(
        action,
        (!uses_modifier_tap(action, &stored)).then_some(stored.clone()),
    );
    Ok(stored)
}

pub(crate) fn register_failed() -> IpcError {
    IpcError::new(
        "registerFailed",
        "Couldn't use this shortcut. Another app may own it.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::{HashMap, HashSet};

    /// Records every call; the real `apply` drives it, so the fake only has to
    /// answer what the OS would.
    #[derive(Default)]
    struct FakePlatform {
        pressed: RefCell<HashMap<String, ShortcutAction>>,
        registered: RefCell<HashSet<String>>,
        calls: RefCell<Vec<String>>,
        taps: RefCell<usize>,
        refuse: Vec<&'static str>,
    }

    impl FakePlatform {
        fn refusing(keys: &'static str) -> Self {
            FakePlatform {
                refuse: vec![keys],
                ..Default::default()
            }
        }

        fn press(&self, keys: &str) -> Option<ShortcutAction> {
            self.pressed.borrow().get(keys).copied()
        }

        fn registered(&self) -> Vec<String> {
            let mut keys: Vec<String> = self.registered.borrow().iter().cloned().collect();
            keys.sort();
            keys
        }

        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    impl ShortcutsPlatform for FakePlatform {
        fn arm(&self, action: ShortcutAction, keys: &str) -> bool {
            let slot = keys.to_string();
            self.calls.borrow_mut().push(format!("arm {}", slot));
            if action == ShortcutAction::DrawKey {
                return true;
            }
            if uses_modifier_tap(action, keys) {
                *self.taps.borrow_mut() += 1;
                return true;
            }
            let spec = spec_keys(keys).expect("a stored value is always registerable");
            if self
                .refuse
                .iter()
                .any(|refused| spec_keys(refused) == Some(spec.clone()))
            {
                return false;
            }
            self.pressed.borrow_mut().insert(spec.clone(), action);
            self.registered.borrow_mut().insert(spec);
            true
        }

        fn disarm(&self, keys: &str) {
            self.calls.borrow_mut().push(format!("disarm {}", keys));
            if let Some(spec) = spec_keys(keys) {
                self.pressed.borrow_mut().remove(&spec);
                self.registered.borrow_mut().remove(&spec);
            }
        }
    }

    fn map(entries: &[(ShortcutAction, &str)]) -> BTreeMap<ShortcutAction, String> {
        entries
            .iter()
            .map(|(action, keys)| (*action, keys.to_string()))
            .collect()
    }

    #[test]
    fn defaults_are_the_spec_table() {
        assert_eq!(
            default_bindings(&BTreeMap::new())
                .into_iter()
                .collect::<Vec<_>>(),
            vec![
                (ShortcutAction::Dictation, "Super+Control".to_string()),
                (ShortcutAction::Session, "Option+Super+K".to_string()),
                (ShortcutAction::Screenshot, "Option+Super+S".to_string()),
                (ShortcutAction::DrawKey, "right_option".to_string()),
            ]
        );
    }

    #[test]
    fn conflicting_shortcut_is_refused_with_the_ui_action_name() {
        let current = map(&[
            (ShortcutAction::Dictation, "F9"),
            (ShortcutAction::Session, "Option+Super+K"),
        ]);

        let e = validate(ShortcutAction::Dictation, "Option+Super+K", false, &current)
            .expect_err("taken");
        assert_eq!(e.kind, "conflict");
        assert_eq!(e.message, "Already used for Start / stop a session");

        validate(ShortcutAction::Dictation, "F9", false, &current).expect("its own value is kept");
    }

    #[test]
    fn system_shortcut_needs_force() {
        let current = default_bindings(&BTreeMap::new());

        let e =
            validate(ShortcutAction::Dictation, "Super+Space", false, &current).expect_err("macOS");
        assert_eq!(e.kind, "systemShortcut");
        assert_eq!(e.message, "macOS uses this for Spotlight");

        assert_eq!(
            validate(ShortcutAction::Dictation, "Super+KeyQ", false, &current)
                .expect_err("macOS")
                .message,
            "macOS uses this for Quit"
        );

        validate(ShortcutAction::Dictation, "Super+Space", true, &current)
            .expect("force resends past the warning");
    }

    #[test]
    fn invalid_shortcut_is_refused() {
        let current = default_bindings(&BTreeMap::new());
        assert_eq!(
            validate(ShortcutAction::Dictation, "Hyper+F8", false, &current)
                .expect_err("no Hyper modifier")
                .kind,
            "invalid"
        );
        assert_eq!(
            validate(ShortcutAction::DrawKey, "LeftOption", false, &current)
                .expect_err("not a draw key")
                .kind,
            "invalid"
        );
    }

    #[test]
    fn u5_name_and_stored_values_both_validate_to_the_stored_draw_key() {
        let current = default_bindings(&BTreeMap::new());
        for (sent, stored) in [
            ("RightOption", "right_option"),
            ("right_option", "right_option"),
            ("RightCommand", "right_command"),
            ("right_command", "right_command"),
            ("Fn", "fn"),
            ("fn", "fn"),
        ] {
            assert_eq!(
                validate(ShortcutAction::DrawKey, sent, false, &current).expect("draw key"),
                stored
            );
        }
    }

    #[test]
    fn settings_values_are_canonicalised_to_shortcut_syntax() {
        assert_eq!(
            spec_keys("Option+Super+K").as_deref(),
            Some("Alt+Super+KeyK")
        );
        assert_eq!(
            spec_keys("Super+Option+S").as_deref(),
            Some("Alt+Super+KeyS")
        );
        assert_eq!(spec_keys("Super+Control").as_deref(), Some("Super+Control"));
        assert_eq!(
            spec_keys("Shift+Super+Digit4").as_deref(),
            Some("Shift+Super+Digit4")
        );
        assert_eq!(spec_keys("F8").as_deref(), Some("F8"));
        assert_eq!(spec_keys("Super+Nope"), None);
        assert_eq!(
            spec_keys("Shift+Meta+KeyA").as_deref(),
            Some("Shift+Super+KeyA")
        );
        assert_eq!(
            canonical("Super+Option+S").as_deref(),
            Some("Option+Super+S")
        );
    }

    #[test]
    fn only_dictation_has_a_modifier_tap_and_register_failure_message_is_fixed() {
        assert!(uses_modifier_tap(
            ShortcutAction::Dictation,
            "Super+Control"
        ));
        assert!(uses_modifier_tap(
            ShortcutAction::Dictation,
            " Super + Control "
        ));
        assert!(!uses_modifier_tap(ShortcutAction::Dictation, "F8"));
        assert!(!uses_modifier_tap(ShortcutAction::Session, "Super+Control"));
        assert!(!uses_modifier_tap(ShortcutAction::DrawKey, "Super+Control"));
        let failed = register_failed();
        assert_eq!(failed.kind, "registerFailed");
        assert_eq!(
            failed.message,
            "Couldn't use this shortcut. Another app may own it."
        );
    }

    #[test]
    fn bindings_list_every_action_in_menu_order() {
        assert_eq!(
            bindings(&map(&[(ShortcutAction::Dictation, "F8")])),
            vec![
                ShortcutBinding {
                    action: ShortcutAction::Dictation,
                    keys: "F8".to_string(),
                },
                ShortcutBinding {
                    action: ShortcutAction::Session,
                    keys: "Option+Super+K".to_string(),
                },
                ShortcutBinding {
                    action: ShortcutAction::Screenshot,
                    keys: "Option+Super+S".to_string(),
                },
                ShortcutBinding {
                    action: ShortcutAction::DrawKey,
                    keys: "right_option".to_string(),
                },
            ]
        );
    }

    fn armed_with(settings: &BTreeMap<ShortcutAction, String>) -> LiveShortcuts {
        let mut live = LiveShortcuts::default();
        for action in ShortcutAction::ALL {
            let keys = settings
                .get(&action)
                .cloned()
                .unwrap_or_else(|| default_for(action));
            live.set(action, (!uses_modifier_tap(action, &keys)).then_some(keys));
        }
        live
    }

    /// Start from the spec defaults, as startup leaves them: the plugin keys
    /// registered, the tap armed and tracked by the live map.
    fn armed() -> (
        FakePlatform,
        LiveShortcuts,
        BTreeMap<ShortcutAction, String>,
    ) {
        let current = default_bindings(&BTreeMap::new());
        let platform = FakePlatform::default();
        for action in [ShortcutAction::Session, ShortcutAction::Screenshot] {
            let spec = spec_keys(&current[&action]).expect("a plugin key");
            platform.registered.borrow_mut().insert(spec.clone());
            platform.pressed.borrow_mut().insert(spec, action);
        }
        (platform, armed_with(&current), current)
    }

    #[test]
    fn set_reregisters_only_that_action() {
        let (platform, mut live, current) = armed();

        apply(
            &platform,
            &mut live,
            &current,
            ShortcutAction::Session,
            "Option+Super+J",
            false,
        )
        .expect("J is free");

        assert_eq!(
            platform.registered(),
            vec!["Alt+Super+KeyJ".to_string(), "Alt+Super+KeyS".to_string()],
            "the screenshot still works; the chord is the tap, not a plugin key"
        );
        assert_eq!(
            platform.press("Alt+Super+KeyK"),
            None,
            "the session's old key was released"
        );
        assert_eq!(*platform.taps.borrow(), 0, "the tap was never restarted");
        assert_eq!(
            platform.press("Alt+Super+KeyJ"),
            Some(ShortcutAction::Session)
        );
    }

    #[test]
    fn moving_the_dictation_key_off_the_chord_arms_one_plugin_key() {
        let (platform, mut live, current) = armed();

        apply(
            &platform,
            &mut live,
            &current,
            ShortcutAction::Dictation,
            "F8",
            false,
        )
        .expect("F8 is free");

        assert_eq!(platform.press("F8"), Some(ShortcutAction::Dictation));
        assert_eq!(
            platform.registered().len(),
            3,
            "F8 joins session and screenshot"
        );
    }

    #[test]
    fn the_chord_hands_over_from_the_plugin_key() {
        let current = map(&[(ShortcutAction::Dictation, "F8")]);
        let platform = FakePlatform::default();
        let mut live = armed_with(&current);
        apply(
            &platform,
            &mut live,
            &current,
            ShortcutAction::Dictation,
            "F8",
            false,
        )
        .expect("F8 arms");
        let mut current = current;
        current.insert(ShortcutAction::Dictation, "F8".into());

        apply(
            &platform,
            &mut live,
            &current,
            ShortcutAction::Dictation,
            "Super+Control",
            false,
        )
        .expect("the chord");

        assert_eq!(platform.press("F8"), None, "F8 was released");
        assert_eq!(
            live.get(ShortcutAction::Dictation),
            None,
            "the chord is not a plugin key"
        );
        assert_eq!(*platform.taps.borrow(), 1);
    }

    #[test]
    fn the_conflict_is_refused_before_the_os_is_touched() {
        let (platform, mut live, current) = armed();

        let e = apply(
            &platform,
            &mut live,
            &current,
            ShortcutAction::Dictation,
            "Option+Super+K",
            false,
        )
        .expect_err("the session holds it");

        assert_eq!(e.kind, "conflict");
        assert_eq!(e.message, "Already used for Start / stop a session");
        assert!(!platform
            .calls()
            .iter()
            .any(|call| call.starts_with("disarm")));
        assert_eq!(
            platform.press("Alt+Super+KeyK"),
            Some(ShortcutAction::Session)
        );
    }

    #[test]
    fn register_failure_keeps_previous() {
        let (platform, mut live, current) = armed();
        let platform = FakePlatform {
            refuse: vec!["Super+Option+X"],
            ..platform
        };

        let e = apply(
            &platform,
            &mut live,
            &current,
            ShortcutAction::Screenshot,
            "Super+Option+X",
            false,
        )
        .expect_err("the OS refused");

        assert_eq!(e.kind, "registerFailed");
        assert_eq!(
            platform.press("Alt+Super+KeyS"),
            Some(ShortcutAction::Screenshot),
            "the previous key was put back"
        );
    }

    #[test]
    fn a_refused_registration_never_reaches_the_live_map_or_settings() {
        let current = default_bindings(&BTreeMap::new());
        let platform = FakePlatform::refusing("Option+Super+J");
        let mut live = LiveShortcuts::default();

        assert!(apply(
            &platform,
            &mut live,
            &current,
            ShortcutAction::Session,
            "Option+Super+J",
            false
        )
        .is_err());

        assert_eq!(live.get(ShortcutAction::Session), None);
    }

    #[test]
    fn the_modifier_chord_is_the_tap_and_other_keys_the_plugin() {
        let (platform, mut live, current) = armed();

        apply(
            &platform,
            &mut live,
            &current,
            ShortcutAction::Dictation,
            "F8",
            false,
        )
        .expect("F8");
        assert_eq!(platform.press("F8"), Some(ShortcutAction::Dictation));

        apply(
            &platform,
            &mut live,
            &current,
            ShortcutAction::Dictation,
            "Super+Control",
            false,
        )
        .expect("the chord");

        assert_eq!(platform.press("F8"), None, "F8 was released");
        assert_eq!(
            live.get(ShortcutAction::Dictation),
            None,
            "the chord is not a plugin key"
        );
        assert_eq!(*platform.taps.borrow(), 1);
        assert_eq!(
            platform.registered().len(),
            2,
            "session and screenshot only"
        );
    }

    #[test]
    fn a_draw_key_change_touches_no_registration() {
        let (platform, mut live, current) = armed();
        let before = platform.registered();

        let stored = apply(
            &platform,
            &mut live,
            &current,
            ShortcutAction::DrawKey,
            "Fn",
            false,
        )
        .expect("an allowed draw key");

        assert_eq!(stored, "fn");
        assert_eq!(platform.registered(), before);
        assert_eq!(*platform.taps.borrow(), 0);
    }

    #[test]
    fn restoring_defaults_never_frees_the_draw_key_or_drops_a_held_key() {
        let current = map(&[
            (ShortcutAction::Dictation, "F8"),
            (ShortcutAction::Session, "Super+Space"),
            (ShortcutAction::Screenshot, "Super+Shift+Digit4"),
            (ShortcutAction::DrawKey, "fn"),
        ]);

        let restored = default_bindings(&current);

        assert_eq!(restored[&ShortcutAction::Dictation], "Super+Control");
        assert_eq!(restored[&ShortcutAction::Session], "Option+Super+K");
        assert_eq!(restored[&ShortcutAction::Screenshot], "Option+Super+S");
        assert_eq!(restored[&ShortcutAction::DrawKey], "right_option");
        assert_eq!(
            restored.len(),
            ShortcutAction::ALL.len(),
            "every global action keeps a key"
        );
    }

    #[test]
    fn actions_serialize_camel_case() {
        let cases = [
            (ShortcutAction::Dictation, "dictation"),
            (ShortcutAction::Session, "session"),
            (ShortcutAction::Screenshot, "screenshot"),
            (ShortcutAction::DrawKey, "drawKey"),
        ];
        for (action, name) in cases {
            assert_eq!(serde_json::to_value(action).unwrap(), name);
            assert_eq!(
                serde_json::from_value::<ShortcutAction>(name.into()).unwrap(),
                action
            );
        }
    }
}
