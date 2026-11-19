<script lang="ts">
  import { liveQuery } from './liveQuery.svelte';

  interface QueryHandle {
    readonly data: unknown;
    readonly error: unknown;
    readonly loading: boolean;
    refetch(): Promise<void>;
  }

  let {
    queryKey,
    fetcher,
    invalidateOn = [],
    pollMs,
    refetchOnFocus = true,
    ready,
  }: {
    queryKey: string;
    fetcher: () => Promise<unknown>;
    invalidateOn?: string[];
    pollMs?: number;
    refetchOnFocus?: boolean;
    ready: (query: QueryHandle) => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  ready(liveQuery(queryKey, fetcher, { invalidateOn, pollMs, refetchOnFocus }));
</script>
