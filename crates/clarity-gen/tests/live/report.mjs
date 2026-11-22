// The report tab's designer, which reads and writes its report through the backend
// (tests/web_windows.rs, the_report_designer_reads_and_writes_through_the_backend),
// driven in a headless Chromium as a person uses it: open the report, drag a part,
// undo and redo, set a look, add a part. After each step it reads the report's files.
// Usage: node report.mjs <the IDE's page> <the IDE's backend> <folder for shots> <the reports folder>
// Prints one line of JSON: what it saw, or the error.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { openBrowser } from "./browser.mjs";

const [page, backend, folder, reports] = process.argv.slice(2);
const reportFile = join(reports, "Tasks.report");
const sheetFile = join(reports, "tasks.css");
const start = readFileSync(reportFile, "utf8");

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

/** Waits up to `ms` for a file to read as `wanted` says; gives what it read last. */
async function fileSoon(path, wanted, what, ms = 5000) {
  const end = Date.now() + ms;
  let text = "";
  while (Date.now() < end) {
    text = readFileSync(path, "utf8");
    if (wanted(text)) return text;
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error(`${what}; the file reads:\n${text}`);
}

const b = await openBrowser("about:blank", { width: 1600, height: 1100, folder });
const result = {};
try {
  await b.send("Page.addScriptToEvaluateOnNewDocument", { source: tauri });
  await b.send("Page.navigate", { url: page });
  const clickAt = async (expression, what) => {
    const p = await b.eval(`(() => { const e = ${expression}; if (!e) return null; e.scrollIntoView({ block: "nearest" }); const r = e.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; })()`);
    if (!p) throw new Error(`nothing to click: ${what}`);
    await b.mouse("mouseMoved", p);
    await b.mouse("mousePressed", p, 1);
    await b.mouse("mouseReleased", p, 0);
    await b.settle(300);
  };

  // The Reports tab, then the report from its tree.
  await b.until(`[...document.querySelectorAll(".ribbon-tabs [role=tab]")].some((t) => t.textContent.trim() === "Reports")`, "the IDE's tabs", 60000);
  await clickAt(`[...document.querySelectorAll(".ribbon-tabs [role=tab]")].find((t) => t.textContent.trim() === "Reports")`, "the Reports tab");
  const row = 'button.row[title="reports/Tasks.report"]';
  await b.until(`document.querySelector(${JSON.stringify(row)})`, "the report in the project tree", 60000);
  await b.eval(`document.querySelector(${JSON.stringify(row)}).closest("details").open = true`);
  await b.settle(200);
  await b.click(row);
  await b.until(`document.querySelectorAll(".paper .part").length === 2`, "the report's two parts", 20000);
  await b.settle(1200);
  result.opened = { parts: await b.eval(`document.querySelectorAll(".paper .part").length`), unchanged: readFileSync(reportFile, "utf8") === start };
  await b.screenshot(join(folder, "opened.png"));

  // Drag the title 20 mm to the right.
  const title = '.part[data-key="0:0"]';
  const perMm = (await b.eval(`document.querySelector(${JSON.stringify(title)}).getBoundingClientRect().width`)) / 80;
  const from = await b.at(title, 0.5, 0.5);
  await b.drag(from, { x: from.x + 20 * perMm, y: from.y }, 10);
  result.moved = await fileSoon(reportFile, (t) => t.includes('<Text at="20,0"'), "the drag did not reach the report");

  // Undo and redo with the keys.
  await b.key("z", ["Control"]);
  result.undone = await fileSoon(reportFile, (t) => t === start, "Ctrl+Z did not take the drag back");
  await b.key("y", ["Control"]);
  result.redone = await fileSoon(reportFile, (t) => t.includes('<Text at="20,0"'), "Ctrl+Y did not do the drag again");

  // Pick the title and make its class bold in its look.
  await b.click(title);
  await clickAt(`[...document.querySelectorAll("[role=tab], button")].find((t) => t.textContent.trim() === "Look")`, "the Look tab of the properties");
  await clickAt(`[...document.querySelectorAll("label.check")].find((l) => l.textContent.trim() === "Bold")?.querySelector("input")`, "the Bold box of the picked part's look");
  const css = await fileSoon(sheetFile, (t) => t.includes("font-weight: bold"), "the look did not reach the stylesheet");
  await b.until(`document.querySelector(${JSON.stringify(title)}).style.fontWeight === "700"`, "the title drawn bold", 5000);
  result.styled = { css, drawn: await b.eval(`document.querySelector(${JSON.stringify(title)}).style.fontWeight`) };

  // Pick the band and click the Text tile: a new part.
  const band = '.paper .band[data-band="0"]';
  await b.click(band, 0.9, 0.85);
  await clickAt(`[...document.querySelectorAll("button.tile")].find((t) => t.title.startsWith("Drag it onto a band") && t.textContent.trim().endsWith("Text"))`, "the Text tile");
  result.added = await fileSoon(reportFile, (t) => t.split("<Text ").length === 4, "the new part did not reach the report");
  await b.screenshot(join(folder, "done.png"));
} catch (e) {
  result.error = String(e?.message ?? e);
  try {
    await b.screenshot(join(folder, "failed.png"));
  } catch {}
}
result.logs = b.logs?.slice(-20);
await b.close();
console.log(JSON.stringify(result));
