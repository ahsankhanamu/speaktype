#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::c_void;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
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

    /// Check if accessibility is granted. If `prompt` is true, macOS will show
    /// the system dialog asking the user to enable it.
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
                let result = AXIsProcessTrustedWithOptions(options);
                CFRelease(options);
                result
            } else {
                AXIsProcessTrustedWithOptions(std::ptr::null())
            }
        }
    }

    /// Request microphone permission. Returns true if authorized.
    /// Uses AVCaptureDevice via the Objective-C runtime.
    pub fn request_microphone() -> bool {
        use std::process::Command;
        // Use osascript to trigger the mic permission prompt via AVFoundation.
        // This is the most reliable way without adding heavy AVFoundation bindings.
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

        match Command::new("osascript").arg("-l").arg("AppleScript").arg("-e").arg(script).output() {
            Ok(output) => {
                let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
                // AVAuthorizationStatus: 0=notDetermined, 1=restricted, 2=denied, 3=authorized
                status == "3"
            }
            Err(_) => false,
        }
    }
}

#[cfg(target_os = "macos")]
pub use macos::{check_accessibility, request_microphone};

#[cfg(not(target_os = "macos"))]
pub fn check_accessibility(_prompt: bool) -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
pub fn request_microphone() -> bool {
    true
}
