// The page's side of Clarity.UI in a browser (DESIGN 7.9): the canvas the
// program draws on, at the page's pixel scale, and what the person does on it,
// sent to the program. Nothing else is drawn here.

let canvas;
let context;
let program;
let field;
// What the screen copied while the key went down, for the browser's copy event.
let copied = null;
// Option makes characters on a Mac (and an iPad), where Alt is a shortcut elsewhere.
const mac = /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);

const buttons = ["left", "middle", "right"];

function modifiers(e) {
  return (e.shiftKey ? 1 : 0) | (e.ctrlKey ? 2 : 0) | (e.altKey ? 4 : 0) | (e.metaKey ? 8 : 0);
}

function fit() {
  const scale = window.devicePixelRatio || 1;
  const box = canvas.getBoundingClientRect();
  canvas.width = Math.max(1, Math.round(box.width * scale));
  canvas.height = Math.max(1, Math.round(box.height * scale));
  program.Resized(box.width, box.height, scale);
}

export async function start(id) {
  canvas = document.getElementById(id);
  if (!canvas) {
    canvas = document.createElement("canvas");
    canvas.id = id;
    Object.assign(canvas.style, { position: "fixed", inset: "0", width: "100vw", height: "100vh" });
    document.body.append(canvas);
  }
  canvas.tabIndex = 0;
  canvas.style.outline = "none";
  canvas.style.touchAction = "none";
  context = canvas.getContext("2d");
  const runtime = await globalThis.getDotnetRuntime(0);
  program = (await runtime.getAssemblyExports("Clarity.Ui.Browser.dll")).Clarity.Ui.Browser.BrowserCanvas;
  new ResizeObserver(fit).observe(canvas);
  fit();
  const at = (e) => {
    const box = canvas.getBoundingClientRect();
    return [e.clientX - box.left, e.clientY - box.top];
  };
  // A finger is the program's to read: a tap clicks, a swipe pans, a rest then a move drags.
  const touch = (e) => e.pointerType === "touch";
  canvas.addEventListener("pointermove", (e) => {
    if (touch(e)) program.TouchMoved(...at(e), e.timeStamp);
    else program.PointerMoved(...at(e));
  });
  canvas.addEventListener("pointerleave", (e) => {
    if (!touch(e)) program.PointerLeft();
  });
  canvas.addEventListener("pointerdown", (e) => {
    field.focus({ preventScroll: true });
    canvas.setPointerCapture(e.pointerId);
    // The page gives presses: the program counts a double or triple click from their times.
    if (touch(e)) program.TouchDown(...at(e), e.timeStamp);
    else program.PointerPressed(...at(e), buttons.indexOf(buttons[e.button] ?? "left"), e.timeStamp);
    e.preventDefault();
  });
  canvas.addEventListener("pointerup", (e) => {
    if (touch(e)) program.TouchUp(...at(e), e.timeStamp);
    else program.PointerReleased(...at(e), buttons.indexOf(buttons[e.button] ?? "left"));
  });
  canvas.addEventListener("pointercancel", (e) => {
    if (touch(e)) program.TouchCancelled();
    else program.PointerCancelled();
  });
  // The page lost the focus mid-press (another window, Alt+Tab): a drag is dropped, as on the desktop.
  window.addEventListener("blur", () => program.PointerCancelled());
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());
  // The wheel as the browser says it, in its deltaMode; the program makes a notch 50, as the desktop turns one.
  canvas.addEventListener("wheel", (e) => {
    program.Wheel(...at(e), e.deltaX, e.deltaY, e.deltaMode, canvas.getBoundingClientRect().height, modifiers(e));
    e.preventDefault();
  }, { passive: false });
  // Keys and text come through a hidden text field that keeps the focus, as
  // a text box's would: what AltGr, Option, a dead key, an input method or a
  // phone's keyboard makes arrives as text, and copying and pasting are the
  // browser's own.
  field = document.createElement("textarea");
  field.setAttribute("aria-hidden", "true");
  field.autocomplete = "off";
  field.spellcheck = false;
  field.setAttribute("autocapitalize", "off");
  field.setAttribute("autocorrect", "off");
  field.inputMode = "none";
  Object.assign(field.style, {
    position: "fixed", left: "0px", top: "0px", width: "1px", height: "1em", opacity: "0",
    padding: "0", border: "0", margin: "0", resize: "none", overflow: "hidden", pointerEvents: "none",
  });
  document.body.append(field);
  const altGraph = (e) => !!(e.getModifierState && e.getModifierState("AltGraph"));
  field.addEventListener("keydown", (e) => {
    // An input method at work keeps its keys until it gives its text.
    if (e.isComposing || e.keyCode === 229) return;
    copied = null;
    // The screen decides what keys do; a character goes on into the field, a key the screen does not keep goes to the browser.
    if (program.KeyPressed(e.key, modifiers(e), altGraph(e), mac)) e.preventDefault();
  });
  field.addEventListener("keyup", (e) => {
    if (e.isComposing) return;
    program.KeyReleased(e.key, modifiers(e), altGraph(e));
  });
  field.addEventListener("input", (e) => {
    // A word an input method is still making comes once it is done.
    if (e.isComposing || e.inputType === "insertCompositionText") return;
    if (e.data) program.TextTyped(e.data);
    field.value = "";
  });
  field.addEventListener("compositionend", (e) => {
    if (e.data) program.TextTyped(e.data);
    field.value = "";
  });
  // What the screen copied goes on the browser's clipboard; a paste brings the browser's in.
  const copy = (e) => {
    if (copied === null) return;
    e.clipboardData.setData("text/plain", copied);
    e.preventDefault();
  };
  field.addEventListener("copy", copy);
  field.addEventListener("cut", copy);
  field.addEventListener("paste", (e) => {
    e.preventDefault();
    program.Pasted(e.clipboardData ? e.clipboardData.getData("text/plain") : "");
  });
  // Time goes on for the windows: an info message goes when its time is up.
  setInterval(() => program.Tick(), 250);
  // The page's light or dark: the theme follows it when it changes.
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", (e) => program.SystemDarkChanged(e.matches));
  field.focus({ preventScroll: true });
}

// What the screen copied: the browser's clipboard, in the copy event of the
// key that copied it, or straight away where the browser lets the page.
export function copyText(text) {
  copied = text;
  try {
    navigator.clipboard?.writeText(text).catch(() => {});
  } catch {
    // No clipboard for the page: the copy event still takes it.
  }
}

// The pointer's shape over the canvas, as CSS names it.
export function setCursor(cursor) {
  if (canvas) canvas.style.cursor = cursor;
}

// The text field goes where the focus is, so an input method shows its words
// there; a phone shows its keyboard only for a part that takes text.
export function textFieldAt(x, y, height, takesText) {
  if (!field) return;
  const box = canvas.getBoundingClientRect();
  field.style.left = `${box.left + x}px`;
  field.style.top = `${box.top + y}px`;
  field.style.height = `${Math.max(height, 16)}px`;
  field.inputMode = takesText ? "text" : "none";
}

// Where the page came from: the server whose sessions a served program's windows are.
export function origin() {
  return location.origin;
}

export function prefersDark() {
  return matchMedia("(prefers-color-scheme: dark)").matches;
}

// A web page opens in a new tab, apart from this page.
export function openTab(address) {
  window.open(address, "_blank", "noopener,noreferrer");
}

// Mail and phone numbers go to the system's program for them, through the page.
export function openHere(address) {
  window.location.href = address;
}

export function setTitle(title) {
  document.title = title;
}

// The next frame: what the program waits to do (a reply of the server
// followed) runs then, and the screen is drawn. A hidden tab draws no
// frames, so a timeout does it there (as slowly as the browser lets a
// hidden tab's timers run); whichever comes first does it.
let frameAsked = 0;
export function requestFrame() {
  const asked = ++frameAsked;
  const frame = () => {
    if (asked !== frameAsked) return;
    frameAsked++;
    program.Frame();
  };
  requestAnimationFrame(frame);
  setTimeout(frame, document.hidden ? 0 : 100);
}

export function present(pixels, width, height) {
  const copy = new Uint8ClampedArray(pixels.slice());
  context.putImageData(new ImageData(copy, width, height), 0, 0);
}

// An element's wake (a trend moving on, a tooltip showing): one timer, set anew when the next wake moves.
let wake;
export function wakeIn(ms) {
  clearTimeout(wake);
  wake = ms >= 0 ? setTimeout(() => program.WakeNow(), ms) : undefined;
}

// The parts' clock: a frame's time, asked for while a timer or a sound runs.
// A hidden tab draws no frames, so a timeout ticks there too (as slowly as
// the browser lets a hidden tab's timers run); whichever comes first ticks.
let tickAsked = 0;
export function requestTick() {
  const asked = ++tickAsked;
  const tick = (time) => {
    if (asked !== tickAsked) return;
    tickAsked++;
    program.TimeTick(time);
  };
  requestAnimationFrame(tick);
  setTimeout(() => tick(performance.now()), document.hidden ? 0 : 100);
}

// A file the person gets: the browser's download of it.
export function download(name, bytes) {
  const url = URL.createObjectURL(new Blob([bytes.slice()]));
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 60000);
}

// A file shown in a tab of its own, as its kind given; a kind the page could run is never given.
export function show(type, bytes) {
  const url = URL.createObjectURL(new Blob([bytes.slice()], { type }));
  window.open(url, "_blank", "noopener,noreferrer");
  setTimeout(() => URL.revokeObjectURL(url), 60000);
}

// What the person keeps in the browser: nothing when its storage is closed to the page.
export function read(key) {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

export function write(key, value) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Storage closed to the page (a private window): the choice lasts this visit.
  }
}

// A file to open: the page's file input, its kinds as given (".png,.jpg").
// The chosen file's name, its bytes kept for pickedBytes; null for none.
let picked = new Uint8Array();
export function openFile(accept) {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    if (accept) input.accept = accept;
    input.style.display = "none";
    document.body.append(input);
    const done = (value) => {
      input.remove();
      resolve(value);
    };
    input.addEventListener("cancel", () => done(null));
    input.addEventListener("change", async () => {
      const file = input.files && input.files[0];
      if (!file) return done(null);
      picked = new Uint8Array(await file.arrayBuffer());
      done(file.name);
    });
    input.click();
  });
}

export function pickedBytes() {
  const bytes = picked;
  picked = new Uint8Array();
  return Array.from(bytes);
}

// Sounds through the page's audio (Web Audio): WAV, MP3 and FLAC, as the
// browser decodes them. A browser lets a page sound only after the person
// did something on it, so the audio resumes at their first press or key.
let audio;
let lastSound = 0;
const sounds = new Map();

function audioContext() {
  if (!audio) {
    audio = new AudioContext();
    const resume = () => audio.resume();
    window.addEventListener("pointerdown", resume, { capture: true });
    window.addEventListener("keydown", resume, { capture: true });
  }
  return audio;
}

export function play(bytes, volume, repeat) {
  const id = ++lastSound;
  const data = bytes.slice().buffer;
  const sound = { stopped: false, source: null };
  sounds.set(id, sound);
  let context;
  try {
    context = audioContext();
  } catch {
    // No audio in this browser: the program runs on, silent.
    sounds.delete(id);
    return id;
  }
  context.decodeAudioData(data).then(
    (buffer) => {
      if (sound.stopped) return;
      const gain = context.createGain();
      gain.gain.value = volume;
      gain.connect(context.destination);
      const source = context.createBufferSource();
      source.buffer = buffer;
      source.loop = repeat;
      source.connect(gain);
      source.onended = () => sounds.delete(id);
      sound.source = source;
      source.start();
    },
    () => sounds.delete(id),
  );
  return id;
}

export function stop(id) {
  const sound = sounds.get(id);
  if (!sound) return;
  sound.stopped = true;
  sounds.delete(id);
  try {
    sound.source?.stop();
  } catch {
    // Already ended.
  }
}
