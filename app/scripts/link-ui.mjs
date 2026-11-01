// The screen package (runtimes/web) uses the IDE's packages: this links its
// node_modules to app/node_modules, so Svelte and the Tauri API resolve from
// the package's own files under Node, Vite and TypeScript alike, one copy
// each. A junction on Windows, which needs no administrator rights.
import { existsSync, lstatSync, symlinkSync, unlinkSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const app = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const link = join(app, "../runtimes/web/node_modules");
const target = join(app, "node_modules");
if (existsSync(link) || lstatSync(link, { throwIfNoEntry: false })?.isSymbolicLink()) {
  if (!lstatSync(link).isSymbolicLink()) process.exit(0);
  unlinkSync(link);
}
symlinkSync(target, link, process.platform === "win32" ? "junction" : "dir");
