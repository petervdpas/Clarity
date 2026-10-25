// The frame loop of an engine beside the screen, on the page (DESIGN 5.12):
// one run per canvas, from when its window opens until it closes. The
// engine runs in a Web Worker of its own (screenWorker.ts), so an engine
// that loops never freezes the window. The page asks for frames at the
// screen's pace and skips one while the engine is busy with the last, so
// frames never pile up; a worker that does not answer within the engine's
// time limit is stopped from outside, and the canvas shows why.

import type { ScreenEngine } from "./generated/ScreenEngine";
import type { Shape } from "./engineHost";

/** What the page tells the worker. */
export type ToWorker =
  | { kind: "start"; engine: ScreenEngine; language: string }
  | { kind: "frame"; ms: number; language: string };

/** What the worker answers. */
export type FromWorker =
  | { kind: "ready" }
  | { kind: "ticked" }
  | { kind: "drawn"; shapes: Shape[] }
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
};

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
        this.#next();
        break;
      case "ticked":
        this.#untime();
        this.#busy = "clarity_draw";
        this.#time("clarity_draw");
        break;
      case "drawn":
        this.#untime();
        this.#busy = null;
        this.#options.onDraw(message.shapes);
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
      this.#busy = "clarity_tick";
      this.#time("clarity_tick");
      this.#send({ kind: "frame", ms, language: this.#options.language() });
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
