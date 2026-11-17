// Serves Clarity's tools to an AI agent (`clarity mcp`) from this checkout's
// own build, so the agent works with the Clarity of this repository, not an
// installed one. It builds the command first when it is behind its source
// (cargo does nothing when it is not), then runs it. .mcp.json names it for
// Claude Code as clarity-cdev, beside the installed Clarity's `clarity` (user
// scope); any agent that starts a command over stdio can use it. Node runs it
// on Linux, Windows and macOS alike (Windows cannot start mcp.sh itself).
//
//   node mcp.mjs        the tools, on the project open in the Clarity IDE
//
// Only MCP goes to standard output; the build says what it does on stderr.
import { spawn, spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url));
const env = { ...process.env, CARGO_BUILD_JOBS: process.env.CARGO_BUILD_JOBS || "2" };
const build = spawnSync("cargo", ["build", "--quiet", "-p", "clarity-cli"], { cwd: root, env, stdio: ["ignore", 2, 2] });
if (build.error) {
  console.error(`mcp.mjs: cannot start cargo: ${build.error.message}`);
  process.exit(1);
}
if (build.status !== 0) process.exit(build.status ?? 1);

const clarity = join(root, "target", "debug", process.platform === "win32" ? "clarity.exe" : "clarity");
const served = spawn(clarity, ["mcp", ...process.argv.slice(2)], { cwd: root, stdio: "inherit" });
for (const signal of ["SIGINT", "SIGTERM"]) process.on(signal, () => served.kill(signal));
served.on("error", (err) => {
  console.error(`mcp.mjs: cannot start ${clarity}: ${err.message}`);
  process.exit(1);
});
served.on("exit", (code) => process.exit(code ?? 1));
