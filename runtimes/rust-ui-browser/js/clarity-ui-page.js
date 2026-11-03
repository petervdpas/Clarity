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
// window.clarityUi then has `started` (a promise), `step(step)` and `shot()`
// (promises of a driven step's answer and of the canvas as a base64 PNG).

(function () {
  "use strict";

  const script = document.currentScript ? document.currentScript.src : location.href;
  const options = Object.assign({}, window.clarityUiOptions || {});
  const here = (path) => new URL(path, script).href;
  const page = (path) => new URL(path, location.href).href;
  const storageKey = options.storage || "clarity.settings";
  const choicesKey = options.choices || "clarity.choices";

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

  function start(canvas) {
    if (!canvas.style.width) canvas.style.width = "100%";
    if (!canvas.style.height) canvas.style.height = "100%";
    canvas.style.display = "block";
    canvas.style.touchAction = "none";
    canvas.style.outline = "none";

    const worker = new Worker(options.worker ? page(options.worker) : here("clarity-ui-worker.js"));
    const offscreen = canvas.transferControlToOffscreen();
    const dark = window.matchMedia ? window.matchMedia("(prefers-color-scheme: dark)") : null;
    let answered = 0;
    const waiting = new Map();
    let startedResolve, startedReject;
    const started = new Promise((resolve, reject) => {
      startedResolve = resolve;
      startedReject = reject;
    });
    const send = (message, transfer) => worker.postMessage(message, transfer || []);
    const place = () => canvas.getBoundingClientRect();
    const box = () => ({ width: canvas.clientWidth || 1, height: canvas.clientHeight || 1, dpr: window.devicePixelRatio || 1 });

    // The hidden textarea takes keys, typed text, an input method's text and pastes.
    const typing = document.createElement("textarea");
    typing.setAttribute("autocapitalize", "off");
    typing.setAttribute("autocomplete", "off");
    typing.setAttribute("spellcheck", "false");
    typing.setAttribute("aria-hidden", "true");
    Object.assign(typing.style, { position: "fixed", left: "0px", top: "0px", width: "1px", height: "1px", opacity: "0", border: "0", padding: "0", resize: "none", overflow: "hidden", pointerEvents: "none" });
    document.body.appendChild(typing);
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

    worker.onmessage = (event) => {
      const message = event.data;
      if (message.type === "tell") {
        const tell = tells[message.tell];
        if (tell) tell(message);
      } else if (message.type === "started") {
        startedResolve();
      } else if (message.type === "failed") {
        console.error("Clarity.UI:", message.what);
        startedReject(new Error(message.what));
      } else if (message.type === "stepped" || message.type === "shot") {
        const done = waiting.get(message.id);
        waiting.delete(message.id);
        if (done) done(message.type === "shot" ? message.png : message.answer);
      }
    };

    const now = box();
    send(
      {
        type: "start",
        canvas: offscreen,
        width: now.width,
        height: now.height,
        dpr: now.dpr,
        dark: !!(dark && dark.matches),
        script: here("clarity-ui.js"),
        base: script,
        files: page(options.files || "clarity/files.json"),
        server: page(options.server || window.clarityServer || "/clarity/invoke"),
        resources: page(options.resources || "clarity/resources/"),
        settings: readKept(storageKey),
        choices: readKept(choicesKey),
      },
      [offscreen],
    );

    if (window.ResizeObserver) {
      new ResizeObserver(() => send(Object.assign({ type: "resize" }, box()))).observe(canvas);
    }
    window.addEventListener("resize", () => send(Object.assign({ type: "resize" }, box())));
    if (dark && dark.addEventListener) dark.addEventListener("change", () => send({ type: "dark", dark: dark.matches }));

    // One pointer at a time: a second finger is not a second mouse. A mouse
    // has one pointer, so a mouse event counts even when it does not say
    // isPrimary: one a page's own script makes (new PointerEvent) leaves it
    // false unless told, and leaves pointerType empty unless told.
    const primary = (event) => event.isPrimary || event.pointerType === "mouse" || event.pointerType === "";
    const pointer = (kind) => (event) => {
      if (!primary(event)) return;
      const at = place();
      send({ type: "pointer", kind, x: event.clientX - at.left, y: event.clientY - at.top, button: event.button, held: held(event) });
    };
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
        const at = place();
        send({ type: "wheel", x: event.clientX - at.left, y: event.clientY - at.top, dx: event.deltaX, dy: event.deltaY, mode: event.deltaMode, held: held(event) });
        event.preventDefault();
      },
      { passive: false },
    );

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
    canvas.tabIndex = 0;
    canvas.addEventListener("focus", focusTyping);

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
      worker,
      canvas,
      started,
      step: (step) => asked("step", { step }),
      shot: () => asked("shot", {}),
    };
  }

  function boot() {
    const canvas = document.getElementById("clarity-ui");
    if (canvas && !window.clarityUi) window.clarityUi = start(canvas);
  }

  // At once when the canvas comes before this script, so the page's own
  // scripts after it find window.clarityUi; else once the page is read.
  if (document.getElementById("clarity-ui") && document.body) boot();
  else document.addEventListener("DOMContentLoaded", boot);
})();
