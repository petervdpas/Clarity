// Showing and reading values in the locale of the person using the
// application (design section 1.5, I1a). Pure functions on `Intl`, so the
// conformance cases in conformance/formats.json can check them outside a
// browser. The names a field's `format` can take come from the backend's
// catalog; this file turns each into Intl options.

/** Where values are shown: the locale and the project's currency. */
export type Formats = {
  /** A language tag such as `nl-NL`. */
  locale: string;
  /** An ISO 4217 code such as `EUR`, or null when the project names none. */
  currency: string | null;
};

/** The types this file formats, by the kind the designer sees. */
export type FormatKind = "integer" | "decimal" | "money" | "date" | "dateTime" | "duration";

/** How a value of a field is shown: its kind, its scale (decimals), and
 *  the named format the field picks, if any. */
export type FieldFormat = {
  kind: FormatKind;
  scale?: number;
  format?: string | null;
};

/** The locale to show values in: the system's, else the project's
 *  language, else English. */
export function userLocale(system: readonly string[] | undefined, projectLanguage: string | null): string {
  const supported = (tag: string | null | undefined) => !!tag && Intl.NumberFormat.supportedLocalesOf(tag).length > 0;
  const found = system?.find(supported);
  return found ?? (supported(projectLanguage) ? (projectLanguage as string) : "en");
}

/** The decimals of the project's currency: 2 for EUR, 0 for JPY. */
function currencyDigits(f: Formats): number {
  if (!f.currency) return 2;
  return new Intl.NumberFormat(f.locale, { style: "currency", currency: f.currency }).resolvedOptions().maximumFractionDigits ?? 2;
}

function numberOptions(field: FieldFormat, f: Formats): Intl.NumberFormatOptions {
  const plain = field.format === "plain";
  switch (field.kind) {
    case "integer":
      return { maximumFractionDigits: 0, useGrouping: !plain };
    case "decimal": {
      // Without a field to take a scale from (a local in code), a value
      // keeps its own digits.
      const scale = field.scale ?? null;
      if (field.format === "percent") {
        if (scale === null) return { style: "percent", maximumFractionDigits: 18 };
        const digits = Math.max(scale - 2, 0);
        return { style: "percent", minimumFractionDigits: digits, maximumFractionDigits: digits };
      }
      if (scale === null) return { maximumFractionDigits: 20, useGrouping: !plain };
      return { minimumFractionDigits: scale, maximumFractionDigits: scale, useGrouping: !plain };
    }
    default: {
      const digits = currencyDigits(f);
      if (f.currency && !plain) return { style: "currency", currency: f.currency };
      return { minimumFractionDigits: digits, maximumFractionDigits: digits };
    }
  }
}

/** A stored date (`2026-10-17`) or date and time (`2026-10-17T09:30:00`)
 *  as a moment in UTC, so no time zone moves it. */
function moment(value: string): Date | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2})(?::(\d{2}))?)?/.exec(value);
  if (!m) return null;
  const [, y, mo, d, h, mi, s] = m;
  return new Date(Date.UTC(Number(y), Number(mo) - 1, Number(d), Number(h ?? 0), Number(mi ?? 0), Number(s ?? 0)));
}

function datePart(date: Date, format: string | null | undefined, locale: string): string {
  if (format === "iso") return date.toISOString().slice(0, 10);
  const options: Intl.DateTimeFormatOptions =
    format === "long"
      ? { year: "numeric", month: "long", day: "numeric", timeZone: "UTC" }
      : { year: "numeric", month: "2-digit", day: "2-digit", timeZone: "UTC" };
  return new Intl.DateTimeFormat(locale, options).format(date);
}

function timePart(date: Date, format: string | null | undefined, locale: string): string {
  if (format === "iso") return date.toISOString().slice(11, 16);
  // A 24-hour clock writes 09:30; a 12-hour one 9:30 AM.
  const cycle = new Intl.DateTimeFormat(locale, { hour: "numeric" }).resolvedOptions().hourCycle;
  const hour = cycle === "h11" || cycle === "h12" ? "numeric" : "2-digit";
  return new Intl.DateTimeFormat(locale, { hour, minute: "2-digit", timeZone: "UTC" }).format(date);
}

/** A duration of whole seconds as `m:ss`, or `h:mm:ss` from an hour, in
 *  every locale. */
function formatDuration(seconds: number): string {
  const sign = seconds < 0 ? "-" : "";
  const all = Math.abs(Math.trunc(seconds));
  const h = Math.floor(all / 3600);
  const m = Math.floor((all % 3600) / 60);
  const s = all % 60;
  const two = (n: number) => String(n).padStart(2, "0");
  return h > 0 ? `${sign}${h}:${two(m)}:${two(s)}` : `${sign}${m}:${two(s)}`;
}

/** Typed text as a duration in seconds: `3:45`, `1:02:03`, or seconds
 *  alone; null when it is no duration. */
function parseDuration(text: string): number | null {
  const m = /^(?:(?:(\d+):)?(\d+):)?(\d+)$/.exec(text.trim());
  if (!m) return null;
  const [, h, min, sec] = m;
  if (min !== undefined && (sec.length !== 2 || Number(sec) > 59)) return null;
  if (h !== undefined && (min.length !== 2 || Number(min) > 59)) return null;
  return Number(h ?? 0) * 3600 + Number(min ?? 0) * 60 + Number(sec);
}

/** A stored value as people read it. Decimals and money arrive as text
 *  (`"1250.50"`), so they are formatted exactly, without rounding through
 *  a floating point number. */
export function formatValue(field: FieldFormat, value: string | number, f: Formats): string {
  if (field.kind === "duration") {
    const seconds = Number(value);
    return Number.isFinite(seconds) ? formatDuration(seconds) : String(value);
  }
  if (field.kind === "date" || field.kind === "dateTime") {
    const date = moment(String(value));
    if (!date) return String(value);
    const day = datePart(date, field.format, f.locale);
    return field.kind === "date" ? day : `${day} ${timePart(date, field.format, f.locale)}`;
  }
  // A value that is no number (a database written by another program)
  // shows as it is.
  if (!/^-?\d+(\.\d+)?$/.test(String(value).trim())) return String(value);
  const options = numberOptions(field, f);
  // Intl takes a decimal string as it is; the type says number only.
  return new Intl.NumberFormat(f.locale, options).format(value as unknown as number);
}

/** What a number input shows: as the value is shown, but money without
 *  its currency sign, so what is typed and what is shown look alike. */
export function editText(field: FieldFormat, value: string | number, f: Formats): string {
  if (field.kind === "money") return formatValue({ ...field, format: "plain" }, value, f);
  return formatValue(field, value, f);
}

/** The separators of a locale's numbers. */
function separators(locale: string): { group: string; decimal: string } {
  const parts = new Intl.NumberFormat(locale).formatToParts(12345.6);
  return {
    group: parts.find((p) => p.type === "group")?.value ?? ",",
    decimal: parts.find((p) => p.type === "decimal")?.value ?? ".",
  };
}

/** Moves the decimal point of a decimal string: `shift("25", -2)` is `"0.25"`. */
function shift(text: string, places: number): string {
  const negative = text.startsWith("-");
  const [whole, fraction = ""] = (negative ? text.slice(1) : text).split(".");
  let digits = whole + fraction;
  let point = whole.length + places;
  if (point <= 0) {
    digits = "0".repeat(1 - point) + digits;
    point = 1;
  }
  if (point > digits.length) digits = digits + "0".repeat(point - digits.length);
  const int = digits.slice(0, point).replace(/^0+(?=\d)/, "");
  const frac = digits.slice(point).replace(/0+$/, "");
  return (negative ? "-" : "") + int + (frac ? "." + frac : "");
}

/** What was typed in a number input, read in the locale: the value to
 *  store (a number for integers, a decimal string otherwise), or null when
 *  it is not a number of that kind. Thousands separators, spaces, the
 *  currency sign and the percent sign may be typed or left out. */
export function parseNumber(field: FieldFormat, text: string, f: Formats): string | number | null {
  if (field.kind === "duration") return parseDuration(text);
  const { group, decimal } = separators(f.locale);
  let clean = text.replace(/[\s  %]/g, "").replace(/−/g, "-");
  if (f.currency) {
    for (const part of new Intl.NumberFormat(f.locale, { style: "currency", currency: f.currency }).formatToParts(1)) {
      if (part.type === "currency") clean = clean.split(part.value).join("");
    }
    clean = clean.split(f.currency).join("");
  }
  if (group.trim() !== "") clean = clean.split(group).join("");
  if (decimal !== ".") clean = clean.split(decimal).join(".");
  if (!/^-?\d+(\.\d+)?$/.test(clean)) return null;
  if (field.kind === "integer") return clean.includes(".") ? null : Number(clean);
  return field.format === "percent" ? shift(clean, -2) : shift(clean, 0);
}

/** The value of a slot, as the backend sends it (runtime `ui::Slot`). */
export type Slot = {
  kind: "text" | "integer" | "decimal" | "money" | "date" | "dateTime" | "duration" | "bool" | "enum";
  value: string | number | boolean | null;
  scale?: number;
  format?: string | null;
  /** The enum of an enum value. */
  enum?: string;
};

/** Text the backend sends to show: plain, or with values in slots
 *  (design section 1.5, I1b). */
export type ShownText = string | { text: string; values: Record<string, Slot> };

function slotText(
  slot: Slot | undefined,
  f: Formats,
  label: (enumName: string, value: string) => string,
  word: (text: string) => string,
): string {
  if (!slot || slot.value === null) return "";
  switch (slot.kind) {
    case "text":
      return String(slot.value);
    case "bool":
      return word(slot.value ? "Yes" : "No");
    case "enum":
      return label(slot.enum ?? "", String(slot.value));
    default:
      return formatValue({ kind: slot.kind, scale: slot.scale, format: slot.format }, slot.value as string | number, f);
  }
}

/** Text as shown: each slot filled with its value in the locale, `{{` and
 *  `}}` as braces. Plain text arrives ready and is shown as it is.
 *  `label` gives the label of an enum value, `word` the renderer's own
 *  words (Yes, No) in the language shown. */
export function fillText(
  text: ShownText,
  f: Formats,
  label: (enumName: string, value: string) => string,
  word: (text: string) => string = (t) => t,
): string {
  if (typeof text === "string") return text;
  const t = text.text;
  let out = "";
  let i = 0;
  while (i < t.length) {
    if (t.startsWith("{{", i) || t.startsWith("}}", i)) {
      out += t[i];
      i += 2;
    } else if (t[i] === "{" && t.indexOf("}", i) > i) {
      const end = t.indexOf("}", i);
      out += slotText(text.values[t.slice(i + 1, end)], f, label, word);
      i = end + 1;
    } else {
      out += t[i];
      i += 1;
    }
  }
  return out;
}
