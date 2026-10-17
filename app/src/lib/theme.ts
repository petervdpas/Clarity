// Themes in the web renderer (design section 1.5, T1). The generator writes
// a CSS file per theme (`ui/themes/shop.css`), and the designer gets the same
// CSS from the backend; the renderer only holds the classes: the theme's on
// the application, a style's on a part, and says light or dark.

/** What the renderer needs of a theme (`ThemeDecl`). */
export type ThemeClasses = {
  mode: "light" | "dark" | "system" | null;
  class: string;
  styles: { name: { name: string }; class: string }[];
};

/** Whether a theme shows dark when the system is `systemDark`: a theme with
 *  mode light or dark keeps to it. */
export function isDark(mode: ThemeClasses["mode"], systemDark: boolean): boolean {
  return mode === "dark" ? true : mode === "light" ? false : systemDark;
}

/** The class of style `name` in `theme`, if the theme has it. */
export function styleClass(theme: ThemeClasses | null, name: string | null): string | undefined {
  if (!theme || !name) return undefined;
  return theme.styles.find((s) => s.name.name === name)?.class;
}
