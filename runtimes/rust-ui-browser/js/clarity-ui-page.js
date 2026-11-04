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
//   access     false: no accessible mirror (by itself it follows `input`: the IDE's designer has none)
// Screen readers read a hidden DOM tree beside the canvas (the mirror, DESIGN
// 7.9 A1): one element per node of the library's accessibility tree with its
// ARIA role, name, states and value, placed where the node shows, kept up to
// date after each frame, the DOM's focus on the node with the screen's focus;
// a click, typed text or a value set on it goes back as the tree's actions.
// The person's keys work through it as through the hidden textarea.
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

  // The roles the mirror writes as ARIA's, where Clarity's tree says otherwise (DESIGN 7.9, A1).
  const ARIA_ROLES = { window: "application", label: null, textarea: null, password: null, date: null, color: null, canvas: "img", textbox: null, spinbutton: "spinbutton" };
  // The roles whose number the mirror says as a range.
  const RANGES = ["slider", "progressbar", "spinbutton"];
  // The inputs the person types into, natively, as the hidden textarea.
  const TYPED = ["textbox", "textarea", "password", "spinbutton"];

  // What element a node is: an input for what is typed, else a plain element.
  function tagOf(node) {
    if (node.role === "textarea") return "textarea";
    if (TYPED.includes(node.role) || ["date", "color"].includes(node.role)) return "input";
    if (node.role === "combobox" && node.actions.includes("setValue")) return "input";
    return "div";
  }

  // The hidden but accessible tree beside the canvas (DESIGN 7.9, A1 phase
  // 2): one element per node of the library's tree, with its ARIA role,
  // name, states and value, placed where the node shows (for magnifiers and
  // touch exploration), updated in place from the host's `access` tells.
  // `act(node, action, value)` sends what assistive technology does back
  // as the tree's actions. Messages and alerts are said through a live region.
  function mirrorOf(canvas, act) {
    const nodes = new Map();
    let rootId = null;
    let focus = null;
    let composing = false;
    const root = document.createElement("div");
    root.setAttribute("data-clarity-mirror", "");
    Object.assign(root.style, { position: "fixed", left: "0px", top: "0px", width: "0px", height: "0px", opacity: "0", pointerEvents: "none", overflow: "hidden", transformOrigin: "0 0", margin: "0", padding: "0", border: "0", color: "transparent", background: "transparent" });
    const live = (role) => {
      const region = document.createElement("div");
      region.setAttribute("role", role);
      region.setAttribute("aria-live", role === "alert" ? "assertive" : "polite");
      region.setAttribute("aria-atomic", "true");
      region.setAttribute("data-clarity-live", role);
      Object.assign(region.style, { position: "fixed", left: "0px", top: "0px", width: "1px", height: "1px", overflow: "hidden", clipPath: "inset(50%)", whiteSpace: "nowrap", pointerEvents: "none" });
      document.body.appendChild(region);
      return region;
    };
    canvas.insertAdjacentElement("afterend", root);
    const polite = live("status");
    const alert = live("alert");

    function say(region, text) {
      // The same text again is said again: emptied first, written a moment later.
      region.textContent = "";
      setTimeout(() => (region.textContent = text), 50);
    }

    function make(node) {
      const tag = tagOf(node);
      const element = node.id === rootId ? root : document.createElement(tag);
      element.setAttribute("data-clarity-node", node.id);
      if (element !== root) {
        Object.assign(element.style, { position: "absolute", margin: "0", padding: "0", border: "0", boxSizing: "border-box", overflow: "hidden" });
        if (tag === "input" || tag === "textarea") {
          element.setAttribute("autocomplete", "off");
          element.setAttribute("spellcheck", "false");
        }
      }
      return { id: node.id, tag: element === root ? "root" : tag, element, text: null, attrs: new Set(), node: null, parent: null, placed: "" };
    }

    // The attributes a node's element has, by its role, name, states and value.
    function attributesOf(node, parentRole) {
      const a = {};
      let role = node.role in ARIA_ROLES ? ARIA_ROLES[node.role] : node.role;
      if (node.role === "cell" && parentRole === "treegrid") role = "gridcell";
      if (node.role === "combobox") role = "combobox";
      if (role) a.role = role;
      const textual = node.role === "label" || node.role === "status";
      if (!textual && node.name) a["aria-label"] = node.name;
      const s = node.states || {};
      if (s.disabled) a["aria-disabled"] = "true";
      if (s.readonly) a["aria-readonly"] = "true";
      if (s.required) a["aria-required"] = "true";
      if (s.modal && (node.role === "dialog" || node.role === "alertdialog")) a["aria-modal"] = "true";
      if (s.checked != null) a[node.role === "button" ? "aria-pressed" : "aria-checked"] = String(s.checked);
      if (s.selected != null) a["aria-selected"] = String(s.selected);
      if (s.expanded != null) a["aria-expanded"] = String(s.expanded);
      const description = s.invalid != null ? s.invalid : node.description;
      if (s.invalid != null) a["aria-invalid"] = "true";
      if (description) a["aria-description"] = description;
      if (node.level != null && ["heading", "row"].includes(node.role)) a["aria-level"] = String(node.level);
      if (RANGES.includes(node.role)) {
        const n = node.numeric || {};
        if (n.value != null) a["aria-valuenow"] = String(n.value);
        if (n.min != null) a["aria-valuemin"] = String(n.min);
        if (n.max != null) a["aria-valuemax"] = String(n.max);
        if (node.value != null && node.role !== "spinbutton") a["aria-valuetext"] = node.value;
      }
      if (node.role === "textarea") a["aria-multiline"] = "true";
      // Only the live region speaks: the mirror's own statuses stay quiet.
      if (node.role === "status" || node.role === "log") a["aria-live"] = "off";
      if (node.actions.includes("focus") || node.id === focus || node.id === rootId) a.tabindex = "-1";
      return a;
    }

    function render(record) {
      const node = record.node;
      const element = record.element;
      // A cell in a tree table is a grid's cell.
      let table = record.parent;
      while (table && table.node && !["table", "treegrid", "grid"].includes(table.node.role)) table = table.parent;
      const wanted = attributesOf(node, table && table.node ? table.node.role : null);
      for (const name of record.attrs) if (!(name in wanted)) element.removeAttribute(name);
      for (const name in wanted) if (element.getAttribute(name) !== wanted[name]) element.setAttribute(name, wanted[name]);
      record.attrs = new Set(Object.keys(wanted));
      if (record.tag === "input" || record.tag === "textarea") {
        const type = node.role === "password" ? "password" : node.role === "date" ? "date" : node.role === "color" ? "color" : "text";
        if (record.tag === "input" && element.type !== type) element.type = type;
        element.readOnly = !!(node.states && (node.states.readonly || node.states.disabled));
        const value = node.value == null ? "" : node.value;
        if (!composing && element.value !== value) {
          element.value = value;
          // A date or color the browser's input does not take is said as text.
          if (record.tag === "input" && type !== "text" && type !== "password" && value && element.value !== value) {
            element.type = "text";
            element.value = value;
          }
          if (document.activeElement === element && element.setSelectionRange && element.type !== "date" && element.type !== "color") {
            try {
              element.setSelectionRange(value.length, value.length);
            } catch (why) {
              // An input without a caret.
            }
          }
        }
      } else {
        // A label's and a status's text, a select-only combobox's value: the element's own text, before its children.
        const own = node.role === "label" || node.role === "status" ? node.name : node.role === "combobox" ? node.value || "" : "";
        if (own) {
          if (!record.text) {
            record.text = document.createTextNode("");
            element.insertBefore(record.text, element.firstChild);
          }
          if (record.text.data !== own) record.text.data = own;
        } else if (record.text) {
          record.text.remove();
          record.text = null;
        }
      }
    }

    // The node's children in its element, in order, after its own text.
    function arrange(record) {
      if (record.tag === "input" || record.tag === "textarea") return;
      const box = record.element;
      let next = record.text ? record.text.nextSibling : box.firstChild;
      for (const id of record.node.children) {
        const child = nodes.get(id);
        if (!child) continue;
        child.parent = record;
        if (child.element === next) {
          next = next.nextSibling;
        } else {
          box.insertBefore(child.element, next);
        }
      }
    }

    // Each element where its node shows: the root over the canvas (scaled as
    // the canvas is shown), the others within the element they are in.
    function place() {
      const at = canvas.getBoundingClientRect();
      const width = canvas.clientWidth || at.width || 1;
      const height = canvas.clientHeight || at.height || 1;
      Object.assign(root.style, { left: at.left + "px", top: at.top + "px", width: width + "px", height: height + "px", transform: at.width && Math.abs(at.width - width) > 0.5 ? "scale(" + at.width / width + "," + at.height / height + ")" : "" });
      for (const record of nodes.values()) {
        if (record.element === root || !record.node) continue;
        const [x, y, w, h] = record.node.at;
        const from = record.parent && record.parent.node && record.parent.element !== root ? record.parent.node.at : [0, 0];
        const placed = x - from[0] + "," + (y - from[1]) + "," + w + "," + h;
        if (placed === record.placed) continue;
        record.placed = placed;
        Object.assign(record.element.style, { left: x - from[0] + "px", top: y - from[1] + "px", width: Math.max(w, 1) + "px", height: Math.max(h, 1) + "px" });
      }
    }

    function ours(element) {
      return !!element && (element === canvas || element.hasAttribute("data-clarity-typing") || root.contains(element));
    }

    function focusElement() {
      const record = nodes.get(focus) || nodes.get(rootId);
      return record ? record.element : root;
    }

    // Takes the host's update: whole or only what changed (clarity_ui::access::mirror::update).
    function apply(update) {
      const hadFocus = ours(document.activeElement);
      if (update.whole) {
        for (const record of nodes.values()) if (record.element !== root) record.element.remove();
        nodes.clear();
        for (const name of [...root.getAttributeNames()]) if (name !== "data-clarity-mirror" && name !== "style") root.removeAttribute(name);
        while (root.firstChild) root.firstChild.remove();
      }
      rootId = update.root;
      focus = update.focus;
      const changed = [];
      for (const node of update.nodes) {
        let record = nodes.get(node.id);
        if (record && record.tag !== "root" && record.tag !== tagOf(node)) {
          // Another kind of element now: made anew in the same place.
          const again = make(node);
          record.element.replaceWith(again.element);
          again.parent = record.parent;
          record = again;
        }
        if (!record) record = make(node);
        record.node = node;
        record.placed = "";
        nodes.set(node.id, record);
        changed.push(record);
      }
      for (const record of changed) arrange(record);
      for (const record of changed) render(record);
      for (const id of update.gone || []) {
        const record = nodes.get(id);
        if (!record) continue;
        nodes.delete(id);
        if (record.element !== root) record.element.remove();
      }
      for (const record of nodes.values()) if (record.parent && record.parent.element !== root && !nodes.has(record.parent.id)) record.placed = "";
      place();
      const wanted = focusElement();
      if (hadFocus && document.activeElement !== wanted) wanted.focus({ preventScroll: true });
      for (const said of update.say || []) say(said.alert ? alert : polite, said.text);
    }

    function recordOf(target) {
      const element = target && target.closest ? target.closest("[data-clarity-node]") : null;
      return element && root.contains(element) ? nodes.get(element.getAttribute("data-clarity-node")) : null;
    }

    // Assistive technology's own doings, as the tree's actions.
    root.addEventListener("click", (event) => {
      event.preventDefault();
      const record = recordOf(event.target);
      if (!record || !record.node) return;
      const node = record.node;
      if (node.actions.includes("click")) act(node.id, "click");
      else if (node.actions.includes("expand") || node.actions.includes("collapse")) act(node.id, node.states && node.states.expanded ? "collapse" : "expand");
      else if (node.actions.includes("focus")) act(node.id, "focus");
    });
    root.addEventListener("focusin", (event) => {
      const record = recordOf(event.target);
      if (record && record.node && record.id !== focus && record.node.actions.includes("focus")) {
        focus = record.id;
        act(record.id, "focus");
      }
    });

    return {
      root,
      apply,
      place,
      ours,
      focusElement,
      recordOf,
      // Whether the person types into this element natively (an input of text), as into the hidden textarea.
      typedIn: (element) => {
        const record = recordOf(element);
        return !!record && record.node && (record.tag === "textarea" || (record.tag === "input" && (element.type === "text" || element.type === "password")) );
      },
      // A key that is an action of the focused node: Up and Down (and Left and Right on a slider) step a number, Right and Left open and close a row.
      stepKey: (event) => {
        const record = recordOf(event.target);
        if (!record || !record.node || event.ctrlKey || event.metaKey || event.altKey) return false;
        const node = record.node;
        const slider = node.role === "slider";
        const up = event.key === "ArrowUp" || (slider && event.key === "ArrowRight");
        const down = event.key === "ArrowDown" || (slider && event.key === "ArrowLeft");
        if ((slider || node.role === "spinbutton") && (up || down)) {
          const action = up ? "increment" : "decrement";
          if (!node.actions.includes(action)) return false;
          act(node.id, action);
          return true;
        }
        if (node.role === "row" && node.states && node.states.expanded != null && (event.key === "ArrowRight" || event.key === "ArrowLeft")) {
          const action = event.key === "ArrowRight" ? "expand" : "collapse";
          if (!node.actions.includes(action)) return false;
          act(node.id, action);
          return true;
        }
        return false;
      },
      composing: (on) => (composing = on),
      ready: () => rootId != null && nodes.has(rootId),
      stop: () => {
        root.remove();
        polite.remove();
        alert.remove();
      },
    };
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
    typing.setAttribute("data-clarity-typing", "");
    Object.assign(typing.style, { position: "fixed", left: "0px", top: "0px", width: "1px", height: "1px", opacity: "0", border: "0", padding: "0", resize: "none", overflow: "hidden", pointerEvents: "none" });
    if (takesInput) document.body.appendChild(typing);
    const accessible = options.access === undefined ? takesInput : options.access !== false;
    const mirror = accessible ? mirrorOf(canvas, (node, action, value) => send({ type: "act", node, action, value })) : null;
    // Keys go where the DOM's focus is ours: the mirror's focused node once it has one, else the hidden textarea.
    const keyTarget = () => (mirror && mirror.ready() ? mirror.focusElement() : typing);
    const focusTyping = () => keyTarget().focus({ preventScroll: true });
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
          else if (document.activeElement === typing || (mirror && mirror.ours(document.activeElement))) document.activeElement.blur();
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
      access: (t) => mirror && mirror.apply(t.update),
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

    const resized = () => {
      send(Object.assign({ type: "resize" }, box()));
      if (mirror) mirror.place();
    };
    const scrolled = () => mirror && mirror.place();
    if (mirror) window.addEventListener("scroll", scrolled, true);
    if (mirror) started.then(() => send({ type: "access", on: true }), () => {});
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
      if (writes && (altGraph || option)) return { send: true, held: held(event) & 1, prevent: false, writes: true };
      const shortcut = event.ctrlKey || event.metaKey || event.altKey;
      return { send: true, held: held(event), prevent: shortcut || !writes, writes: writes && !shortcut };
    }

    // The hidden textarea's keys, and the mirror's (`mirrored`): its inputs of
    // text are typed into as the textarea is; elsewhere in it a key that
    // writes sends its text itself, and a value set otherwise (a screen
    // reader's, a pick) goes back as setValue.
    let pasteKey = null;
    function listenKeys(target, mirrored) {
      const typedIn = (event) => !mirrored || mirror.typedIn(event.target);
      target.addEventListener("keydown", (event) => {
        const shortcut = event.ctrlKey || event.metaKey;
        if (shortcut && !event.altKey && (event.key === "v" || event.key === "V")) {
          pasteKey = { key: event.key, code: event.code, held: held(event) };
          return;
        }
        if (mirrored && mirror.stepKey(event)) {
          event.preventDefault();
          return;
        }
        const down = keyDown(event);
        if (down.send) send({ type: "key", down: true, key: event.key, code: event.code, held: down.held });
        if (down.send && down.writes && !typedIn(event)) {
          send({ type: "text", text: event.key });
          event.preventDefault();
          return;
        }
        if (down.prevent) event.preventDefault();
      });
      target.addEventListener("keyup", (event) => {
        const up = keyDown(event);
        if (up.send) send({ type: "key", down: false, key: event.key, code: event.code, held: up.held });
      });
      // A phone's keyboard sends no keys (229, Unidentified) but what it does
      // to the text: deleting and new lines are the keys, text is typed.
      const editKeys = { deleteContentBackward: ["Backspace", "Backspace"], deleteContentForward: ["Delete", "Delete"], insertLineBreak: ["Enter", "Enter"], insertParagraph: ["Enter", "Enter"] };
      target.addEventListener("beforeinput", (event) => {
        const key = editKeys[event.inputType];
        if (!key) return;
        event.preventDefault();
        send({ type: "key", down: true, key: key[0], code: key[1], held: 0 });
        send({ type: "key", down: false, key: key[0], code: key[1], held: 0 });
      });
      target.addEventListener("input", (event) => {
        if (event.isComposing) return;
        if (!mirrored) {
          if (typing.value) send({ type: "text", text: typing.value });
          typing.value = "";
          return;
        }
        const record = mirror.recordOf(event.target);
        if (!record || !record.node) return;
        // Typed by the person: text, as the textarea's; set any other way: the whole value.
        if (typedIn(event) && (event.inputType === "insertText" || event.inputType === "insertCompositionText")) {
          if (event.data) send({ type: "text", text: event.data });
        } else if (record.node.actions.includes("setValue")) {
          send({ type: "act", node: record.id, action: "setValue", value: event.target.value });
        }
      });
      target.addEventListener("compositionstart", () => mirrored && mirror.composing(true));
      target.addEventListener("compositionupdate", (event) => send({ type: "compose", text: event.data || "", start: null, end: null }));
      target.addEventListener("compositionend", (event) => {
        if (mirrored) mirror.composing(false);
        send({ type: "compose", text: "", start: null, end: null });
        if (event.data) send({ type: "text", text: event.data });
        if (!mirrored) typing.value = "";
      });
      target.addEventListener("paste", (event) => {
        event.preventDefault();
        const text = event.clipboardData ? event.clipboardData.getData("text/plain") : "";
        send({ type: "paste", text });
        const key = pasteKey || { key: "v", code: "KeyV", held: 2 };
        pasteKey = null;
        send({ type: "key", down: true, key: key.key, code: key.code, held: key.held });
        send({ type: "key", down: false, key: key.key, code: key.code, held: key.held });
      });
    }
    listenKeys(typing, false);
    if (mirror && takesInput) listenKeys(mirror.root, true);
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
      // The accessible mirror's root element, or null without one.
      mirror: mirror ? mirror.root : null,
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
        if (mirror) {
          mirror.stop();
          window.removeEventListener("scroll", scrolled, true);
        }
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
