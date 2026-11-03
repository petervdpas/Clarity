// Clarity.UI's worker (DESIGN 7.9, round B): runs clarity-ui.wasm, draws with
// WebGL2 on the OffscreenCanvas the page handed it, and asks the program on
// the server with synchronous requests, which a worker may make. The page
// script (clarity-ui-page.js) starts it and sends it what the person does;
// it tells the page what only the page can do. Plain JavaScript, no build step.
//
// Messages from the page, by their `type`:
//   start   {canvas, width, height, dpr, dark, files, server, resources, settings, choices}
//   resize  {width, height, dpr}        dark {dark}
//   pointer {kind, x, y, button, held}  wheel {x, y, dx, dy, mode, held}
//   key     {down, key, code, held}     text {text}
//   compose {text, start, end}          paste {text}
//   file    {name, bytes}               sound-ended {id}
//   step    {id, step}                  shot {id}
// Every request of the wire is {"command", "args", "browser", "settings"}:
// `browser` names this tab, `settings` are the person's settings as the page
// keeps them. An answer that carries "settings" changed them: the worker
// keeps them (as keep_settings does) before the library reads the answer.
//
// Messages to the page: {type: "started"} or {type: "failed", what}, a tell of
// the host {type: "tell", tell, ...} (see clarity_ui::browser::page), and the
// answers {type: "stepped", id, answer} and {type: "shot", id, png}.
//
// A long call's request is sent without waiting (askLater): the worker draws
// on while the server runs the handler, and the long-call bar's progress and
// Stop go as requests of their own meanwhile.

"use strict";

let ui = null;
let server = "/clarity/invoke";
let resources = "clarity/resources/";
let settings = {};
let scheduled = false;
let waking = null;
let canvas = null;
// This tab's own number, so the server tells its pages apart.
const browser = self.crypto && self.crypto.randomUUID ? self.crypto.randomUUID() : String(Math.random()).slice(2) + String(Date.now());

function post(message) {
  self.postMessage(message);
}

// The program's answer to a command, as the server gives it; the person's
// settings stay in the page's storage, so the worker answers those itself.
function wire(command, args) {
  return '{"command":' + JSON.stringify(command) + ',"args":' + args + ',"browser":' + JSON.stringify(browser) + ',"settings":' + JSON.stringify(settings) + "}";
}

function keepSettings(kept) {
  settings = kept && typeof kept === "object" ? kept : {};
  post({ type: "tell", tell: "keep", settings });
}

// The server's answer, its settings kept when it brings them.
function answered(text) {
  if (text.indexOf('"settings"') >= 0) {
    try {
      const answer = JSON.parse(text);
      if (answer && answer.settings && typeof answer.settings === "object") keepSettings(answer.settings);
    } catch (why) {
      // The library says what does not read.
    }
  }
  return text;
}

function ask(command, args) {
  if (command === "read_settings") {
    return JSON.stringify({ value: settings });
  }
  if (command === "keep_settings") {
    keepSettings(JSON.parse(args).settings);
    return JSON.stringify({ value: null });
  }
  try {
    const request = new XMLHttpRequest();
    request.open("POST", server, false);
    request.setRequestHeader("Content-Type", "application/json");
    request.send(wire(command, args));
    if (request.status >= 200 && request.status < 300) {
      return answered(request.responseText);
    }
    return JSON.stringify({ error: "The server answered " + command + " with " + request.status + "." });
  } catch (why) {
    return JSON.stringify({ error: "The server could not be reached: " + why });
  }
}

// A long call's request, sent without waiting; its answer goes to the host by `id` when it comes.
function askLater(id, command, args) {
  const answer = (text) => {
    if (!ui) return;
    withText(text, (at) => ui._clarity_ui_answered(id, at));
    want();
  };
  fetch(server, { method: "POST", headers: { "Content-Type": "application/json" }, body: wire(command, args) }).then(
    (response) =>
      response.text().then((text) => answer(response.ok ? answered(text) : JSON.stringify({ error: "The server answered " + command + " with " + response.status + "." }))),
    (why) => answer(JSON.stringify({ error: "The server could not be reached: " + why })),
  );
}

function get(url, binary) {
  const request = new XMLHttpRequest();
  request.open("GET", url, false);
  if (binary) request.responseType = "arraybuffer";
  request.send();
  if (request.status < 200 || request.status >= 300) return null;
  return binary ? new Uint8Array(request.response) : request.responseText;
}

function resource(name) {
  try {
    return get(resources + name.split("/").map(encodeURIComponent).join("/"), true);
  } catch (why) {
    return null;
  }
}

function text(value) {
  return ui.stringToNewUTF8(value == null ? "" : String(value));
}

function taken(pointer) {
  if (!pointer) return null;
  const value = ui.UTF8ToString(pointer);
  ui._clarity_ui_string_free(pointer);
  return value;
}

function withText(value, act) {
  const at = text(value);
  try {
    return act(at);
  } finally {
    ui._free(at);
  }
}

// Frames run while the screen changes or waits for a time it asked for.
function frame() {
  scheduled = false;
  if (!ui) return;
  ui._clarity_ui_frame();
  const wake = ui._clarity_ui_next_wake();
  if (wake === 0) {
    want();
  } else if (wake > 0) {
    clearTimeout(waking);
    waking = setTimeout(want, Math.min(wake, 1000));
  }
}

function want() {
  if (scheduled) return;
  scheduled = true;
  if (self.requestAnimationFrame) self.requestAnimationFrame(frame);
  else setTimeout(frame, 16);
}

function size(width, height, dpr) {
  canvas.width = Math.max(1, Math.round(width * dpr));
  canvas.height = Math.max(1, Math.round(height * dpr));
}

async function start(message) {
  server = message.server || server;
  resources = message.resources || resources;
  settings = message.settings || {};
  canvas = message.canvas;
  size(message.width, message.height, message.dpr);
  importScripts(message.script || "clarity-ui.js");
  ui = await ClarityUi({
    clarityAsk: ask,
    clarityAskLater: askLater,
    clarityTell: (json) => post(Object.assign({ type: "tell" }, JSON.parse(json))),
    clarityResource: resource,
    // The wasm is clarity-ui.wasm beside the script, whatever Emscripten named it.
    locateFile: (path) => new URL(path.endsWith(".wasm") ? "clarity-ui.wasm" : path, message.base || self.location.href).href,
    print: (line) => console.log(line),
    printErr: (line) => console.error(line),
  });
  const gl = canvas.getContext("webgl2", { alpha: false, antialias: false, depth: false, stencil: true, premultipliedAlpha: true, preserveDrawingBuffer: false });
  if (!gl) {
    post({ type: "failed", what: "This browser gives no WebGL2." });
    return;
  }
  ui.GL.makeContextCurrent(ui.GL.registerContext(gl, { majorVersion: 2, minorVersion: 0 }));
  const files = get(message.files || "clarity/files.json", false);
  if (files == null) {
    post({ type: "failed", what: "The program's files did not come: " + (message.files || "clarity/files.json") });
    return;
  }
  const choices = JSON.stringify(message.choices || {});
  const why = withText(files, (at) => withText(choices, (kept) => taken(ui._clarity_ui_start(at, kept, message.width, message.height, message.dpr, message.dark ? 1 : 0))));
  if (why) {
    post({ type: "failed", what: why });
    return;
  }
  post({ type: "started" });
  want();
}

const handlers = {
  start,
  resize: (m) => {
    size(m.width, m.height, m.dpr);
    ui._clarity_ui_resize(m.width, m.height, m.dpr);
  },
  dark: (m) => ui._clarity_ui_dark(m.dark ? 1 : 0),
  pointer: (m) => ui._clarity_ui_pointer(m.kind, m.x, m.y, m.button, m.held),
  wheel: (m) => ui._clarity_ui_wheel(m.x, m.y, m.dx, m.dy, m.mode, m.held),
  key: (m) => withText(m.key, (key) => withText(m.code, (code) => ui._clarity_ui_key(m.down ? 1 : 0, key, code, m.held))),
  text: (m) => withText(m.text, (at) => ui._clarity_ui_text(at)),
  compose: (m) => withText(m.text, (at) => ui._clarity_ui_compose(at, m.start == null ? -1 : m.start, m.end == null ? -1 : m.end)),
  paste: (m) => withText(m.text, (at) => ui._clarity_ui_pasted(at)),
  "sound-ended": (m) => ui._clarity_ui_sound_ended(m.id),
  file: (m) => {
    const bytes = new Uint8Array(m.bytes);
    const at = ui._malloc(Math.max(bytes.length, 1));
    ui.HEAPU8.set(bytes, at);
    try {
      withText(m.name, (name) => ui._clarity_ui_file_chosen(name, at, bytes.length));
    } finally {
      ui._free(at);
    }
  },
  step: (m) => post({ type: "stepped", id: m.id, answer: JSON.parse(withText(JSON.stringify(m.step), (at) => taken(ui._clarity_ui_step(at)))) }),
  shot: (m) => post({ type: "shot", id: m.id, png: taken(ui._clarity_ui_shot()) }),
};

self.onmessage = (event) => {
  const message = event.data;
  const handler = handlers[message.type];
  if (!handler || (!ui && message.type !== "start")) return;
  try {
    handler(message);
  } catch (why) {
    console.error("Clarity.UI:", why);
  }
  if (ui) want();
};
