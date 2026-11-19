use tauri_specta::{collect_commands, collect_events, Builder};

/// The one typed-IPC builder: every command and event the frontend can reach,
/// in one place, so `bindings.ts` covers the whole app.
pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        // The app's types have identical serialize/deserialize shapes; a
        // single phase keeps the generated types plain objects.
        .disable_serde_phases()
        // Safe here: the only wide integers on the IPC surface are SQLite ids,
        // epoch-milliseconds and byte counts — every real value sits far below
        // Number.MAX_SAFE_INTEGER (2^53). The plan fixes these as i64/u64.
        .dangerously_cast_bigints_to_number()
        .commands(collect_commands![
            crate::commands::toggle_recording,
            crate::commands::cancel_recording,
            crate::commands::check_permissions,
            crate::commands::start_mic_test,
            crate::commands::stop_mic_test,
            crate::commands::get_audio_input_info,
            crate::commands::get_input_devices,
            crate::commands::log_frontend,
            crate::commands::copy_to_clipboard,
            crate::commands::request_accessibility,
            crate::commands::request_microphone_access,
            crate::commands::get_onboarding_status,
            crate::commands::open_onboarding,
            crate::commands::finish_onboarding,
            crate::commands::open_system_pane,
            crate::commands::open_models_folder,
            crate::commands::open_recordings_folder,
            crate::commands::get_settings,
            crate::commands::save_settings,
            crate::commands::set_launch_at_login,
            crate::commands::shortcuts_get,
            crate::commands::shortcut_set,
            crate::commands::shortcuts_reset,
            crate::commands::check_server,
            crate::commands::get_server_status,
            crate::commands::stop_whisper_server,
            crate::commands::unload_model,
            crate::commands::load_server,
            crate::commands::server_info,
            crate::commands::restart_whisper_server,
            crate::commands::load_model,
            crate::commands::queue_model_download,
            crate::commands::restart_model_download,
            crate::commands::cancel_model_download,
            crate::commands::pause_model_download,
            crate::commands::get_models,
            crate::commands::get_model_progress,
            crate::commands::open_about,
            crate::commands::open_settings,
            crate::commands::hide_widget,
            crate::commands::minimize_widget,
            crate::commands::quit_app,
            crate::commands::save_window_position,
            crate::commands::reset_widget_position,
            crate::commands::get_history,
            crate::commands::delete_history_entry,
            crate::commands::delete_history_audio,
            crate::commands::clear_history,
            crate::commands::get_history_audio,
            crate::commands::reprocess_history_entry,
            crate::commands::paste_last,
            crate::commands::delete_model,
            crate::commands::mic_levels_start,
            crate::commands::mic_levels_stop,
            crate::commands::get_debug_sessions,
            crate::commands::clear_debug_sessions,
            crate::commands::delete_debug_session,
            crate::commands::get_debug_audio_slice,
            crate::commands::reveal_path,
            crate::widget_cmds::set_widget_corner,
            crate::widget_cmds::screen_recording_status,
            crate::widget_cmds::request_screen_recording,
            crate::widget_cmds::test_typing,
            crate::widget_cmds::reprocess_preview,
            crate::session_cmds::session_start,
            crate::session_cmds::session_stop,
            crate::session_cmds::session_active,
            crate::session_cmds::session_set_mute,
            crate::session_cmds::session_list,
            crate::session_cmds::session_get,
            crate::session_cmds::session_rename,
            crate::session_cmds::session_delete,
            crate::session_cmds::item_update,
            crate::session_cmds::item_delete,
            crate::session_cmds::item_restore,
            crate::session_cmds::item_move,
            crate::session_cmds::item_split,
            crate::session_cmds::prompt_render,
            crate::session_cmds::prompt_copy,
            crate::session_cmds::bundle_reveal,
            crate::session_cmds::overlay_pen,
            crate::session_cmds::overlay_mark_complete,
            crate::stt_cmds::stt_set_provider,
            crate::stt_cmds::stt_set_api_key,
            crate::stt_cmds::stt_has_api_key,
            crate::stt_cmds::stt_reveal_api_key,
            crate::stt_cmds::stt_list_models,
            crate::stt_cmds::stt_test,
        ])
        .events(collect_events![
            crate::session_cmds::SessionState,
            crate::session_cmds::SessionItemAdded,
            crate::session_cmds::SessionItemUpdated,
            crate::session_cmds::SessionItemsDeleted,
            crate::session_cmds::SessionsChanged,
            crate::settings::SettingsChanged,
            crate::controller::DictationState,
            crate::history::HistoryChanged,
            crate::widget_cmds::MicDevicesChanged,
            crate::widget_cmds::WidgetCornerChanged,
            crate::shortcuts::ShortcutsChanged,
        ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use specta_typescript::Typescript;

    /// The committed bindings the webui imports. Resolved at run time: a shared
    /// target dir may hold a test binary compiled in another worktree.
    fn bindings_path() -> std::path::PathBuf {
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("set by cargo test"))
            .join("../webui/src/lib/bindings.ts")
    }

    #[test]
    fn export_bindings() {
        let dir =
            std::env::temp_dir().join(format!("speaktype_bindings_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("tempdir");
        let out = dir.join("bindings.ts");
        builder()
            .export(Typescript::default(), &out)
            .expect("export bindings");
        let generated = std::fs::read_to_string(&out).expect("read generated");
        let committed =
            std::fs::read_to_string(bindings_path()).expect("committed bindings.ts is missing");
        assert_eq!(
            generated, committed,
            "apps/widget/webui/src/lib/bindings.ts is stale — regenerate with \
             `cargo test -p speaktype export_bindings -- --ignored --nocapture`"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn session_events_are_registered_in_bindings() {
        let bindings = std::fs::read_to_string(bindings_path()).expect("bindings");
        for name in [
            "\"session-state\"",
            "\"session-item-added\"",
            "\"session-item-updated\"",
            "\"session-items-deleted\"",
            "\"sessions-changed\"",
            "\"settings-changed\"",
        ] {
            assert!(bindings.contains(name), "bindings.ts must register {name}");
        }
    }

    #[test]
    fn stt_commands_are_registered_in_bindings() {
        let bindings = std::fs::read_to_string(bindings_path()).expect("bindings");
        for name in [
            "sttSetProvider",
            "sttSetApiKey",
            "sttHasApiKey",
            "sttRevealApiKey",
            "sttListModels",
            "sttTest",
        ] {
            assert!(
                bindings.contains(&format!("{name}: (")),
                "bindings.ts must export {name}"
            );
        }
    }

    #[test]
    #[ignore = "writer variant: regenerate apps/widget/webui/src/lib/bindings.ts"]
    fn export_bindings_write() {
        builder()
            .export(Typescript::default(), bindings_path())
            .expect("export bindings");
    }
}
