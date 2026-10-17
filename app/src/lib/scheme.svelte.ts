// Light or dark: the system's choice, kept as state so a theme that follows
// the system changes with it. The page takes its palette from it through
// `data-scheme` (app.css); a themed area can set its own.

export const scheme = $state({ dark: false });

/** Follows the system's light or dark mode for the page, from now on. */
export function followSystemScheme() {
  const media = window.matchMedia("(prefers-color-scheme: dark)");
  const apply = () => {
    scheme.dark = media.matches;
    document.documentElement.dataset.scheme = media.matches ? "dark" : "light";
  };
  apply();
  media.addEventListener("change", apply);
}
