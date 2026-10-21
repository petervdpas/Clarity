// The themes of Clarity itself, an app setting: the palette of the IDE, not
// of the applications it designs (those have their own themes). The page
// takes the light or dark palette from `data-scheme` and a theme's own
// colors from `data-ide-theme` (app.css).

export type IdeTheme = "system" | "light" | "dark" | "cream-green";

export const IDE_THEMES: { value: IdeTheme; label: string; note: string }[] = [
  { value: "system", label: "System", note: "Light or dark, as the system is" },
  { value: "light", label: "Light", note: "White panels, blue accents" },
  { value: "dark", label: "Dark", note: "Dark panels, easy on the eyes at night" },
  { value: "cream-green", label: "Cream and green", note: "Cream pages ruled in green, with a dark green binding" },
];

/** Whether the page is light or dark under `theme`, the system being dark or not. */
export function schemeOf(theme: IdeTheme, systemDark: boolean): "light" | "dark" {
  if (theme === "system") return systemDark ? "dark" : "light";
  return theme === "dark" ? "dark" : "light";
}

export function isIdeTheme(value: string | null): value is IdeTheme {
  return IDE_THEMES.some((t) => t.value === value);
}
