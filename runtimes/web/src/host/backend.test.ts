// How the renderer reaches the backend of the application it runs in.

import { test } from "node:test";
import assert from "node:assert/strict";
import { inBrowser, isTauri, logLine, transport, unwrap, type BrowserPage } from "./backend.ts";

test("a Tauri window has Tauri's internals; a Wails window does not", () => {
  assert.equal(isTauri({ __TAURI_INTERNALS__: {} }), true);
  assert.equal(isTauri({ _wails: {} }), false);
});

test("the Go shell answers with the value, or with what the screen shows as the error", () => {
  assert.deepEqual(unwrap({ value: { state: 1 } }), { state: 1 });
  assert.equal(unwrap({ value: null }), null);
  const refusal = { text: "Order {n} is over", values: {} };
  assert.throws(() => unwrap({ error: refusal }), (e) => e === refusal);
  assert.throws(() => unwrap({ error: "storage: locked" }), (e) => e === "storage: locked");
});

test("each shell is told by what it puts in the page", () => {
  assert.equal(transport({ __TAURI_INTERNALS__: {} }), "tauri");
  // The C and C++ shells bind one function into the page, through webview.
  assert.equal(transport({ clarityInvoke: () => {} }), "webview");
  assert.equal(transport({}), "wails");
});

test("an engine's line goes to the shell, which writes it into the application's log", async () => {
  const sent: [string, Record<string, unknown>][] = [];
  const page = globalThis as unknown as { clarityInvoke?: unknown };
  page.clarityInvoke = async (command: string, args: Record<string, unknown>) => {
    sent.push([command, args]);
    return { value: null };
  };
  try {
    await logLine("engine Screen: level 2");
  } finally {
    delete page.clarityInvoke;
  }
  assert.deepEqual(sent, [["log_line", { line: "engine Screen: level 2" }]]);
});

test("a page a server serves talks to it over HTTP", () => {
  assert.equal(transport({ clarityServer: "/clarity/invoke" }), "server");
});

test("a page a server serves does in the browser what belongs to it", () => {
  const done: string[] = [];
  const stored = new Map<string, string>();
  const page: BrowserPage = {
    setTitle: (title) => done.push(`title ${title}`),
    open: (address) => done.push(`open ${address}`),
    download: (name, data) => done.push(`download ${name} ${data}`),
    show: (name, data) => done.push(`show ${name} ${data}`),
    read: (key) => stored.get(key) ?? null,
    write: (key, value) => stored.set(key, value),
  };
  assert.deepEqual(inBrowser("set_title", { title: "Pills" }, page), { value: null });
  assert.deepEqual(inBrowser("open_link", { address: "https://example.com" }, page), { value: null });
  assert.deepEqual(inBrowser("open_link", { address: "file:///etc/passwd" }, page), { error: "a link opens only an http, https or mailto address, not file:///etc/passwd" });
  assert.deepEqual(inBrowser("save_file", { name: "rules.pdf", data: "AAEC" }, page), { value: "rules.pdf" });
  assert.deepEqual(inBrowser("open_file", { name: "rules.pdf", data: "AAEC" }, page), { value: null });
  assert.deepEqual(inBrowser("quit_app", {}, page), { value: null });
  // The person's settings stay in their browser, without the secret ones.
  assert.deepEqual(inBrowser("read_settings", {}, page), { value: null });
  assert.deepEqual(inBrowser("keep_settings", { settings: { language: "nl", password: "x" }, secrets: ["password"] }, page), { value: null });
  assert.deepEqual(inBrowser("read_settings", {}, page), { value: { language: "nl" } });
  // The rest goes to the server.
  assert.equal(inBrowser("dispatch_event", {}, page), null);
  assert.deepEqual(done, ["title Pills", "open https://example.com", "download rules.pdf AAEC", "show rules.pdf AAEC"]);
});
