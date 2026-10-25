// The figures of Clarity settings, System, and the status bar's meters:
// how sizes and shares are written, the minute the graphs show, and the
// io meter's scale. The readings come from the backend (system.rs).

/** The readings the graphs show: one a second, a minute. */
export const HISTORY = 60;

/** `history` with `value` added, keeping the last `HISTORY`. */
export function kept<T>(history: T[], value: T): T[] {
  const next = [...history, value];
  return next.length > HISTORY ? next.slice(next.length - HISTORY) : next;
}

const UNITS = ["KiB", "MiB", "GiB", "TiB"];

/** A size as people read it: `152 bytes`, `1.6 KiB`, `3.6 GiB`. */
export function bytes(n: number): string {
  if (n < 1024) return `${Math.round(n)} ${Math.round(n) === 1 ? "byte" : "bytes"}`;
  let value = n / 1024;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${value.toFixed(1)} ${UNITS[unit]}`;
}

/** A share: `4.7%`. */
export function percent(n: number): string {
  return `${n.toFixed(1)}%`;
}

/** Where the io meter is full: 100 MiB a second. */
const IO_FULL = 100 * 1024 * 1024;

/** How full the io meter is, from 0 to 1, for bytes a second: on a log
    scale, so a few KiB shows and 100 MiB fills it. */
export function ioLevel(perSecond: number): number {
  if (perSecond <= 0) return 0;
  return Math.min(1, Math.log1p(perSecond) / Math.log1p(IO_FULL));
}
