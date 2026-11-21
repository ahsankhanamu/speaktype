import { toast } from '@speaktype/ui';
import { liveQuery, useEvent } from '../data';
import {
  cancelModelDownload,
  getModels,
  getSettings,
  loadModel,
  pauseModelDownload,
  queueModelDownload,
} from '../ipc';
import { deleteModel } from './deleteModel';
import { MODEL_ORDER, modelState, plainModel, type ModelInfo, type ModelProgress, type ModelState } from './models';

export interface ModelRow {
  model: ModelInfo;
  name: string;
  state: ModelState;
}

const baseId = (id: string) => id.replace(/\.en$/, '');
const REFETCH_ON = new Set(['done', 'error', 'cancelled']);

/** Starts or stops a download, or switches to a downloaded model. */
export async function runModelAction(action: NonNullable<ModelState['action']>, id: string) {
  const result = (await {
    use: () => loadModel(id),
    download: () => queueModelDownload(id, false),
    resume: () => queueModelDownload(id, false),
    retry: () => queueModelDownload(id, false),
    pause: () => pauseModelDownload(id),
    cancel: () => cancelModelDownload(id),
  }[action]()) as { status?: string; message?: string } | null;
  if (result?.status === 'busy' || result?.status === 'error') {
    toast.show({ message: result.message ?? 'Something went wrong' });
  }
}

/**
 * The six catalog models in slider order, with live download state. Call during component init.
 * Progress is kept from the event payloads; the catalog is refetched only when a download ends,
 * so a running download does not hit the disk on every tick.
 */
export function useModels() {
  const catalog = liveQuery('models', getModels, { invalidateOn: ['settings-changed'] });
  const settings = liveQuery('settings', getSettings, { invalidateOn: ['settings-changed'] });

  const live = $state<Record<string, ModelProgress>>({});
  const reportingSince: Record<string, number> = {};
  useEvent<ModelProgress & { model: string }>('model:progress', async ({ model, ...progress }) => {
    if (progress.phase === 'downloading') reportingSince[model] ??= Date.now();
    else delete reportingSince[model];
    const since = reportingSince[model];
    live[model] = { ...progress, elapsed_ms: since === undefined ? undefined : Date.now() - since };
    if (!REFETCH_ON.has(progress.phase)) return;
    await catalog.refetch();
    if (progress.phase !== 'error') delete live[model];
  });

  const activeId = $derived(settings.data?.model ?? '');

  const rows = $derived.by((): ModelRow[] => {
    const data = catalog.data;
    if (!data) return [];
    const english = settings.data?.language === 'en';
    const fetched = Object.fromEntries(data.progress.map(({ model, ...p }) => [model, p]));
    const downloadingNow = Object.keys(live).find((id) => live[id].phase === 'downloading') ?? data.downloading[0];
    const after = downloadingNow ? plainModel(downloadingNow).name : undefined;
    return MODEL_ORDER.map((base) => {
      const variants = data.available.filter((m) => baseId(m.id) === base);
      const model =
        variants.find((m) => m.id === activeId) ??
        variants.find((m) => m.id.endsWith('.en') === english) ??
        variants[0];
      const queued = data.waiting.includes(model.id) ? { phase: 'waiting' } : undefined;
      const progress = live[model.id] ?? fetched[model.id] ?? queued;
      return {
        model,
        name: plainModel(model.id).name,
        state: modelState(model, progress?.phase === 'waiting' ? { ...progress, after } : progress, activeId),
      };
    });
  });

  return {
    get rows() { return rows; },
    get activeId() { return activeId; },
    get loaded() { return catalog.data !== undefined; },
    get bytesOnDisk() {
      return (catalog.data?.available ?? []).reduce((sum, m) => sum + (m.downloaded ? (m.size_bytes ?? 0) : 0), 0);
    },
    async remove(id: string) {
      await deleteModel(id);
      await catalog.refetch();
      toast.show({ message: `Deleted ${plainModel(id).name}` });
    },
  };
}
