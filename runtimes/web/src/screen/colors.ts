// The color field type of U1c apart from the screen
// (conformance/colors.json): a color is kept as text "#rrggbb", read from
// what is typed the same way on every renderer, and picked from a palette
// they share (clarity_spec::colors::PALETTE).

/** The colors the color input offers, in rows: grays, then light to dark shades of ten hues. */
export const PALETTE: readonly (readonly string[])[] = [
  ["#ffffff", "#f8fafc", "#e5e7eb", "#d1d5db", "#9ca3af", "#6b7280", "#4b5563", "#374151", "#1f2937", "#000000"],
  ["#e2e8f0", "#fecaca", "#fed7aa", "#fde68a", "#bbf7d0", "#99f6e4", "#bfdbfe", "#c7d2fe", "#e9d5ff", "#fbcfe8"],
  ["#94a3b8", "#f87171", "#fb923c", "#fbbf24", "#4ade80", "#2dd4bf", "#60a5fa", "#818cf8", "#c084fc", "#f472b6"],
  ["#64748b", "#ef4444", "#f97316", "#f59e0b", "#22c55e", "#14b8a6", "#3b82f6", "#6366f1", "#a855f7", "#ec4899"],
  ["#475569", "#dc2626", "#ea580c", "#d97706", "#16a34a", "#0d9488", "#2563eb", "#4f46e5", "#9333ea", "#db2777"],
  ["#1e293b", "#991b1b", "#9a3412", "#92400e", "#166534", "#115e59", "#1e40af", "#3730a3", "#6b21a8", "#9d174d"],
];

/** The color `typed` is, as "#" and six lowercase digits; "" for no color; null when it is no color at all. */
export function readColor(typed: string): string | null {
  const text = typed.trim();
  if (text === "") return "";
  const digits = text.startsWith("#") ? text.slice(1) : text;
  if (!/^[0-9a-f]*$/i.test(digits)) return null;
  if (digits.length === 3) return `#${[...digits].map((c) => c + c).join("")}`.toLowerCase();
  return digits.length === 6 ? `#${digits.toLowerCase()}` : null;
}
