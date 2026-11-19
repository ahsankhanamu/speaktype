import { liveQuery } from '../lib/data';
import { shortcutsGet, type ShortcutAction } from './shortcuts';

/** The shortcut registry as a live query, refreshed by `shortcuts-changed`. */
export function useShortcuts() {
  const query = liveQuery('shortcuts', shortcutsGet, { invalidateOn: ['shortcuts-changed'] });
  return {
    keys: (action: ShortcutAction) => query.data?.find((binding) => binding.action === action)?.keys ?? '',
  };
}
