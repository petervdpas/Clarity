// What is open in the main area: a window, the app, or a part of the data
// dictionary or the module structure, or the project settings.

export type Selected = {
  kind: "window" | "app" | "entity" | "enum" | "module" | "settings";
  name: string;
};
