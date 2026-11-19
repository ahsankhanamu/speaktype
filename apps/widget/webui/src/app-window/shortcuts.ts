import { invoke } from '@tauri-apps/api/core';

export type ShortcutAction = 'dictation' | 'session' | 'screenshot' | 'drawKey';
export interface ShortcutBinding {
  action: ShortcutAction;
  keys: string;
}

/** Rejections carry R5's `IpcError`: `conflict`, `systemShortcut`, `registerFailed` or `invalid`. */
export const shortcutsGet = () => invoke<ShortcutBinding[]>('shortcuts_get');
export const shortcutSet = (action: ShortcutAction, keys: string, force: boolean) =>
  invoke<null>('shortcut_set', { action, keys, force });
export const shortcutsReset = () => invoke<ShortcutBinding[]>('shortcuts_reset');
