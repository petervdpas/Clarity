// What is open in the main area: a window, the app, or a part of the data
// dictionary, an engine, a theme, a language, a resource or the module structure, or the project settings.

export type Selected = {
  kind: "window" | "app" | "entity" | "enum" | "engine" | "theme" | "language" | "resource" | "module" | "settings";
  name: string;
};
