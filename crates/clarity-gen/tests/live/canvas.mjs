// The IDE's canvas started as the IDE starts it (tests/live_canvas.rs): each
// page given opens in the live tests' browser (browser.mjs: headless, cut off
// from the desktop), in real time; what the page wrote and what the browser
// said (errors, warnings, exceptions) come back, a line per page.
//
//   node canvas.mjs <folder> <pages.json>   pages.json: [{ "name", "url" }]

import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { openBrowser } from "./browser.mjs";

const [folder, list] = process.argv.slice(2);
const pages = JSON.parse(readFileSync(list, "utf8"));
for (const { name, url } of pages) {
  const b = await openBrowser(url, { width: 1200, height: 900, folder });
  let said = "";
  try {
    // A swap page says SWAP: the server serves the other browser files once the mark is made.
    if (url.includes("swap")) {
      await b.until(`/SWAP|FAILED|ERROR|REJECTED/.test(document.getElementById("out")?.textContent ?? "")`, "the first start", 45000);
      writeFileSync(join(folder, "swapped"), "");
    }
    await b.until(`/READ|FAILED|ERROR|REJECTED/.test(document.getElementById("out")?.textContent ?? "")`, "a start", 45000);
    await b.settle(1500);
  } catch (e) {
    said = `TIMEOUT ${e.message} | `;
  }
  said += (await b.eval(`document.getElementById("out")?.textContent ?? ""`).catch(() => "")).trim().replace(/\n/g, " | ");
  const logs = b.logs.map((l) => l.split("\n")[0]).join(" | ");
  console.log(`${name}: ${said}${logs ? " || " + logs : ""}`);
  await b.close();
}
