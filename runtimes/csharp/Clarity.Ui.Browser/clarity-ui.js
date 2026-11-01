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
  canvas.addEventListener("keydown", (e) => {
    program.KeyPressed(e.key, modifiers(e));
    if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) program.TextTyped(e.key);
    // The screen decides what keys do, Tab and Space included.
    e.preventDefault();
  });
  canvas.focus();
}

export function requestFrame() {
  requestAnimationFrame(() => program.Frame());
}

export function present(pixels, width, height) {
  const copy = new Uint8ClampedArray(pixels.slice());
  context.putImageData(new ImageData(copy, width, height), 0, 0);
}
