// Clarity.UI's page script (DESIGN 7.9, round B): owns the <canvas>, hands it
// to the worker (clarity-ui-worker.js) as an OffscreenCanvas, and sends it what
// the person does: pointer, wheel, keys, typed text and an input method's
// (through a hidden textarea), size and device pixel ratio, light or dark. It
// does what only the page can: the clipboard, the file input, downloads, the
// title, links, the person's kept settings (localStorage) and sound (Web Audio).
// Plain JavaScript, no build step.
//
// The page has <canvas id="clarity-ui"></canvas> and loads this script; it
// starts by itself. window.clarityUiOptions may change where things are:
//   worker     the worker script     (clarity-ui-worker.js beside this script)
//   files      the program's files   ("clarity/files.json")
//   server     the program's wire    (window.clarityServer or "/clarity/invoke")
//   resources  its resources folder  ("clarity/resources/")
//   storage    the localStorage key of the kept settings ("clarity.settings")
//   choices    the localStorage key of the person's choices: language, theme, columns ("clarity.choices")
//   thread     "worker" or "page"    (by itself: the worker where a worker can draw with WebGL2, else the page)
//   input      false: the drawing takes no input (the IDE's designer lays its own layer over it)
//   chosen     choices that win over the kept ones: {language: "nl"} (the IDE's language for its preview)
// window.clarityUi then has `started` (a promise), `step(step)` and `shot()`
// (promises of a driven step's answer and of the canvas as a base64 PNG),
// `reload(files)` (starts the program again from new files on the same
// canvas), `layout()` (where the design's parts and grids show) and `stop()`.
// window.ClarityUiPage.start(canvas, options) does the same for a canvas of
// a page's own choosing (the IDE's preview and designer).
//
// The worker draws on an OffscreenCanvas with WebGL2. A browser whose
// workers cannot (WebKitGTK, the webview of Tauri on Linux) runs the same
// worker script on the page's thread instead (ClarityUiPageWorker), on the
// page's own canvas; its requests to the server are synchronous there too.

(function () {
  "use strict";

  const script = document.currentScript ? document.currentScript.src : location.href;
  const here = (path) => new URL(path, script).href;
  const page = (path) => new URL(path, location.href).href;

  function readKept(key) {
    try {
      return JSON.parse(localStorage.getItem(key) || "{}") || {};
    } catch (why) {
      return {};
    }
  }

  function keepIn(key, value) {
    try {
      localStorage.setItem(key, JSON.stringify(value || {}));
    } catch (why) {
      // A page without storage keeps them for this visit only.
    }
  }

  // The addresses a link opens: web pages, mail, phone numbers (clarity_ui::window::address::opens).
  function opens(address) {
    return typeof address === "string" && /^(https?|mailto|tel):/i.test(address) && !/[\u0000-\u001f\u007f]/.test(address);
  }

  function held(event) {
    return (event.shiftKey ? 1 : 0) | (event.ctrlKey ? 2 : 0) | (event.altKey ? 4 : 0) | (event.metaKey ? 8 : 0);
  }

  function bytesOf(base64) {
    const raw = atob(base64);
    const bytes = new Uint8Array(raw.length);
    for (let i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i);
    return bytes;
  }

  // Whether a worker of this browser can draw with WebGL2 on an OffscreenCanvas.
  let drawing = null;
  function workerDraws() {
    if (typeof OffscreenCanvas === "undefined" || !HTMLCanvasElement.prototype.transferControlToOffscreen || typeof Worker === "undefined") return Promise.resolve(false);
    drawing =
      drawing ||
      new Promise((resolve) => {
        let probe;
        try {
          const code = "try { postMessage(!!new OffscreenCanvas(1, 1).getContext('webgl2')); } catch (why) { postMessage(false); }";
          probe = new Worker(URL.createObjectURL(new Blob([code], { type: "text/javascript" })));
        } catch (why) {
          resolve(false);
          return;
        }
        const done = (yes) => {
          probe.terminate();
          resolve(yes);
        };
        probe.onmessage = (event) => done(event.data === true);
        probe.onerror = () => done(false);
        setTimeout(() => done(false), 3000);
      });
    return drawing;
  }

  // The worker script from another origin (the IDE's own server) starts in a worker of this page's.
  function workerOf(url) {
    if (new URL(url).origin === location.origin) return new Worker(url);
    const code = "importScripts(" + JSON.stringify(url) + ");";
    return new Worker(URL.createObjectURL(new Blob([code], { type: "text/javascript" })));
  }

  // The worker script on this page's thread, loaded once.
  let pageScript = null;
  function pageWorker(url) {
    pageScript =
      pageScript ||
      new Promise((resolve, reject) => {
        if (window.ClarityUiPageWorker) return resolve();
        const element = document.createElement("script");
        element.src = url;
        element.onload = () => resolve();
        element.onerror = () => reject(new Error("Clarity.UI's worker script did not load: " + url));
        document.head.appendChild(element);
      });
    return pageScript.then(() => window.ClarityUiPageWorker());
  }

  function start(canvas, given) {
    const options = Object.assign({}, window.clarityUiOptions || {}, given || {});
    const storageKey = options.storage || "clarity.settings";
    const choicesKey = options.choices || "clarity.choices";
    const takesInput = options.input !== false;
    if (!canvas.style.width) canvas.style.width = "100%";
    if (!canvas.style.height) canvas.style.height = "100%";
    canvas.style.display = "block";
    canvas.style.touchAction = "none";
    canvas.style.outline = "none";

    const workerUrl = options.worker ? page(options.worker) : here("clarity-ui-worker.js");
    const libraryUrl = options.library ? page(options.library) : here("clarity-ui.js");
    // The worker, once it is known where the library runs; messages wait for it.
    let worker = null;
    const queued = [];
    let stopped = false;
    const dark = window.matchMedia ? window.matchMedia("(prefers-color-scheme: dark)") : null;
    let answered = 0;
    const waiting = new Map();
    let startedResolve, startedReject;
    const started = new Promise((resolve, reject) => {
      startedResolve = resolve;
      startedReject = reject;
    });
    const send = (message, transfer) => (worker ? worker.postMessage(message, transfer || []) : queued.push([message, transfer || []]));
    const place = () => canvas.getBoundingClientRect();
    const box = () => ({ width: canvas.clientWidth || 1, height: canvas.clientHeight || 1, dpr: window.devicePixelRatio || 1 });
    // A point of the page in the canvas's own CSS pixels, also where the canvas is shown scaled (the IDE fits a window to its room).
    const within = (event) => {
      const at = place();
      const sx = at.width > 0 ? canvas.clientWidth / at.width : 1;
      const sy = at.height > 0 ? canvas.clientHeight / at.height : 1;
      return { x: (event.clientX - at.left) * sx, y: (event.clientY - at.top) * sy };
    };

    // The hidden textarea takes keys, typed text, an input method's text and pastes.
    const typing = document.createElement("textarea");
    typing.setAttribute("autocapitalize", "off");
    typing.setAttribute("autocomplete", "off");
    typing.setAttribute("spellcheck", "false");
    typing.setAttribute("aria-hidden", "true");
    Object.assign(typing.style, { position: "fixed", left: "0px", top: "0px", width: "1px", height: "1px", opacity: "0", border: "0", padding: "0", resize: "none", overflow: "hidden", pointerEvents: "none" });
    if (takesInput) document.body.appendChild(typing);
    const focusTyping = () => typing.focus({ preventScroll: true });
    // Whether the last press was a finger's.
    let touched = false;

    // Sound through Web Audio, started by the person's first press.
    let audio = null;
    const playing = new Map();
    function play(tell) {
      audio = audio || new (window.AudioContext || window.webkitAudioContext)();
      const bytes = bytesOf(tell.data);
      audio.decodeAudioData(bytes.buffer).then((buffer) => {
        const source = audio.createBufferSource();
        const gain = audio.createGain();
        gain.gain.value = tell.volume;
        source.buffer = buffer;
        source.loop = !!tell.repeat;
        source.connect(gain).connect(audio.destination);
        source.onended = () => {
          playing.delete(tell.id);
          send({ type: "sound-ended", id: tell.id });
        };
        playing.set(tell.id, source);
        source.start();
      }, () => send({ type: "sound-ended", id: tell.id }));
    }

    // A file shown in a tab has its type (the host sends only kinds that run
    // nothing); one without is downloaded, never shown in this page's origin.
    function save(name, base64, open, type) {
      open = open && !!type;
      const url = URL.createObjectURL(open ? new Blob([bytesOf(base64)], { type }) : new Blob([bytesOf(base64)], { type: "application/octet-stream" }));
      if (open) {
        window.open(url, "_blank", "noopener");
      } else {
        const link = document.createElement("a");
        link.href = url;
        link.download = name;
        document.body.appendChild(link);
        link.click();
        link.remove();
      }
      setTimeout(() => URL.revokeObjectURL(url), 60000);
    }

    function pick(tell) {
      const input = document.createElement("input");
      input.type = "file";
      if (tell.accept && tell.accept.length) input.accept = tell.accept.join(",");
      input.onchange = () => {
        const file = input.files && input.files[0];
        if (!file) return;
        file.arrayBuffer().then((bytes) => worker.postMessage({ type: "file", name: file.name, bytes }, [bytes]));
      };
      input.click();
    }

    const tells = {
      title: (t) => (document.title = t.title),
      cursor: (t) => (canvas.style.cursor = t.cursor),
      caret: (t) => {
        // A touch shows the on-screen keyboard only where text is typed.
        if (touched) {
          if (t.at) focusTyping();
          else typing.blur();
        }
        if (!t.at) return;
        const at = place();
        typing.style.left = at.left + t.at[0] + "px";
        typing.style.top = at.top + t.at[1] + t.at[3] + "px";
      },
      copy: (t) => navigator.clipboard && navigator.clipboard.writeText(t.text).catch(() => {}),
      // Only what a link opens on the desktop (the host checks it too).
      open: (t) => opens(t.address) && window.open(t.address, "_blank", "noopener"),
      download: (t) => save(t.name, t.data, false),
      show: (t) => save(t.name, t.data, true, t.type),
      play,
      stop: (t) => {
        const source = playing.get(t.id);
        if (source) source.stop();
      },
      keep: (t) => keepIn(storageKey, t.settings),
      choices: (t) => keepIn(choicesKey, t.choices),
      pick,
      ended: () => {},
      failed: (t) => console.error("Clarity.UI:", t.what),
    };

    const onmessage = (event) => {
      const message = event.data;
      if (message.type === "tell") {
        const tell = tells[message.tell];
        if (tell) tell(message);
      } else if (message.type === "started") {
        startedResolve();
      } else if (message.type === "failed") {
        console.error("Clarity.UI:", message.what);
        startedReject(new Error(message.what));
      } else if (["stepped", "shot", "reloaded", "laid"].includes(message.type)) {
        const done = waiting.get(message.id);
        waiting.delete(message.id);
        const answers = { stepped: message.answer, shot: message.png, reloaded: message.why || null, laid: message.layout };
        if (done) done(answers[message.type]);
      }
    };

    const thread = options.thread === "page" || options.thread === "worker" ? Promise.resolve(options.thread === "worker") : workerDraws();
    thread
      .then((inWorker) => {
        if (stopped) return;
        const now = box();
        const message = {
          type: "start",
          width: now.width,
          height: now.height,
          dpr: now.dpr,
          dark: !!(dark && dark.matches),
          script: libraryUrl,
          base: libraryUrl,
          files: page(options.files || "clarity/files.json"),
          server: page(options.server || window.clarityServer || "/clarity/invoke"),
          resources: page(options.resources || "clarity/resources/"),
          settings: readKept(storageKey),
          choices: Object.assign(readKept(choicesKey), options.chosen || {}),
        };
        if (inWorker) {
          const offscreen = canvas.transferControlToOffscreen();
          worker = workerOf(workerUrl);
          worker.onmessage = onmessage;
          worker.postMessage(Object.assign(message, { canvas: offscreen }), [offscreen]);
          return;
        }
        return pageWorker(workerUrl).then((face) => {
          if (stopped) return;
          worker = face;
          worker.onmessage = onmessage;
          worker.postMessage(Object.assign(message, { canvas }));
        });
      })
      .then(() => {
        for (const [message, transfer] of queued.splice(0)) if (worker) worker.postMessage(message, transfer);
      })
      .catch((why) => startedReject(why));

    const resized = () => send(Object.assign({ type: "resize" }, box()));
    const watching = window.ResizeObserver ? new ResizeObserver(resized) : null;
    if (watching) watching.observe(canvas);
    window.addEventListener("resize", resized);
    const darkened = () => send({ type: "dark", dark: dark.matches });
    if (dark && dark.addEventListener) dark.addEventListener("change", darkened);

    // One pointer at a time: a second finger is not a second mouse. A mouse
    // has one pointer, so a mouse event counts even when it does not say
    // isPrimary: one a page's own script makes (new PointerEvent) leaves it
    // false unless told, and leaves pointerType empty unless told.
    const primary = (event) => event.isPrimary || event.pointerType === "mouse" || event.pointerType === "";
    const pointer = (kind) => (event) => {
      if (!primary(event)) return;
      const at = within(event);
      send({ type: "pointer", kind, x: at.x, y: at.y, button: event.button, held: held(event) });
    };
    if (takesInput) {
      canvas.addEventListener("pointermove", pointer(0));
      canvas.addEventListener("pointerdown", (event) => {
        if (!primary(event)) return;
        try {
          canvas.setPointerCapture(event.pointerId);
        } catch (why) {
          // A pointer the page made up (a test's) cannot be captured.
        }
        if (audio && audio.state === "suspended") audio.resume();
        pointer(1)(event);
        event.preventDefault();
        touched = event.pointerType === "touch";
        if (!touched) focusTyping();
      });
      canvas.addEventListener("pointerup", pointer(2));
      canvas.addEventListener("pointerleave", pointer(3));
      canvas.addEventListener("pointercancel", pointer(4));
      canvas.addEventListener("contextmenu", (event) => event.preventDefault());
      canvas.addEventListener(
        "wheel",
        (event) => {
          const at = within(event);
          send({ type: "wheel", x: at.x, y: at.y, dx: event.deltaX, dy: event.deltaY, mode: event.deltaMode, held: held(event) });
          event.preventDefault();
        },
        { passive: false },
      );
    }

    // A key that writes text goes as a key and, through the textarea, as
    // text; the others (and shortcuts) only as a key. Ctrl+V waits for the
    // paste, so the worker knows the clipboard's text before the key.
    // Keys the browser keeps for itself: reload, zoom, full screen, its
    // tools and tabs. The page neither takes nor sends them.
    function kept(event) {
      const command = event.ctrlKey || event.metaKey;
      if (["F5", "F11", "F12"].includes(event.key)) return true;
      if (!command) return false;
      const key = event.key.toLowerCase();
      if (["+", "=", "-", "0", "r", "l", "t", "w", "n", "tab"].includes(key)) return true;
      return event.shiftKey && ["i", "j", "c"].includes(key);
    }

    // What a key down is, by how it writes: text keys go as a key with the
    // keys held that are not part of the writing, and the browser writes
    // their text into the textarea (`input`). AltGr (Ctrl+Alt on Windows) and
    // the Mac's Option with a key that writes a letter of its own write text;
    // a dead key writes nothing yet (its letter comes with the next key).
    function keyDown(event) {
      if (event.isComposing || event.keyCode === 229 || event.key === "Unidentified" || event.key === "Dead") return { send: false, prevent: false };
      if (kept(event)) return { send: false, prevent: false };
      const writes = event.key.length === 1 || [...event.key].length === 1;
      const altGraph = event.getModifierState && event.getModifierState("AltGraph");
      const option = event.altKey && !event.ctrlKey && !event.metaKey && writes && !/^[a-z0-9]$/i.test(event.key);
      if (writes && (altGraph || option)) return { send: true, held: held(event) & 1, prevent: false };
      const shortcut = event.ctrlKey || event.metaKey || event.altKey;
      return { send: true, held: held(event), prevent: shortcut || !writes };
    }

    let pasteKey = null;
    typing.addEventListener("keydown", (event) => {
      const shortcut = event.ctrlKey || event.metaKey;
      if (shortcut && !event.altKey && (event.key === "v" || event.key === "V")) {
        pasteKey = { key: event.key, code: event.code, held: held(event) };
        return;
      }
      const down = keyDown(event);
      if (down.send) send({ type: "key", down: true, key: event.key, code: event.code, held: down.held });
      if (down.prevent) event.preventDefault();
    });
    typing.addEventListener("keyup", (event) => {
      const up = keyDown(event);
      if (up.send) send({ type: "key", down: false, key: event.key, code: event.code, held: up.held });
    });
    // A phone's keyboard sends no keys (229, Unidentified) but what it does
    // to the text: deleting and new lines are the keys, text is typed.
    const editKeys = { deleteContentBackward: ["Backspace", "Backspace"], deleteContentForward: ["Delete", "Delete"], insertLineBreak: ["Enter", "Enter"], insertParagraph: ["Enter", "Enter"] };
    typing.addEventListener("beforeinput", (event) => {
      const key = editKeys[event.inputType];
      if (!key) return;
      event.preventDefault();
      send({ type: "key", down: true, key: key[0], code: key[1], held: 0 });
      send({ type: "key", down: false, key: key[0], code: key[1], held: 0 });
    });
    typing.addEventListener("input", (event) => {
      if (event.isComposing) return;
      if (typing.value) send({ type: "text", text: typing.value });
      typing.value = "";
    });
    typing.addEventListener("compositionupdate", (event) => send({ type: "compose", text: event.data || "", start: null, end: null }));
    typing.addEventListener("compositionend", (event) => {
      send({ type: "compose", text: "", start: null, end: null });
      if (event.data) send({ type: "text", text: event.data });
      typing.value = "";
    });
    typing.addEventListener("paste", (event) => {
      event.preventDefault();
      const text = event.clipboardData ? event.clipboardData.getData("text/plain") : "";
      send({ type: "paste", text });
      const key = pasteKey || { key: "v", code: "KeyV", held: 2 };
      pasteKey = null;
      send({ type: "key", down: true, key: key.key, code: key.code, held: key.held });
      send({ type: "key", down: false, key: key.key, code: key.code, held: key.held });
    });
    if (takesInput) {
      canvas.tabIndex = 0;
      canvas.addEventListener("focus", focusTyping);
    }

    function asked(type, extra) {
      return started.then(
        () =>
          new Promise((resolve) => {
            const id = ++answered;
            waiting.set(id, resolve);
            send(Object.assign({ type, id }, extra));
          }),
      );
    }

    return {
      get worker() {
        return worker;
      },
      canvas,
      started,
      step: (step) => asked("step", { step }),
      shot: () => asked("shot", {}),
      // Starts the program again from new files (a URL of files.json) on the same canvas, with
      // choices that win over the kept ones (`chosen`, as the option): null, or why it could not.
      reload: (files, chosen) => asked("reload", { files: page(files), choices: Object.assign(readKept(choicesKey), chosen || options.chosen || {}) }),
      layout: () => asked("layout", {}),
      // Ends it: no more frames, no more requests.
      stop: () => {
        stopped = true;
        if (worker) worker.terminate();
        if (watching) watching.disconnect();
        window.removeEventListener("resize", resized);
        if (dark && dark.removeEventListener) dark.removeEventListener("change", darkened);
        typing.remove();
      },
    };
  }

  window.ClarityUiPage = { start };

  function boot() {
    const canvas = document.getElementById("clarity-ui");
    if (canvas && !window.clarityUi) window.clarityUi = start(canvas);
  }

  // At once when the canvas comes before this script, so the page's own
  // scripts after it find window.clarityUi; else once the page is read.
  if (document.getElementById("clarity-ui") && document.body) boot();
  else document.addEventListener("DOMContentLoaded", boot);
})();
