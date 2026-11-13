// What you see is what you get (tests/web_windows.rs, the_design_canvas_lays_out_a_window_as_the_preview_does):
// the IDE's design canvas and its preview draw the same window with the same
// library, so every named part has the same place on both.
// Usage: node wysiwyg.mjs <the IDE's page, run by Vite> <the IDE's backend> <folder for the profile and shots> <window>
// Prints one line of JSON: the places on each canvas, or the error.

import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { openBrowser } from "./browser.mjs";

const [page, backend, folder, window] = process.argv.slice(2);

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
  const read = (selector) => b.eval(`document.querySelector(${JSON.stringify(selector)}).clarityUi.step({ do: "read" })`);

  // The preview: the window as the program draws it.
  const shown = ".shown canvas[data-drawn]";
  await b.until(`document.querySelector(${JSON.stringify(shown)})`, "the preview drawn", 60000);
  await b.settle(800);
  const preview = await read(shown);
  result.preview = { window: preview.window, size: preview.size ?? null, shown: preview.shown };
  await b.screenshot(join(folder, "wysiwyg-preview.png"));

  // The designer: the same window on the design canvas.
  const clickWhere = async (expression, what) => {
    const p = await b.eval(`(() => { const e = ${expression}; if (!e) return null; const r = e.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; })()`);
    if (!p) throw new Error(`nothing to click: ${what}`);
    await b.mouse("mouseMoved", p);
    await b.mouse("mousePressed", p, 1);
    await b.mouse("mouseReleased", p, 0);
    await b.settle(200);
  };
  await clickWhere(`[...document.querySelectorAll(".ribbon-tabs [role=tab]")].find((t) => t.textContent.trim() === "Design")`, "the Design tab");
  await clickWhere(`document.querySelector('button.row[title^=${JSON.stringify(window + " in module")}]')`, `the window ${window}`);
  const canvas = ".client canvas[data-drawn]";
  await b.until(`document.querySelector(${JSON.stringify(canvas)})`, "the design canvas", 60000);
  await b.settle(1500);
  const design = await read(canvas);
  result.design = { window: design.window, size: design.size ?? null, shown: design.shown };
  result.sizes = await b.eval(`[${JSON.stringify(shown)}, ${JSON.stringify(canvas)}].map((s) => { const c = document.querySelector(s); return c ? { client: [c.clientWidth, c.clientHeight], pixels: [c.width, c.height] } : null; })`);
  await b.screenshot(join(folder, "wysiwyg-design.png"));
  // What each canvas was given, for a difference to be found.
  if (process.env.WYSIWYG_FILES) {
    const files = await b.eval(`(async () => { const base = await window.__TAURI_INTERNALS__.invoke("preview_server_address"); const out = []; for (const d of [0, 1]) { const url = base + "files.json?window=${window}&design=" + d + "&revision=9"; out.push({ url, text: await (await fetch(url)).text() }); } return out; })()`);
    files.forEach((f, i) => writeFileSync(join(folder, `files-${i}.json`), f.url + "\n" + f.text));
  }
} catch (why) {
  result.error = String(why && why.stack ? why.stack : why);
}
result.logs = b.logs.slice(0, 40);
await b.close();
console.log(JSON.stringify(result));
