// The frame loop of an engine beside the screen, on the page (DESIGN 5.12):
// one run per canvas, from when its window opens until it closes. The
// engine runs in a Web Worker of its own (screenWorker.ts), so an engine
// that loops never freezes the window. The page asks for frames at the
// screen's pace and skips one while the engine is busy with the last, so
// frames never pile up; a worker that does not answer within the engine's
// time limit is stopped from outside, and the canvas shows why. Pointer,
// keys and what handlers tell the engine (hears) wait for the next frame and
// go with it; what the engine tells comes back with the frame's shapes
// (conformance/screen-engines.json).

import type { ScreenEngine } from "./generated/ScreenEngine";
import type { Hears, Input, Shape, Tell } from "./engineHost";

/** What the page tells the worker. */
export type ToWorker =
  | { kind: "start"; engine: ScreenEngine; language: string }
  | { kind: "frame"; ms: number; input: Input[]; language: string };

/** What the worker answers. */
export type FromWorker =
  | { kind: "ready"; hears: Hears }
  | { kind: "calling"; call: string }
  | { kind: "drawn"; shapes: Shape[]; tells: Tell[] }
  | { kind: "failed"; message: string }
  | { kind: "log"; line: string };

/** The part of a Worker the run uses. */
export type WorkerLike = {
  postMessage(message: ToWorker): void;
  terminate(): void;
  onmessage: ((event: { data: FromWorker }) => void) | null;
  /** The worker's own failure, such as a script it cannot run. */
  onerror: ((event: { message: string }) => void) | null;
};

export type RunOptions = {
  worker: () => WorkerLike;
  /** The screen's frames: requestAnimationFrame, or a test's. */
  frames: { request(f: (now: number) => void): number; cancel(id: number): void };
  timers: { set(f: () => void, ms: number): number; clear(id: number): void };
  /** The language the screen shows now. */
  language: () => string;
  onDraw: (shapes: Shape[]) => void;
  onFail: (message: string) => void;
  onLog: (line: string) => void;
  /** The engine is made, and takes this input. */
  onReady?: (hears: Hears) => void;
  /** The engine told the window something: its handler runs. */
  onTell?: (tell: string, values: unknown[]) => void;
};

/** Adds `input` to what waits for the next frame: moves in a row keep only the last. */
export function queue(waiting: Input[], input: Input) {
  const last = waiting.at(-1);
  if (last && "pointer" in last && last.pointer === "move" && "pointer" in input && input.pointer === "move") waiting[waiting.length - 1] = input;
  else waiting.push(input);
}

/** The named keys an engine gets, as the page names them too. */
const NAMED_KEYS = new Set([
  "Enter", "Escape", "Backspace", "Delete", "ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End", "PageUp", "PageDown",
  "Shift", "Control", "Alt", "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
]);

/** Clarity's name of a key from the page's (KeyboardEvent.key); null for one an engine does not get. */
export function keyName(key: string): string | null {
  if (key === " ") return "Space";
  if (/^[A-Za-z0-9]$/.test(key)) return key.toLowerCase();
  return NAMED_KEYS.has(key) ? key : null;
}

/** The keys that are down, so each goes down once however long it repeats. */
export class HeldKeys {
  #held: string[] = [];

  /** Whether `name` went down now, rather than repeating. */
  down(name: string): boolean {
    if (this.#held.includes(name)) return false;
    this.#held.push(name);
    return true;
  }

  /** Whether `name` was down. */
  up(name: string): boolean {
    const was = this.#held.includes(name);
    this.#held = this.#held.filter((k) => k !== name);
    return was;
  }

  /** Lets go of every key: the canvas lost the keyboard. */
  all(): string[] {
    const held = this.#held;
    this.#held = [];
    return held;
  }
}

/** The words of a time limit that ran out, as every host says it. */
export function ranLonger(engine: string, call: string, seconds: number): string {
  return `engine ${engine} failed in ${call}: it ran longer than ${seconds} second${seconds === 1 ? "" : "s"}`;
}

export class ScreenRun {
  readonly #engine: ScreenEngine;
  readonly #options: RunOptions;
  #worker: WorkerLike | null = null;
  #frame: number | null = null;
  #limit: number | null = null;
  /** The call the worker is in, while it is busy. */
  #busy: string | null = null;
  #last: number | null = null;
  #ended = false;
  /** The input it takes, once made. */
  #hears: Hears = { pointer: false, keys: false };
  #waiting: Input[] = [];

  constructor(engine: ScreenEngine, options: RunOptions) {
    this.#engine = engine;
    this.#options = options;
  }

  start() {
    const worker = this.#options.worker();
    this.#worker = worker;
    worker.onmessage = (event) => this.#heard(event.data);
    worker.onerror = (event) => {
      if (this.#ended) return;
      const call = this.#busy ?? "clarity_tick";
      this.#end();
      this.#options.onFail(`engine ${this.#engine.name} failed in ${call}: ${event.message}`);
    };
    this.#busy = "clarity_open";
    if (this.#engine.opens) this.#time("clarity_open");
    // A plain copy: the engine may come from reactive state, whose proxy cannot cross to a worker.
    this.#send({ kind: "start", engine: JSON.parse(JSON.stringify(this.#engine)), language: this.#options.language() });
  }

  /** Sends to the worker; what it cannot take ends the run, saying why. */
  #send(message: ToWorker) {
    try {
      this.#worker?.postMessage(message);
    } catch (e) {
      const call = this.#busy ?? "clarity_tick";
      this.#end();
      this.#options.onFail(`engine ${this.#engine.name} failed in ${call}: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  /** The pointer on the canvas, in its units. */
  pointer(kind: "down" | "move" | "up", x: number, y: number) {
    if (!this.#ended && this.#hears.pointer) queue(this.#waiting, { pointer: kind, x, y });
  }

  /** A key on the canvas, by Clarity's name. */
  key(kind: "down" | "up", name: string) {
    if (!this.#ended && this.#hears.keys) queue(this.#waiting, { key: kind, name });
  }

  /** What a handler told the engine; kept, even before it is ready, for the next frame. */
  hear(name: string, values: unknown[]) {
    if (!this.#ended) queue(this.#waiting, { hear: name, values });
  }

  /** Ends the run: its window closed. */
  stop() {
    this.#end();
  }

  #heard(message: FromWorker) {
    if (this.#ended) return;
    switch (message.kind) {
      case "ready":
        this.#untime();
        this.#busy = null;
        this.#hears = message.hears;
        this.#options.onReady?.(message.hears);
        this.#next();
        break;
      case "calling":
        this.#untime();
        this.#busy = message.call;
        this.#time(message.call);
        break;
      case "drawn":
        this.#untime();
        this.#busy = null;
        this.#options.onDraw(message.shapes);
        for (const told of message.tells) this.#options.onTell?.(told.tell, told.values);
        break;
      case "failed":
        this.#end();
        this.#options.onFail(message.message);
        break;
      case "log":
        this.#options.onLog(message.line);
        break;
    }
  }

  #next() {
    this.#frame = this.#options.frames.request((now) => this.#tick(now));
  }

  #tick(now: number) {
    this.#frame = null;
    if (this.#ended) return;
    if (this.#busy === null) {
      const ms = this.#last === null ? 0 : now - this.#last;
      this.#last = now;
      const input = this.#waiting;
      this.#waiting = [];
      const first = input[0] === undefined ? "clarity_tick" : "hear" in input[0] ? input[0].hear : "pointer" in input[0] ? "clarity_pointer" : "clarity_key";
      this.#busy = first;
      this.#time(first);
      this.#send({ kind: "frame", ms, input, language: this.#options.language() });
    }
    if (!this.#ended) this.#next();
  }

  #time(call: string) {
    this.#limit = this.#options.timers.set(() => {
      this.#limit = null;
      this.#end();
      this.#options.onFail(ranLonger(this.#engine.name, call, this.#engine.seconds));
    }, this.#engine.seconds * 1000);
  }

  #untime() {
    if (this.#limit !== null) this.#options.timers.clear(this.#limit);
    this.#limit = null;
  }

  #end() {
    this.#ended = true;
    this.#untime();
    if (this.#frame !== null) this.#options.frames.cancel(this.#frame);
    this.#frame = null;
    this.#worker?.terminate();
    this.#worker = null;
  }
}

type Listener = (hear: string, values: unknown[]) => void;

/**
 * Where hears go: the canvas of the window they were told on, by its
 * number and the part's name. A hear told while the canvas does not show
 * (an `on open` handler's reply comes before it does) waits for it.
 */
export class CanvasHears {
  #listening = new Map<string, Listener>();
  #waiting = new Map<string, { hear: string; values: unknown[] }[]>();

  hear(instance: number, part: string, hear: string, values: unknown[]) {
    const key = `${instance}\n${part}`;
    const listener = this.#listening.get(key);
    if (listener) listener(hear, values);
    else this.#waiting.set(key, [...(this.#waiting.get(key) ?? []), { hear, values }]);
  }

  /** The canvas shows: it hears what waited, then what comes, until the returned stop. */
  listen(instance: number, part: string, listener: Listener): () => void {
    const key = `${instance}\n${part}`;
    this.#listening.set(key, listener);
    const waited = this.#waiting.get(key) ?? [];
    this.#waiting.delete(key);
    for (const w of waited) listener(w.hear, w.values);
    return () => {
      if (this.#listening.get(key) === listener) this.#listening.delete(key);
    };
  }

  /** The window closed: what its canvases would hear goes. */
  closed(instance: number) {
    for (const map of [this.#listening, this.#waiting]) for (const key of [...map.keys()]) if (key.startsWith(`${instance}\n`)) map.delete(key);
  }
}
