export type ToastOptions = { message: string; action?: { label: string; run: () => void }; durationMs?: number };
export type ToastItem = ToastOptions & { id: number; durationMs: number };

export const MAX_VISIBLE = 3;

export const toasts = $state<ToastItem[]>([]);

let nextId = 0;

export function dismiss(id: number) {
  const index = toasts.findIndex((item) => item.id === id);
  if (index >= 0) toasts.splice(index, 1);
}

export const toast = {
  show(options: ToastOptions): () => void {
    const id = nextId++;
    toasts.push({ ...options, id, durationMs: options.durationMs ?? 5000 });
    return () => dismiss(id);
  },
};
