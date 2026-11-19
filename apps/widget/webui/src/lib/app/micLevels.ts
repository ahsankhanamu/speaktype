import { invoke } from '@tauri-apps/api/core';

// P1-R2 adds `micLevelsStart` / `micLevelsStop` to bindings.ts. When it merges,
// replace this file's body with: export { micLevelsStart, micLevelsStop } from '../bindings';
export const micLevelsStart = () => invoke<null>('mic_levels_start');
export const micLevelsStop = () => invoke<null>('mic_levels_stop');
