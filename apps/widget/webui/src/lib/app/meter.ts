/** The quietest and loudest levels a meter shows, in dBFS: room tone sits below -55, close speech near -15. */
const FLOOR_DB = -55;
const CEILING_DB = -15;

/** How many of `bars` an input's RMS level lights, on the decibel scale macOS meters use. */
export function litBars(level: number, bars: number): number {
  if (!(level > 0)) return 0;
  const db = 20 * Math.log10(level);
  const share = Math.min(1, Math.max(0, (db - FLOOR_DB) / (CEILING_DB - FLOOR_DB)));
  return Math.round(share * bars);
}
