// Everything here stands in for a contract that is not merged yet.
// Each block names the workstream whose merge deletes it.
import { invoke } from '@tauri-apps/api/core';
import type { HistoryEntry, Settings } from '../lib/ipc';

// REMOVE when P1-R1 merges: these fields arrive in the generated `Settings` type.
export type SettingsV2 = Settings & {
  sounds?: boolean;
  widget_size?: 'small' | 'regular' | 'large';
  widget_visibility?: 'always' | 'active' | 'never';
  ink_color?: 'orange' | 'red' | 'amber' | 'green' | 'blue' | 'purple' | 'white';
  ink_thickness?: 'thin' | 'regular' | 'bold';
  draw_key?: 'right_option' | 'right_command' | 'fn';
  draw_anytime?: boolean;
  history_limit?: number | null;
  launch_at_login?: boolean;
  onboarding_step?: number;
};

// REMOVE when P1-R1 merges: use `commands.setLaunchAtLogin`.
export const setLaunchAtLogin = (on: boolean) => invoke<null>('set_launch_at_login', { on });

// REMOVE when P1-R2 merges: `outcome` arrives in the generated `HistoryEntry` type.
export type HistoryEntryV2 = HistoryEntry & { outcome?: 'pasted' | 'copied' | 'pasteFailed' | null };

// REMOVE when P1-R2 merges: use `commands.pasteLast`.
export const pasteLast = () => invoke<null>('paste_last');

// NOT IN ANY CONTRACT (open question 1): Permissions > Test typing needs a Rust
// command that pastes "SpeakType test" into the focused field.
export const testTyping = () => invoke<null>('test_typing');
