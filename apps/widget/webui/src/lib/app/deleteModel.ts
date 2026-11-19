import { invoke } from '@tauri-apps/api/core';

// P1-R2 adds `deleteModel` to bindings.ts. When it merges, replace this file's body with:
// export { deleteModel } from '../bindings';
export const deleteModel = (id: string) => invoke<null>('delete_model', { id });
