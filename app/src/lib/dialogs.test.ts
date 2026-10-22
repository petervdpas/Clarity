import { test } from "node:test";
import assert from "node:assert/strict";
import { Dialogs } from "./dialogs.ts";

test("confirm waits for yes or no", async () => {
  const shown: (string | null)[] = [];
  const dialogs = new Dialogs((d) => shown.push(d?.kind ?? null));
  const answer = dialogs.confirm({ text: "Delete Album?", action: "Delete", danger: true });
  assert.deepEqual(dialogs.current, { kind: "confirm", text: "Delete Album?", action: "Delete", danger: true });
  dialogs.answer(true);
  assert.equal(await answer, true);
  const no = dialogs.confirm({ text: "Write it again?", action: "Write again" });
  dialogs.answer(false);
  assert.equal(await no, false);
  assert.deepEqual(shown, ["confirm", null, "confirm", null]);
});

test("a message is closed with OK", async () => {
  const dialogs = new Dialogs(() => {});
  const closed = dialogs.message({ text: "The build failed.", title: "Build" });
  assert.equal(dialogs.current?.kind, "message");
  dialogs.answer(true);
  assert.equal(await closed, undefined);
});

test("prompt gives the text typed, or null when cancelled", async () => {
  const dialogs = new Dialogs(() => {});
  const named = dialogs.prompt({ text: "Name of the copy", value: "orders2", action: "Copy" });
  assert.equal(dialogs.current?.kind === "prompt" && dialogs.current.value, "orders2");
  dialogs.answer("sales");
  assert.equal(await named, "sales");
  const cancelled = dialogs.prompt({ text: "Name", action: "OK" });
  dialogs.answer(null);
  assert.equal(await cancelled, null);
});

test("choose gives the value of the button pressed, or null when cancelled", async () => {
  const dialogs = new Dialogs(() => {});
  const choices = [{ value: "all", label: "All entities" }, { value: "drawn", label: "Only the boxes drawn" }];
  const picked = dialogs.choose({ text: "Write it again from the project with", choices });
  dialogs.answer("drawn");
  assert.equal(await picked, "drawn");
  const none = dialogs.choose({ text: "Which?", choices });
  dialogs.answer(null);
  assert.equal(await none, null);
});

test("a dialog asked while another is open waits its turn", async () => {
  const dialogs = new Dialogs(() => {});
  const first = dialogs.confirm({ text: "One?", action: "Yes" });
  const second = dialogs.message({ text: "Two." });
  assert.equal(dialogs.current?.text, "One?");
  dialogs.answer(false);
  assert.equal(await first, false);
  assert.equal(dialogs.current?.text, "Two.");
  dialogs.answer(true);
  await second;
  assert.equal(dialogs.current, null);
});

test("Clarity shows no dialog of the system or the browser: only the file pickers come from the dialog plugin", async () => {
  const { readdirSync, readFileSync, statSync } = await import("node:fs");
  const { join } = await import("node:path");
  const root = new URL("..", import.meta.url).pathname;
  const files: string[] = [];
  const walk = (dir: string) => {
    for (const name of readdirSync(dir)) {
      const path = join(dir, name);
      if (statSync(path).isDirectory()) {
        if (name !== "generated") walk(path);
      } else if (/\.(svelte|ts)$/.test(name) && !name.endsWith(".test.ts")) files.push(path);
    }
  };
  walk(root);
  const found: string[] = [];
  for (const file of files) {
    const text = readFileSync(file, "utf8");
    for (const m of text.matchAll(/import\s*\{([^}]*)\}\s*from\s*"@tauri-apps\/plugin-dialog"/g)) {
      const names = m[1].split(",").map((n) => n.trim().split(/\s+as\s+/)[0]).filter(Boolean);
      for (const name of names) if (name !== "open" && name !== "save") found.push(`${file}: ${name} from the dialog plugin`);
    }
    // The browser's own: window.alert, window.confirm, window.prompt, or alert alone (Clarity has functions named confirm and prompt).
    for (const m of text.matchAll(/(?:window\.(alert|confirm|prompt)|(?<![\w.])(alert))\s*\(/g)) found.push(`${file}: ${m[1] ?? m[2]}()`);
  }
  assert.deepEqual(found, []);
});

test("every dialog of the IDE is framed by Modal.svelte: no other draws a backdrop of its own", async () => {
  const { readdirSync, readFileSync } = await import("node:fs");
  const dir = new URL(".", import.meta.url).pathname;
  const ownFrames = readdirSync(dir)
    .filter((name) => name.endsWith(".svelte") && name !== "Modal.svelte")
    .filter((name) => readFileSync(dir + name, "utf8").includes('class="backdrop"'));
  assert.deepEqual(ownFrames, []);
});
