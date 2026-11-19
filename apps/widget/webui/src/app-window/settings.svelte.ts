import { toast } from '@speaktype/ui';
import { liveQuery } from '../lib/data';
import { getSettings, saveSettings, type Settings } from '../lib/ipc';
import type { SettingsV2 } from './pending';

/** The settings file as a live query: the widget changing a setting updates every page. */
export function useSettings() {
  const query = liveQuery('settings', getSettings, { invalidateOn: ['settings-changed'] });
  const data = $derived(query.data as SettingsV2 | undefined);

  async function save(patch: Partial<SettingsV2>) {
    try {
      await saveSettings({ ...(await getSettings()), ...patch } as Settings);
      await query.refetch();
    } catch (error) {
      toast.show({ message: `Couldn't save that setting. ${(error as { message?: string })?.message ?? error}` });
    }
  }

  /** A two-way binding target: reads the live value, writes through `save`. */
  function field<K extends keyof SettingsV2>(key: K, fallback: NonNullable<SettingsV2[K]>) {
    return {
      get value(): NonNullable<SettingsV2[K]> {
        return (data?.[key] ?? fallback) as NonNullable<SettingsV2[K]>;
      },
      set value(next: SettingsV2[K]) {
        void save({ [key]: next } as Partial<SettingsV2>);
      },
    };
  }

  return {
    get data() { return data; },
    save,
    field,
    refetch: () => query.refetch(),
  };
}
