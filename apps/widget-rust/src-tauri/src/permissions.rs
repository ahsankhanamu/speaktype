#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::c_void;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFDictionaryCreate(
            allocator: *const c_void,
            keys: *const *const c_void,
            values: *const *const c_void,
            num_values: isize,
            key_callbacks: *const c_void,
            value_callbacks: *const c_void,
        ) -> *const c_void;
        fn CFRelease(cf: *const c_void);
        static kCFTypeDictionaryKeyCallBacks: c_void;
        static kCFTypeDictionaryValueCallBacks: c_void;
        static kCFBooleanTrue: *const c_void;
    }

    extern "C" {
        static kAXTrustedCheckOptionPrompt: *const c_void;
    }

    fn current_exe_path() -> String {
        std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "(unknown)".into())
    }

    /// Check if accessibility is granted. If `prompt` is true, macOS will show
    /// the system dialog asking the user to enable it.
    ///
    /// Note: TCC grants are per code signature. Enabling "SpeakType" in System
    /// Settings for an old/ad-hoc build does not trust a newly signed
    /// /Applications copy (or vice versa). A running process also typically
    /// stays untrusted until it is fully quit and relaunched after the toggle.
    pub fn check_accessibility(prompt: bool) -> bool {
        unsafe {
            if prompt {
                let keys = [kAXTrustedCheckOptionPrompt];
                let values = [kCFBooleanTrue];
                let options = CFDictionaryCreate(
                    std::ptr::null(),
                    keys.as_ptr(),
                    values.as_ptr(),
                    1,
                    &kCFTypeDictionaryKeyCallBacks,
                    &kCFTypeDictionaryValueCallBacks,
                );
                if options.is_null() {
                    return AXIsProcessTrusted();
                }
                let result = AXIsProcessTrustedWithOptions(options);
                CFRelease(options);
                result || AXIsProcessTrusted()
            } else {
                AXIsProcessTrusted() || AXIsProcessTrustedWithOptions(std::ptr::null())
            }
        }
    }

    pub fn accessibility_exe_path() -> String {
        current_exe_path()
    }

    /// True when running a cargo/tauri debug build (not /Applications).
    pub fn is_dev_binary() -> bool {
        current_exe_path().contains("/target/debug/")
    }

    /// True when the executable lives inside an .app bundle. Bundled apps opened
    /// via LaunchServices are responsible for their own TCC grants; bare
    /// executables are not.
    pub fn is_bundled() -> bool {
        current_exe_path().contains(".app/Contents/MacOS/")
    }

    /// macOS attributes TCC requests to the *responsible process*. A bare
    /// executable spawned from a shell inherits responsibility from the
    /// terminal app, so Accessibility must be granted to that app — granting it
    /// to the executable itself has no effect.
    ///
    /// Walks the parent chain and returns the outermost enclosing `.app` bundle.
    pub fn responsible_app_path() -> Option<String> {
        use std::process::Command;

        // A bundled app opened via LaunchServices is responsible for itself.
        let exe = current_exe_path();
        if let Some(idx) = exe.find(".app/") {
            if exe.contains(".app/Contents/MacOS/") {
                return Some(exe[..idx + 4].to_string());
            }
        }

        let mut pid = std::process::id().to_string();

        for _ in 0..12 {
            // `comm=` is the executable path without arguments, so command-line
            // text cannot produce a false match.
            let output = Command::new("ps")
                .args(["-o", "ppid=,comm=", "-p", &pid])
                .output()
                .ok()?;
            let line = String::from_utf8_lossy(&output.stdout);
            let line = line.trim();
            if line.is_empty() {
                return None;
            }

            let (ppid, exe) = line.split_once(char::is_whitespace)?;
            let exe = exe.trim();

            // Take the outermost bundle so helper processes resolve to the
            // parent app (e.g. "Cursor Helper.app" -> "Cursor.app").
            if exe.contains(".app/Contents/MacOS/") {
                if let Some(idx) = exe.find(".app/") {
                    return Some(exe[..idx + 4].to_string());
                }
            }

            let ppid = ppid.trim();
            if ppid.is_empty() || ppid == "1" || ppid == "0" || ppid == pid {
                return None;
            }
            pid = ppid.to_string();
        }

        None
    }

    /// Display name of the responsible app as System Settings lists it, e.g.
    /// "Cursor" or "SpeakType Dev".
    pub fn responsible_app_name() -> Option<String> {
        let path = responsible_app_path()?;

        // System Settings shows the bundle's display name, which can differ
        // from the folder name.
        for key in ["CFBundleDisplayName", "CFBundleName"] {
            let plist = format!("{}/Contents/Info.plist", path);
            let output = std::process::Command::new("defaults")
                .args(["read", &plist, key])
                .output();
            if let Ok(output) = output {
                if output.status.success() {
                    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !name.is_empty() {
                        return Some(name);
                    }
                }
            }
        }

        let name = std::path::Path::new(&path).file_stem()?.to_string_lossy();
        Some(name.to_string())
    }

    /// Check microphone authorization WITHOUT prompting. Returns true if authorized.
    /// Safe to call repeatedly (e.g. polling) — it never shows the system dialog.
    pub fn check_microphone() -> bool {
        use std::process::Command;
        use std::sync::mpsc;
        use std::time::Duration;
        use std::thread;

        let script = r#"
            use framework "AVFoundation"
            set status to (current application's AVCaptureDevice's authorizationStatusForMediaType:(current application's AVMediaTypeAudio)) as integer
            return status as text
        "#;

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = Command::new("osascript")
                .arg("-l").arg("AppleScript")
                .arg("-e").arg(script)
                .output();
            let _ = tx.send(result);
        });

        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(output)) => String::from_utf8_lossy(&output.stdout).trim() == "3",
            _ => false,
        }
    }

    /// Request microphone permission. Returns true if authorized.
    pub fn request_microphone() -> bool {
        use std::process::Command;
        use std::sync::mpsc;
        use std::time::Duration;
        use std::thread;

        let script = r#"
            use framework "AVFoundation"
            set status to (current application's AVCaptureDevice's authorizationStatusForMediaType:(current application's AVMediaTypeAudio)) as integer
            if status is 0 then
                -- Not determined: request access
                current application's AVCaptureDevice's requestAccessForMediaType:(current application's AVMediaTypeAudio) completionHandler:(missing value)
                delay 0.5
                set status to (current application's AVCaptureDevice's authorizationStatusForMediaType:(current application's AVMediaTypeAudio)) as integer
            end if
            return status as text
        "#;

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = Command::new("osascript")
                .arg("-l").arg("AppleScript")
                .arg("-e").arg(script)
                .output();
            let _ = tx.send(result);
        });

        match rx.recv_timeout(Duration::from_secs(10)) {
            Ok(Ok(output)) => {
                let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
                // AVAuthorizationStatus: 0=notDetermined, 1=restricted, 2=denied, 3=authorized
                status == "3"
            }
            _ => false,
        }
    }
}

#[cfg(target_os = "macos")]
pub use macos::{
    accessibility_exe_path, check_accessibility, check_microphone, is_bundled, is_dev_binary,
    request_microphone, responsible_app_name, responsible_app_path,
};

#[cfg(not(target_os = "macos"))]
pub fn check_accessibility(_prompt: bool) -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
pub fn accessibility_exe_path() -> String {
    String::new()
}

#[cfg(not(target_os = "macos"))]
pub fn is_dev_binary() -> bool {
    false
}

#[cfg(not(target_os = "macos"))]
pub fn is_bundled() -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
pub fn responsible_app_path() -> Option<String> {
    None
}

#[cfg(not(target_os = "macos"))]
pub fn responsible_app_name() -> Option<String> {
    None
}

#[cfg(not(target_os = "macos"))]
pub fn check_microphone() -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
pub fn request_microphone() -> bool {
    true
}
