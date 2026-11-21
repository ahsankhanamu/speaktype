//! What has keyboard focus in the app SpeakType is about to paste into, read
//! through the Accessibility API, so a paste with nowhere to land is caught
//! instead of reported as done.

/// The focused element, as far as pasting is concerned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    /// A text field, text view or editor; `chars` is its text length when the
    /// app reports one, to confirm the paste landed.
    Editable { chars: Option<usize> },
    /// Something focused that text can't go into (a button, a list, a table).
    NotEditable,
    /// The app doesn't say (no AX support, a web area, a custom-drawn view).
    Unknown,
}

/// Roles that never take typed text. Anything else that isn't clearly
/// editable is `Unknown`, so an app that hides its text views from AX still
/// gets its paste.
const NOT_TEXT_ROLES: &[&str] = &[
    "AXButton",
    "AXCheckBox",
    "AXRadioButton",
    "AXList",
    "AXOutline",
    "AXTable",
    "AXRow",
    "AXCell",
    "AXImage",
    "AXStaticText",
    "AXLink",
    "AXMenuItem",
    "AXMenuBarItem",
    "AXTabGroup",
    "AXToolbar",
    "AXSlider",
    "AXPopUpButton",
    "AXDisclosureTriangle",
];
const TEXT_ROLES: &[&str] = &["AXTextField", "AXTextArea", "AXComboBox", "AXSearchField"];

/// Classify a focused element from what AX said about it.
pub fn classify(
    role: Option<&str>,
    value_settable: bool,
    has_selection: bool,
    chars: Option<usize>,
) -> Focus {
    match role {
        Some(role) if TEXT_ROLES.contains(&role) || value_settable || has_selection => {
            Focus::Editable { chars }
        }
        Some(role) if NOT_TEXT_ROLES.contains(&role) => Focus::NotEditable,
        _ => Focus::Unknown,
    }
}

/// Whether a paste visibly landed: the field's text changed (it grows, or a
/// selection was replaced). `None` when the app
/// doesn't report lengths, so it can't be told.
pub fn landed(before: Focus, after: Focus) -> Option<bool> {
    match (before, after) {
        (Focus::Editable { chars: Some(a) }, Focus::Editable { chars: Some(b) }) => Some(b != a),
        _ => None,
    }
}

#[cfg(target_os = "macos")]
mod ax {
    use core_foundation::base::{CFType, CFTypeRef, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::number::CFNumber;
    use core_foundation::string::{CFString, CFStringRef};
    use std::ffi::c_void;

    type AXUIElementRef = *const c_void;
    const AX_SUCCESS: i32 = 0;

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
        fn AXUIElementCopyAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: *mut CFTypeRef,
        ) -> i32;
        fn AXUIElementIsAttributeSettable(
            element: AXUIElementRef,
            attribute: CFStringRef,
            settable: *mut u8,
        ) -> i32;
        fn AXUIElementSetAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: CFTypeRef,
        ) -> i32;
        fn AXUIElementSetMessagingTimeout(element: AXUIElementRef, seconds: f32) -> i32;
    }

    /// An AX element we own (released on drop).
    pub struct Element(CFType);

    impl Element {
        pub fn application(pid: u32) -> Option<Self> {
            // SAFETY: returns a +1 reference (Create rule) or null.
            let raw = unsafe { AXUIElementCreateApplication(pid as i32) };
            if raw.is_null() {
                return None;
            }
            let app = Element(unsafe { CFType::wrap_under_create_rule(raw) });
            // A hung app must not stall the dictation.
            unsafe { AXUIElementSetMessagingTimeout(app.raw(), 0.25) };
            Some(app)
        }

        fn raw(&self) -> AXUIElementRef {
            self.0.as_CFTypeRef()
        }

        pub fn attribute(&self, name: &str) -> Option<CFType> {
            let name = CFString::new(name);
            let mut value: CFTypeRef = std::ptr::null();
            // SAFETY: on success `value` is a +1 reference we take ownership of.
            let err = unsafe {
                AXUIElementCopyAttributeValue(self.raw(), name.as_concrete_TypeRef(), &mut value)
            };
            (err == AX_SUCCESS && !value.is_null())
                .then(|| unsafe { CFType::wrap_under_create_rule(value) })
        }

        pub fn element(&self, name: &str) -> Option<Element> {
            self.attribute(name).map(Element)
        }

        pub fn string(&self, name: &str) -> Option<String> {
            self.attribute(name)?
                .downcast::<CFString>()
                .map(|s| s.to_string())
        }

        pub fn number(&self, name: &str) -> Option<i64> {
            self.attribute(name)?.downcast::<CFNumber>()?.to_i64()
        }

        pub fn settable(&self, name: &str) -> bool {
            let name = CFString::new(name);
            let mut settable = 0u8;
            let err = unsafe {
                AXUIElementIsAttributeSettable(
                    self.raw(),
                    name.as_concrete_TypeRef(),
                    &mut settable,
                )
            };
            err == AX_SUCCESS && settable != 0
        }

        /// Chromium and Electron build their AX tree only when asked.
        pub fn enable_manual_accessibility(&self) {
            let name = CFString::new("AXManualAccessibility");
            unsafe {
                AXUIElementSetAttributeValue(
                    self.raw(),
                    name.as_concrete_TypeRef(),
                    CFBoolean::true_value().as_CFTypeRef(),
                )
            };
        }
    }
}

/// What has keyboard focus in the app with `pid`.
#[cfg(target_os = "macos")]
pub fn probe(pid: u32) -> Focus {
    let Some(app) = ax::Element::application(pid) else {
        return Focus::Unknown;
    };
    app.enable_manual_accessibility();
    let Some(focused) = app.element("AXFocusedUIElement") else {
        return Focus::Unknown;
    };
    let role = focused.string("AXRole");
    let chars = focused
        .number("AXNumberOfCharacters")
        .map(|n| n.max(0) as usize)
        .or_else(|| focused.string("AXValue").map(|v| v.chars().count()));
    classify(
        role.as_deref(),
        focused.settable("AXValue"),
        focused.attribute("AXSelectedTextRange").is_some(),
        chars,
    )
}

#[cfg(not(target_os = "macos"))]
pub fn probe(_pid: u32) -> Focus {
    Focus::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_fields_and_settable_values_are_editable() {
        assert_eq!(
            classify(Some("AXTextField"), false, false, Some(3)),
            Focus::Editable { chars: Some(3) }
        );
        assert_eq!(
            classify(Some("AXGroup"), true, false, None),
            Focus::Editable { chars: None }
        );
        assert_eq!(
            classify(Some("AXWebArea"), false, true, None),
            Focus::Editable { chars: None }
        );
    }

    #[test]
    fn a_focused_button_or_list_cannot_take_a_paste() {
        assert_eq!(
            classify(Some("AXButton"), false, false, None),
            Focus::NotEditable
        );
        assert_eq!(
            classify(Some("AXOutline"), false, false, None),
            Focus::NotEditable
        );
    }

    #[test]
    fn anything_unclear_still_gets_its_paste() {
        assert_eq!(
            classify(Some("AXWebArea"), false, false, None),
            Focus::Unknown
        );
        assert_eq!(
            classify(Some("AXGroup"), false, false, None),
            Focus::Unknown
        );
        assert_eq!(classify(None, false, false, None), Focus::Unknown);
    }

    #[test]
    fn a_paste_landed_only_when_the_text_changed() {
        let at = |n| Focus::Editable { chars: Some(n) };
        assert_eq!(landed(at(10), at(42)), Some(true));
        assert_eq!(landed(at(10), at(10)), Some(false));
        assert_eq!(landed(Focus::Unknown, at(10)), None);
        assert_eq!(landed(Focus::Editable { chars: None }, at(10)), None);
    }
}
