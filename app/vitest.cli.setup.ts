// Builds the command line Clarity once, before any test runs: the designer
// tests read every example through it (src/lib/ui/everyWindow.svelte.test.ts),
// and a test that compiled it would spend its own time limit on the build.
import { execFileSync } from "node:child_process";
import { join } from "node:path";

export default function setup() {
  execFileSync("cargo", ["build", "-q", "-p", "clarity-cli"], {
    cwd: join(import.meta.dirname, ".."),
    stdio: "inherit",
    env: { ...process.env, CARGO_BUILD_JOBS: process.env.CARGO_BUILD_JOBS ?? "2" },
  });
}
