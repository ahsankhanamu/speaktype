export type AlertOptions = { title: string; message?: string; confirmLabel: string; destructive?: boolean };

export const alertState = $state<{ open: boolean; options: AlertOptions }>({ open: false, options: { title: '', confirmLabel: '' } });

let resolver: ((confirmed: boolean) => void) | null = null;

export function confirmAlert(options: AlertOptions): Promise<boolean> {
  resolver?.(false);
  alertState.options = options;
  alertState.open = true;
  return new Promise((resolve) => (resolver = resolve));
}

export function settleAlert(confirmed: boolean) {
  alertState.open = false;
  resolver?.(confirmed);
  resolver = null;
}
