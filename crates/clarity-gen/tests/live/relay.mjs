// Carries the live steps (tests/clarity_live) to a web program in a real
// browser: each line on standard input is one step, as the desktop program
// takes them with CLARITY_HEADLESS, given to the page's own step channel
// (window.clarityUi.step, Clarity.UI's steps), and its answer goes back as
// one line on standard output. A shot is the browser's screenshot, and a
// file is the browser's own file chooser answered with it. What
// the page writes on its console shows on standard error. The steps are the
// same on the desktop and in the browser, so the two runs cannot drift apart.
// Arguments: the address the program is served at, the folder it runs in.

import { createInterface } from "node:readline";
import { resolve } from "node:path";
import { openBrowser } from "./browser.mjs";

const [url, given] = process.argv.slice(2);
const b = await openBrowser(url, { folder: resolve(given) });
let said = 0;
const console = () => {
  for (; said < b.logs.length; said++) process.stderr.write(`(browser) ${typeof b.logs[said] === "string" ? b.logs[said] : JSON.stringify(b.logs[said])}\n`);
};

let ready = null;
try {
  await b.until("window.clarityUi", "the page's start", 30000);
  await b.eval("window.clarityUi.started.then(() => true)");
} catch (e) {
  ready = `the page did not start: ${e.message}`;
}
console();

for await (const line of createInterface({ input: process.stdin })) {
  let answer;
  try {
    const step = JSON.parse(line);
    if (ready) answer = { ok: false, error: ready };
    else if (step.do === "file") {
      // The browser's own file chooser answers the next file question, as a person picks the file there.
      b.answerFiles([step.path]);
      answer = { ok: true };
    } else if (step.do === "shot") {
      await b.screenshot(step.path);
      answer = { ok: true };
    } else answer = await b.eval(`window.clarityUi.step(${JSON.stringify(step)})`);
  } catch (e) {
    answer = { ok: false, error: String(e.message ?? e) };
  }
  console();
  process.stdout.write(JSON.stringify(answer ?? { ok: false, error: "no answer" }) + "\n");
}
await b.close();
