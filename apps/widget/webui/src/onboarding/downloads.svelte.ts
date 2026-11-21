import { plainModel } from '../lib/app/models';
import { runModelAction, useModels, type ModelRow } from '../lib/app/models.svelte';

const STOPS = new Set(['fast', 'balanced', 'accurate']);
const SELECTABLE = new Set(['use', 'download', 'resume', 'retry']);

export function footerText(stops: ModelRow[]): string {
  const ready = stops.filter((row) => row.model.downloaded).map((row) => `${row.name} ready`);
  const loading = stops.filter((row) => row.state.percent !== undefined);
  if (!ready.length && loading.length) return `Downloading the ${loading[0].name} speech model`;
  return [...ready, ...loading.map((row) => `${row.name} ${row.state.percent}%`)].join(' · ');
}

/** The three slider stops with live download state; `choose` switches to a pick, or queues it to load when downloaded. */
export function useOnboardingModels() {
  const models = useModels();
  const stops = $derived(models.rows.filter((row) => STOPS.has(plainModel(row.model.id).tier)));

  function choose(row: ModelRow) {
    if (row.state.action && SELECTABLE.has(row.state.action)) runModelAction('use', row.model.id);
  }

  return {
    get stops() { return stops; },
    get activeId() { return models.activeId; },
    get loaded() { return models.loaded; },
    get footer() { return footerText(stops); },
    get percent() { return stops.find((row) => row.state.percent !== undefined)?.state.percent; },
    choose,
  };
}

export type OnboardingModels = ReturnType<typeof useOnboardingModels>;
