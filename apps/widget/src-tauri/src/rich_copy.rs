//! A session copy that every app can paste: one pasteboard item carrying plain
//! text (words and image paths, for terminals and coding agents), HTML with the
//! images inline (browsers, Docs, Slack), RTFD with the images attached (Notes,
//! Mail, Pages) and a private marker so SpeakType's own clipboard watcher never
//! records the copy as a clip. The app being pasted into takes what it reads.

/// The marker type on every session copy.
pub const MARKER: &str = "com.speaktype.session-copy";

/// What goes on the pasteboard: the plain text, the HTML with inline images,
/// and the HTML with `file://` images that rich text is made from.
pub struct SessionCopy<'a> {
    pub text: &'a str,
    pub html: &'a str,
    pub html_for_rich_text: &'a str,
}

#[cfg(target_os = "macos")]
mod mac {
    use super::{SessionCopy, MARKER};
    use objc2::rc::Retained;
    use objc2::AnyThread;
    use objc2_app_kit::{
        NSAttributedStringAppKitDocumentFormats, NSPasteboard, NSPasteboardTypeHTML,
        NSPasteboardTypeRTFD, NSPasteboardTypeString,
    };
    use objc2_foundation::{NSAttributedString, NSData, NSDictionary, NSRange, NSString};

    /// RTFD made from HTML whose images are `file://` URLs. AppKit's HTML import
    /// uses WebKit, so this must run on the main thread.
    pub fn rtfd_from_html(html: &str) -> Option<Retained<NSData>> {
        let data = NSData::with_bytes(html.as_bytes());
        unsafe {
            let text = NSAttributedString::initWithHTML_documentAttributes(
                NSAttributedString::alloc(),
                &data,
                None,
            )?;
            text.RTFDFromRange_documentAttributes(
                NSRange::new(0, text.length()),
                &NSDictionary::new(),
            )
        }
    }

    pub fn write(board: &NSPasteboard, copy: &SessionCopy, rtfd: Option<&NSData>) -> bool {
        let marker = NSString::from_str(MARKER);
        unsafe {
            board.clearContents();
            let mut ok =
                board.setString_forType(&NSString::from_str(copy.text), NSPasteboardTypeString);
            ok &= board.setString_forType(&NSString::from_str(copy.html), NSPasteboardTypeHTML);
            if let Some(rtfd) = rtfd {
                ok &= board.setData_forType(Some(rtfd), NSPasteboardTypeRTFD);
            }
            ok &= board.setData_forType(Some(&NSData::new()), &marker);
            ok
        }
    }

    pub fn has_marker(board: &NSPasteboard) -> bool {
        let marker = NSString::from_str(MARKER);
        board
            .types()
            .is_some_and(|types| types.iter().any(|t| t.isEqualToString(&marker)))
    }
}

/// Put a session copy on the clipboard. Rich text is built on the main thread
/// (AppKit requires it); the pasteboard write itself is thread-safe.
#[cfg(target_os = "macos")]
pub fn write_session_copy(app: &tauri::AppHandle, copy: SessionCopy) -> Result<(), String> {
    use objc2_foundation::MainThreadMarker;
    let rich_html = copy.html_for_rich_text.to_owned();
    let rtfd = if MainThreadMarker::new().is_some() {
        mac::rtfd_from_html(&rich_html).map(|d| d.to_vec())
    } else {
        let (tx, rx) = std::sync::mpsc::channel();
        app.run_on_main_thread(move || {
            let _ = tx.send(mac::rtfd_from_html(&rich_html).map(|d| d.to_vec()));
        })
        .map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())?
    };
    let rtfd = rtfd.map(|bytes| objc2_foundation::NSData::with_bytes(&bytes));
    let board = objc2_app_kit::NSPasteboard::generalPasteboard();
    if mac::write(&board, &copy, rtfd.as_deref()) {
        Ok(())
    } else {
        Err("The clipboard didn't accept the copy".into())
    }
}

#[cfg(not(target_os = "macos"))]
pub fn write_session_copy(_app: &tauri::AppHandle, copy: SessionCopy) -> Result<(), String> {
    crate::commands::copy_to_clipboard(copy.text.to_owned())
}

/// Whether the clipboard holds a session copy SpeakType made.
pub fn is_session_copy() -> bool {
    #[cfg(target_os = "macos")]
    {
        mac::has_marker(&objc2_app_kit::NSPasteboard::generalPasteboard())
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeHTML, NSPasteboardTypeString};

    #[test]
    fn a_session_copy_carries_text_html_and_the_marker() {
        // A private pasteboard: the person's clipboard is never touched.
        let board = NSPasteboard::pasteboardWithUniqueName();
        let copy = SessionCopy {
            text: "hello ![Mark 1](</a b.png>)",
            html: "<p>hello</p>",
            html_for_rich_text: "<p>hello</p>",
        };
        assert!(mac::write(&board, &copy, None));
        let text = unsafe { board.stringForType(NSPasteboardTypeString) }.expect("text");
        let html = unsafe { board.stringForType(NSPasteboardTypeHTML) }.expect("html");
        assert_eq!(text.to_string(), copy.text);
        assert_eq!(html.to_string(), copy.html);
        assert!(mac::has_marker(&board));
    }

    #[test]
    fn an_ordinary_copy_has_no_marker() {
        let board = NSPasteboard::pasteboardWithUniqueName();
        unsafe {
            board.clearContents();
            board.setString_forType(
                &objc2_foundation::NSString::from_str("copied elsewhere"),
                NSPasteboardTypeString,
            );
        }
        assert!(!mac::has_marker(&board));
    }
}
