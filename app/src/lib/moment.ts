// A moment (a datetime field) as a day and a 24-hour time typed as hh:mm,
// the same on the web as on Avalonia. The backends read and write moments
// as 2026-10-20T09:30:00.

const TIME = /^(\d{1,2}):(\d{2})$/;
const MOMENT = /^(\d{4}-\d{2}-\d{2})[T ](\d{2}):(\d{2})/;

/** The day (yyyy-mm-dd) and the time (hh:mm) of a moment; empty for none. */
export function splitMoment(value: unknown): { day: string; time: string } {
  const found = typeof value === "string" ? MOMENT.exec(value) : null;
  return found ? { day: found[1], time: `${found[2]}:${found[3]}` } : { day: "", time: "" };
}

/** A typed time as hh:mm; "" when nothing is typed, null when it is no time. */
export function readTime(text: string): string | null {
  const typed = text.trim();
  if (typed === "") return "";
  const found = TIME.exec(typed);
  if (!found) return null;
  const hours = Number(found[1]);
  const minutes = Number(found[2]);
  if (hours > 23 || minutes > 59) return null;
  return `${String(hours).padStart(2, "0")}:${found[2]}`;
}

/** The moment of a day and a time; no day is no moment, no time is midnight. */
export function joinMoment(day: string, time: string): string | null {
  if (day === "") return null;
  return `${day}T${time === "" ? "00:00" : time}:00`;
}
