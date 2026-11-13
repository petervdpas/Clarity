// What is open in the main area: a window, the app, or a part of the data
// dictionary, an engine, a custom part, a theme, a language, a resource, a diagram (a view written from the project, or a
// written one of the diagrams folder) or the module structure, the project settings, the project's README, a
// mapping (how a file is laid over an entity), or the project's sign-in setup (the project file's signin block).

export type Selected = {
  kind: "window" | "app" | "entity" | "enum" | "engine" | "part" | "library" | "theme" | "language" | "resource" | "diagram" | "module" | "settings" | "api" | "readme" | "mapping" | "signin" | "agents";
  name: string;
};
