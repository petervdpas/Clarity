// What every build of the IDE and of the web screen shares: the screen's
// package, @clarity/ui (runtimes/web), is linked in from outside app/, so
// Vite may read it there, and the packages it uses are this folder's own,
// one copy each (two copies of Svelte would not share their state).
import { searchForWorkspaceRoot, type UserConfig } from "vite";

export const shared: UserConfig = {
  resolve: { dedupe: ["svelte", "@tauri-apps/api", "vitest"] },
  server: { fs: { allow: [searchForWorkspaceRoot(process.cwd()), "../runtimes/web"] } },
};
