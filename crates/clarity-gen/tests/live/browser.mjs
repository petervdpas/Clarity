// A real browser for the live tests (tests/everything_live.rs): Chromium,
// headless, driven over its DevTools protocol with Node's own WebSocket, so
// nothing is installed. Input goes in as a person's does: key presses,
// typed text, pointer presses, moves and releases at a part's place.

import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { join } from "node:path";

const KEYS = {
  ArrowUp: { code: "ArrowUp", vk: 38 },
  ArrowDown: { code: "ArrowDown", vk: 40 },
  ArrowLeft: { code: "ArrowLeft", vk: 37 },
  ArrowRight: { code: "ArrowRight", vk: 39 },
  Enter: { code: "Enter", vk: 13, text: "\r" },
  Tab: { code: "Tab", vk: 9 },
  Escape: { code: "Escape", vk: 27 },
  z: { code: "KeyZ", vk: 90, text: "z" },
  y: { code: "KeyY", vk: 89, text: "y" },
  a: { code: "KeyA", vk: 65, text: "a" },
};

const MODIFIERS = { Alt: 1, Control: 2, Meta: 4, Shift: 8 };

export async function openBrowser(url, { width = 1400, height = 1000, folder }) {
  // The profile lives beside the program, not in /tmp (which may be memory).
  const profile = mkdtempSync(join(folder, "browser-"));
  // Cut off from the person's desktop: Chromium's own binary (not the
  // launcher, which joins the desktop session), without their display,
  // session bus or keyring. A test browser never touches the desktop.
  const env = { ...process.env };
  for (const name of ["DISPLAY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS", "XDG_SESSION_ID", "XDG_SESSION_TYPE", "XDG_SESSION_DESKTOP", "XDG_CURRENT_DESKTOP", "DESKTOP_SESSION", "GNOME_KEYRING_CONTROL", "SSH_AUTH_SOCK"]) delete env[name];
  const binary = existsSync("/usr/lib/chromium/chromium") ? "/usr/lib/chromium/chromium" : "chromium";
  const chrome = spawn(
    binary,
    ["--headless=new", "--no-first-run", "--no-default-browser-check", "--disable-gpu", "--password-store=basic", "--use-mock-keychain", "--disable-dev-shm-usage", "--disable-features=MediaRouter,DialMediaRouteProvider", `--window-size=${width},${height}`, "--remote-debugging-port=0", `--user-data-dir=${profile}`, "about:blank"],
    { stdio: ["ignore", "ignore", "pipe"], env, detached: false },
  );
  const port = await new Promise((resolve, reject) => {
    let said = "";
    const timer = setTimeout(() => reject(new Error(`chromium did not start: ${said}`)), 20000);
    chrome.stderr.on("data", (d) => {
      said += d;
      const m = said.match(/DevTools listening on ws:\/\/[^:]+:(\d+)\//);
      if (m) {
        clearTimeout(timer);
        resolve(Number(m[1]));
      }
    });
  });
  const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  const page = pages.find((p) => p.type === "page");
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = reject;
  });
  let next = 1;
  const waiting = new Map();
  const logs = [];
  ws.onmessage = (m) => {
    const msg = JSON.parse(m.data);
    if (msg.id && waiting.has(msg.id)) {
      const { resolve, reject } = waiting.get(msg.id);
      waiting.delete(msg.id);
      msg.error ? reject(new Error(`${msg.error.message} ${msg.error.data ?? ""}`)) : resolve(msg.result);
    } else if (msg.method === "Runtime.consoleAPICalled" && ["error", "warning"].includes(msg.params.type)) {
      // Where it was said, for a warning that names no place itself.
      const where = (msg.params.stackTrace?.callFrames ?? []).slice(0, 40).map((f) => `${f.functionName || "?"}:${f.lineNumber + 1}:${f.columnNumber + 1}`).join(" < ");
      logs.push(`${msg.params.type}: ${msg.params.args.map((a) => a.value ?? a.description).join(" ")}${where ? `\n         at ${where}` : ""}`);
    } else if (msg.method === "Runtime.exceptionThrown") {
      logs.push(`exception: ${msg.params.exceptionDetails.exception?.description ?? msg.params.exceptionDetails.text}`);
    }
  };
  const send = (method, params = {}) =>
    new Promise((resolve, reject) => {
      const id = next++;
      waiting.set(id, { resolve, reject });
      ws.send(JSON.stringify({ id, method, params }));
    });
  await send("Runtime.enable");
  await send("Page.enable");
  await send("DOM.enable");
  await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
  await send("Page.navigate", { url });

  const b = {
    send,
    logs,
    /** The value of a JavaScript expression in the page. */
    async eval(expression) {
      const r = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
      if (r.exceptionDetails) throw new Error(`${expression}: ${r.exceptionDetails.exception?.description ?? r.exceptionDetails.text}`);
      return r.result.value;
    },
    /** Waits until `expression` is true in the page. */
    async until(expression, what = expression, ms = 10000) {
      const end = Date.now() + ms;
      while (Date.now() < end) {
        if (await b.eval(`!!(${expression})`).catch(() => false)) return;
        await new Promise((r) => setTimeout(r, 50));
      }
      throw new Error(`waited ${ms} ms for ${what}`);
    },
    async settle(ms = 300) {
      await new Promise((r) => setTimeout(r, ms));
    },
    /** The middle (or a point by fractions) of the element `selector` finds. */
    async at(selector, fx = 0.5, fy = 0.5) {
      const box = await b.eval(`(() => { const e = document.querySelector(${JSON.stringify(selector)}); if (!e) return null; e.scrollIntoView({ block: "nearest", inline: "nearest" }); const r = e.getBoundingClientRect(); return { x: r.left, y: r.top, w: r.width, h: r.height }; })()`);
      if (!box) throw new Error(`nothing at ${selector}`);
      return { x: box.x + box.w * fx, y: box.y + box.h * fy };
    },
    async mouse(type, { x, y }, buttons = 0) {
      await send("Input.dispatchMouseEvent", { type, x, y, button: type === "mouseMoved" ? (buttons ? "left" : "none") : "left", buttons, clickCount: type === "mouseMoved" ? 0 : 1 });
    },
    async click(selector, fx, fy) {
      const p = await b.at(selector, fx, fy);
      await b.mouse("mouseMoved", p);
      await b.mouse("mousePressed", p, 1);
      await b.mouse("mouseReleased", p, 0);
      await b.settle(150);
    },
    async doubleClick(selector) {
      const p = await b.at(selector);
      for (const clickCount of [1, 2]) {
        await send("Input.dispatchMouseEvent", { type: "mousePressed", ...p, button: "left", buttons: 1, clickCount });
        await send("Input.dispatchMouseEvent", { type: "mouseReleased", ...p, button: "left", buttons: 0, clickCount });
      }
      await b.settle(150);
    },
    /** A press at `from`, moves in steps, a release at `to`: a person's drag. */
    async drag(from, to, steps = 8) {
      await b.mouse("mouseMoved", from);
      await b.mouse("mousePressed", from, 1);
      for (let i = 1; i <= steps; i++) await b.mouse("mouseMoved", { x: from.x + ((to.x - from.x) * i) / steps, y: from.y + ((to.y - from.y) * i) / steps }, 1);
      await b.mouse("mouseReleased", to, 0);
      await b.settle(300);
    },
    /** A key, with modifiers such as ["Control"] or ["Alt"]. */
    async key(key, modifiers = []) {
      const k = KEYS[key] ?? { code: key, vk: 0 };
      const mods = modifiers.reduce((m, n) => m | MODIFIERS[n], 0);
      const text = mods & (MODIFIERS.Control | MODIFIERS.Alt | MODIFIERS.Meta) ? undefined : k.text;
      await send("Input.dispatchKeyEvent", { type: text ? "keyDown" : "rawKeyDown", key, code: k.code, windowsVirtualKeyCode: k.vk, modifiers: mods, text });
      await send("Input.dispatchKeyEvent", { type: "keyUp", key, code: k.code, windowsVirtualKeyCode: k.vk, modifiers: mods });
      await b.settle(200);
    },
    /** Text typed key by key, as into a date input, which takes no pasted text. */
    async typeKeys(text) {
      for (const ch of text) {
        const code = /[0-9]/.test(ch) ? `Digit${ch}` : `Key${ch.toUpperCase()}`;
        await send("Input.dispatchKeyEvent", { type: "keyDown", key: ch, code, text: ch, windowsVirtualKeyCode: ch.toUpperCase().charCodeAt(0) });
        await send("Input.dispatchKeyEvent", { type: "keyUp", key: ch, code, windowsVirtualKeyCode: ch.toUpperCase().charCodeAt(0) });
      }
      await b.settle(100);
    },
    async type(text) {
      await send("Input.insertText", { text });
      await b.settle(100);
    },
    async files(selector, paths) {
      const { root } = await send("DOM.getDocument", { depth: -1, pierce: true });
      const { nodeId } = await send("DOM.querySelector", { nodeId: root.nodeId, selector });
      if (!nodeId) throw new Error(`no file input at ${selector}`);
      await send("DOM.setFileInputFiles", { nodeId, files: paths });
      await b.settle(300);
    },
    async screenshot(path) {
      const { data } = await send("Page.captureScreenshot", { format: "png" });
      (await import("node:fs")).writeFileSync(path, Buffer.from(data, "base64"));
    },
    async close() {
      try {
        ws.close();
      } catch {}
      const ended = new Promise((resolve) => chrome.once("exit", resolve));
      chrome.kill();
      await ended;
      // Chromium's helpers may still be writing a moment after it ended.
      for (let i = 0; i < 20; i++) {
        try {
          rmSync(profile, { recursive: true, force: true });
          break;
        } catch {
          await new Promise((r) => setTimeout(r, 250));
        }
      }
    },
  };
  return b;
}
