// The IDE's preview and design canvas on Clarity.UI (DESIGN 8.1), driven in
// a headless Chromium (tests/web_windows.rs, the_ide_previews_and_designs_on_clarity_ui_in_a_browser).
// Usage: node ide.mjs <the IDE's page, run by Vite> <the IDE's backend> <folder for the profile and shots>
// The page runs without Tauri: its `invoke` posts each command to the backend
// the test answers. Prints one line of JSON: what it saw, or the error.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { openBrowser } from "./browser.mjs";

const [page, backend, folder] = process.argv.slice(2);

// Tauri's part of the page, as far as the IDE's front end asks it.
const tauri = `
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
  transformCallback: (callback) => { const id = Math.floor(Math.random() * 1e9); window["_" + id] = callback; return id; },
  unregisterCallback: () => {},
  convertFileSrc: (path) => path,
  invoke: async (cmd, args) => {
    const answer = await (await fetch(${JSON.stringify(backend)}, { method: "POST", body: JSON.stringify({ cmd, args: args ?? {} }) })).json();
    if (answer.error !== undefined) throw answer.error;
    return answer.value;
  },
};
window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
`;

const b = await openBrowser("about:blank", { width: 1600, height: 1100, folder });
const result = {};
try {
  await b.send("Page.addScriptToEvaluateOnNewDocument", { source: tauri });
  await b.send("Page.navigate", { url: page });

  // The preview: Main, drawn by Clarity.UI with sample data.
  const shown = ".shown canvas[data-drawn]";
  await b.until(`document.querySelector(${JSON.stringify(shown)})`, "the preview drawn", 60000);
  await b.settle(500);
  const drawn = (step) => b.eval(`document.querySelector(${JSON.stringify(shown)}).clarityUi.step(${JSON.stringify(step)})`);
  const read = await drawn({ do: "read" });
  result.preview = { window: read.window, rows: (read.data?.tasks ?? []).length };
  await b.screenshot(join(folder, "preview.png"));
  // A click on the drawn Add button, at its place on the page.
  const add = await b.eval(`(() => { const c = document.querySelector(${JSON.stringify(shown)}); const r = c.getBoundingClientRect(); const s = r.width / c.clientWidth; return { x: r.left, y: r.top, s }; })()`);
  const [x, y, w, h] = read.shown.addButton;
  const at = { x: add.x + (x + w / 2) * add.s, y: add.y + (y + h / 2) * add.s };
  await b.mouse("mouseMoved", at);
  await b.mouse("mousePressed", at, 1);
  await b.mouse("mouseReleased", at, 0);
  // What the window says after the click: the program's answer.
  let after = null;
  for (let i = 0; i < 50; i++) {
    after = await drawn({ do: "read" });
    if (after.said.length > 0 || after.notes.length > 0) break;
    await b.settle(100);
  }
  result.preview.said = [...after.said, ...after.notes];
  await b.screenshot(join(folder, "preview-clicked.png"));

  // The designer: the Design tab, the window Main.
  const clickWhere = async (expression, what) => {
    const p = await b.eval(`(() => { const e = ${expression}; if (!e) return null; const r = e.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; })()`);
    if (!p) throw new Error(`nothing to click: ${what}`);
    await b.mouse("mouseMoved", p);
    await b.mouse("mousePressed", p, 1);
    await b.mouse("mouseReleased", p, 0);
    await b.settle(200);
  };
  await clickWhere(`[...document.querySelectorAll(".ribbon-tabs [role=tab]")].find((t) => t.textContent.trim() === "Design")`, "the Design tab");
  await clickWhere(`document.querySelector('button.row[title^="Main in module"]')`, "the window Main");
  const canvas = ".client canvas[data-drawn]";
  await b.until(`document.querySelector(${JSON.stringify(canvas)}) && document.querySelectorAll("[data-design-layer] [data-widget]").length >= 6`, "the design canvas and its layer", 60000);
  await b.settle(500);
  result.design = { parts: await b.eval(`document.querySelectorAll("[data-design-layer] [data-widget]").length`) };
  await b.screenshot(join(folder, "design.png"));
  // A click on the drawn Delete button picks it.
  await b.click(`[data-design-layer] [data-part="deleteButton"]`);
  result.design.picked = await b.eval(`!!document.querySelector('[data-design-layer] .part.selected[data-part="deleteButton"]')`);
  await b.screenshot(join(folder, "design-picked.png"));
  // A drag four columns to the left moves it in the window's file.
  const from = await b.at(`[data-design-layer] [data-part="deleteButton"]`);
  const pitch = await b.eval(`(() => { const g = document.querySelector("[data-design-layer] [data-grid]"); const r = g.getBoundingClientRect(); const s = r.width / g.offsetWidth; return (r.width + 8 * s) / Number(g.dataset.columns); })()`);
  await b.drag(from, { x: from.x - 4 * pitch, y: from.y }, 12);
  const file = join(folder, "project", "todo", "todo.clarity");
  const end = Date.now() + 15000;
  while (!/deleteButton at 7,11/.test(readFileSync(file, "utf8")) && Date.now() < end) await b.settle(200);
  result.design.moved = /deleteButton at 7,11/.test(readFileSync(file, "utf8"));
  // The drawing follows: the box of the button moves with the button Clarity.UI draws again.
  await b.until(`(() => { const e = document.querySelector('[data-design-layer] [data-part="deleteButton"]'); return e && e.getBoundingClientRect().left + 2 < ${from.x} - ${pitch} ; })()`, "the layer following the drawing", 15000).catch((why) => (result.design.follows = String(why)));
  result.design.follows ??= true;
  await b.settle(300);
  await b.screenshot(join(folder, "design-moved.png"));

  // A table's column, sized by its header's edge on the canvas.
  const tableLine = () => readFileSync(file, "utf8").split("\n").find((l) => l.includes("table name grid")) ?? "";
  const before = tableLine();
  const edge = await b.at(`[data-design-layer] [data-column="title"]`);
  await b.drag(edge, { x: edge.x + 60, y: edge.y }, 6);
  const sized = Date.now() + 15000;
  while (tableLine() === before && Date.now() < sized) await b.settle(200);
  result.design.column = tableLine() !== before ? tableLine().trim() : null;
  await b.screenshot(join(folder, "design-column.png"));

  // The window Users has its own ribbon: its buttons are picked on the canvas.
  await clickWhere(`[...document.querySelectorAll(".side-tabs [role=tab]")].find((t) => t.textContent.trim() === "Elements")`, "the Elements side tab");
  await clickWhere(`document.querySelector('button.row[title^="Users in module"]')`, "the window Users");
  await b.until(`document.querySelector('[data-design-layer] [data-command][data-part="mainButton"]')`, "the ribbon's buttons on the layer", 30000);
  await b.click(`[data-design-layer] [data-command][data-part="mainButton"]`);
  result.design.command = await b.eval(`!!document.querySelector('[data-design-layer] .part.command.selected[data-part="mainButton"]')`);
  await b.screenshot(join(folder, "design-command.png"));
} catch (why) {
  result.error = String(why && why.stack ? why.stack : why);
}
result.logs = b.logs.slice(0, 40);
await b.close();
console.log(JSON.stringify(result));
