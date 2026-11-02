// The page's side of Clarity.UI in a browser (DESIGN 7.9): the canvas the
// program draws on, at the page's pixel scale, and what the person does on it,
// sent to the program. Nothing else is drawn here.

let canvas;
let context;
let program;

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
  canvas.addEventListener("pointermove", (e) => program.PointerMoved(...at(e)));
  canvas.addEventListener("pointerleave", () => program.PointerLeft());
  canvas.addEventListener("pointerdown", (e) => {
    canvas.focus();
    canvas.setPointerCapture(e.pointerId);
    program.PointerPressed(...at(e), buttons.indexOf(buttons[e.button] ?? "left"));
  });
  canvas.addEventListener("pointerup", (e) => program.PointerReleased(...at(e), buttons.indexOf(buttons[e.button] ?? "left")));
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());
  // The wheel in the page's pixels: a line is 40, a page the canvas's height, as the desktop turns a notch.
  canvas.addEventListener("wheel", (e) => {
    const unit = e.deltaMode === 1 ? 40 : e.deltaMode === 2 ? canvas.getBoundingClientRect().height : 1;
    program.Wheel(...at(e), e.deltaX * unit, e.deltaY * unit, modifiers(e));
    e.preventDefault();
  }, { passive: false });
  canvas.addEventListener("keyup", (e) => {
    program.KeyReleased(e.key, modifiers(e));
    e.preventDefault();
  });
  canvas.addEventListener("keydown", (e) => {
    program.KeyPressed(e.key, modifiers(e));
    if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) program.TextTyped(e.key);
    // The screen decides what keys do, Tab and Space included.
    e.preventDefault();
  });
  // Time goes on for the windows: an info message goes when its time is up.
  setInterval(() => program.Tick(), 250);
  // The page's light or dark: the theme follows it when it changes.
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", (e) => program.SystemDarkChanged(e.matches));
  canvas.focus();
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

export function requestFrame() {
  requestAnimationFrame(() => program.Frame());
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
export function requestTick() {
  requestAnimationFrame((time) => program.TimeTick(time));
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
