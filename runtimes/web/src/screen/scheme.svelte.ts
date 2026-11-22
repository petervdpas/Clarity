// Light or dark: the system's choice, kept as state so a theme that follows
// the system changes with it. The page takes its palette from it through
// `data-scheme` (app.css); a themed area can set its own.

export const scheme = $state({ dark: false });

/** The theme a person chose among those the application offers; undefined
 *  shows the one it starts with. The designer sets it to show its canvas
 *  in another theme. */
export const themeChoice = $state<{ theme: string | undefined }>({ theme: undefined });
