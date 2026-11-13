// What is open in the main area: a window, the app, or a part of the data
// dictionary, an engine, a custom part, a theme, a language, a resource, a diagram (a view written from the project, or a
// written one of the diagrams folder) or the module structure, the project settings, the project's README, a
// mapping (how a file is laid over an entity), the project's sign-in setup (the project file's signin block), or the
// app's settings block (`settingsBlock`: what each person sets, as against `settings`, the project file's settings),
// or a toast the project declares.

export type Selected = {
  kind: "window" | "app" | "entity" | "enum" | "engine" | "part" | "library" | "toast" | "command" | "function" | "theme" | "language" | "resource" | "diagram" | "module" | "settings" | "api" | "readme" | "mapping" | "signin" | "agents" | "settingsBlock";
  name: string;
};
