import { commands } from '../lib/bindings';

export type { ShortcutAction, ShortcutBinding } from '../lib/bindings';

/** Rejections carry an `IpcError`: `conflict`, `systemShortcut`, `registerFailed` or `invalid`. */
export const { shortcutsGet, shortcutSet, shortcutsReset } = commands;
