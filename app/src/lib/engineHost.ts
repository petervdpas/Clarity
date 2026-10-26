// The renderer's own engine host (DESIGN 5.12): it runs an engine beside
// the screen with the browser's WebAssembly, in a Web Worker
// (screenWorker.ts). It speaks Clarity's engine interface as every backend
// host does (conformance/engines.json): the arguments as a JSON array in
// room from clarity_alloc, the answer `{"value": ...}` or `{"error": ...}`
// at the place and length the function gives, packed in an i64. The frame
// loop is conformance/screen-engines.json. The time limit is the page's:
// a worker that does not answer in time is stopped from outside
// (screenEngine.ts). Pointer and keys go to clarity_pointer and
// clarity_key, when the engine has them, at the start of the next frame;
// so do hears, each to the engine's function of its name. An answer may
// carry `tells`, which the frame gives back with its shapes.

/** What an engine is declared with. */
export type Declared = {
  uses: string[];
  opens: boolean;
  seconds: number;
  megabytes: number;
  /** The functions code calls; an engine beside the screen has none. */
  functions?: string[];
  /** Beside the screen: it exports clarity_tick and clarity_draw. */
  screen?: boolean;
};

/** The services of Clarity, as the renderer gives them. */
export type Services = {
  /** Milliseconds since 1970-01-01 UTC. */
  clock: () => number;
  /** The seed of a call's random sequence. */
  seed: () => bigint;
  /** A line into the application's log. */
  log: (line: string) => void;
  /** The language the screen picked, "" for the texts as written. */
  language: () => string;
};

/** What a shape on a canvas is, as a draw handler's (conformance/drawing.json). */
export type Shape = { op: string; [value: string]: unknown };

/** Pointer or key on the canvas, as the engine gets it (conformance/screen-engines.json). */
export type Input =
  | { pointer: "down" | "move" | "up"; x: number; y: number }
  | { key: "down" | "up"; name: string }
  /** What a handler told the canvas (`sky.start(2)`): the engine's function of that name. */
  | { hear: string; values: unknown[] };

/** What the engine tells the window: the handler wired to it runs with its values. */
export type Tell = { tell: string; values: unknown[] };

/** Which input an engine beside the screen takes: it exports clarity_pointer, clarity_key. */
export type Hears = { pointer: boolean; keys: boolean };

/** A call that failed: the engine's error, or what went wrong running it. */
export class EngineFailed extends Error {}

const SERVICES = ["clock", "random", "log", "language"];
const INTERFACE = ["clarity_alloc", "clarity_free"];
const SCREEN_INTERFACE = ["clarity_tick", "clarity_draw"];
const PAGE = 64 * 1024;

/** The next number of splitmix64, the random sequence every host gives alike. */
function splitmix(state: { at: bigint }): bigint {
  state.at = BigInt.asUintN(64, state.at + 0x9e3779b97f4a7c15n);
  let z = state.at;
  z = BigInt.asUintN(64, (z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n);
  z = BigInt.asUintN(64, (z ^ (z >> 27n)) * 0x94d049bb133111ebn);
  return BigInt.asIntN(64, z ^ (z >> 31n));
}

type Exports = {
  memory: WebAssembly.Memory;
  clarity_alloc: (len: number) => number;
  clarity_free: (at: number, len: number) => void;
  [name: string]: unknown;
};

/** An engine loaded and made: one instance, which keeps its memory from call to call. */
export class Engine {
  readonly #name: string;
  readonly #exports: Exports;
  readonly #megabytes: number;
  readonly #random: { at: bigint };
  readonly #seed: () => bigint;
  /** The pointer and keys it takes. */
  readonly hears: Hears;
  /** What the calls since the last frame told. */
  #told: Tell[] = [];

  constructor(name: string, exports: Exports, megabytes: number, random: { at: bigint }, seed: () => bigint) {
    this.#name = name;
    this.#exports = exports;
    this.#megabytes = megabytes;
    this.#random = random;
    this.#seed = seed;
    this.hears = { pointer: typeof exports.clarity_pointer === "function", keys: typeof exports.clarity_key === "function" };
  }

  /** Calls `function` with `args`: its value, or EngineFailed. */
  call(name: string, args: unknown[]): unknown {
    const failed = (why: string) => new EngineFailed(`engine ${this.#name} failed in ${name}: ${why}`);
    const run = this.#exports[name];
    if (typeof run !== "function") throw failed("it has no such function");
    this.#random.at = BigInt.asUintN(64, this.#seed());
    let answer: { value?: unknown; error?: string; tells?: Tell[] };
    try {
      const input = new TextEncoder().encode(JSON.stringify(args));
      const at = this.#exports.clarity_alloc(input.length);
      new Uint8Array(this.#exports.memory.buffer, at, input.length).set(input);
      const packed = BigInt.asUintN(64, run(at, input.length) as bigint);
      const place = Number(packed >> 32n);
      const size = Number(packed & 0xffffffffn);
      const text = new TextDecoder().decode(new Uint8Array(this.#exports.memory.buffer, place, size));
      this.#exports.clarity_free(place, size);
      answer = JSON.parse(text);
    } catch (e) {
      if (e instanceof EngineFailed) throw e;
      throw failed(e instanceof Error ? e.message : String(e));
    }
    // The page cannot hold a module's own memory back, so it looks after the call.
    if (this.#exports.memory.buffer.byteLength > this.#megabytes * 1024 * 1024) throw failed(`it grew past its memory limit of ${this.#megabytes} MB`);
    if (answer.error !== undefined) throw failed(answer.error);
    for (const told of Array.isArray(answer.tells) ? answer.tells : []) {
      if (typeof told?.tell !== "string") throw failed("it told something without a name");
      this.#told.push({ tell: told.tell, values: Array.isArray(told.values) ? told.values : [] });
    }
    return answer.value;
  }

  /**
   * One frame: the input since the last, then the milliseconds since the
   * last frame, then the canvas's shapes. `calling` hears each call before
   * it runs, for its time limit. Input it does not take is left out.
   */
  frame(ms: number, input: Input[] = [], calling: (call: string) => void = () => {}): { shapes: Shape[]; tells: Tell[] } {
    const run = (name: string, args: unknown[]) => {
      calling(name);
      return this.call(name, args);
    };
    this.#told = [];
    for (const i of input) {
      if ("hear" in i) run(i.hear, i.values);
      else if ("pointer" in i) {
        if (this.hears.pointer) run("clarity_pointer", [i.pointer, i.x, i.y]);
      } else if (this.hears.keys) run("clarity_key", [i.key, i.name]);
    }
    run("clarity_tick", [ms]);
    const shapes = run("clarity_draw", []);
    if (!Array.isArray(shapes)) throw new EngineFailed(`engine ${this.#name} failed in clarity_draw: it gave no list of shapes`);
    const tells = this.#told;
    this.#told = [];
    return { shapes: shapes as Shape[], tells };
  }
}

/**
 * Loads `bytes` as engine `name` and makes its instance; refuses a module
 * that is no engine, imports anything but the services it uses, or lacks
 * the interface or a function it is declared with. With `on open` it runs
 * clarity_open once.
 */
export async function loadEngine(name: string, bytes: Uint8Array, declared: Declared, services: Services): Promise<Engine> {
  const refused = (why: string) => new EngineFailed(`engine ${name} ${why}`);
  if (bytes.length < 4 || bytes[0] !== 0 || bytes[1] !== 0x61 || bytes[2] !== 0x73 || bytes[3] !== 0x6d) throw refused("is no WebAssembly module");
  let module: WebAssembly.Module;
  try {
    module = await WebAssembly.compile(bytes as BufferSource);
  } catch (e) {
    throw refused(`is no WebAssembly module: ${e instanceof Error ? e.message : e}`);
  }
  for (const imported of WebAssembly.Module.imports(module)) {
    if (imported.module !== "clarity" || imported.kind !== "function" || !SERVICES.includes(imported.name) || !declared.uses.includes(imported.name))
      throw refused(`imports ${imported.module}.${imported.name}; an engine imports only the services it uses`);
  }
  const exported = WebAssembly.Module.exports(module);
  const functions = new Set(exported.filter((e) => e.kind === "function").map((e) => e.name));
  for (const needed of INTERFACE) if (!functions.has(needed)) throw refused(`lacks ${needed} of the engine interface`);
  if (!exported.some((e) => e.kind === "memory" && e.name === "memory")) throw refused("lacks the memory of the engine interface");
  for (const needed of declared.screen ? SCREEN_INTERFACE : []) if (!functions.has(needed)) throw refused(`lacks ${needed}, which an engine beside the screen needs`);
  for (const f of declared.functions ?? []) if (!functions.has(f)) throw refused(`has no function ${f}`);
  if (declared.opens && !functions.has("clarity_open")) throw refused("lacks clarity_open, which its on open needs");

  const random = { at: 0n };
  let memory: () => WebAssembly.Memory = () => {
    throw new Error("the engine is not made yet");
  };
  const bytesAt = (at: number, len: number) => new Uint8Array(memory().buffer, at, len);
  const clarity = {
    clock: () => BigInt(Math.trunc(services.clock())),
    random: () => splitmix(random),
    log: (at: number, len: number) => services.log(`engine ${name}: ${new TextDecoder().decode(bytesAt(at, len))}`),
    language: (at: number, room: number) => {
      const tag = new TextEncoder().encode(services.language());
      bytesAt(at, Math.min(room, tag.length)).set(tag.subarray(0, room));
      return tag.length;
    },
  };
  let instance: WebAssembly.Instance;
  try {
    instance = await WebAssembly.instantiate(module, { clarity });
  } catch (e) {
    throw refused(`did not start: ${e instanceof Error ? e.message : e}`);
  }
  const exports = instance.exports as Exports;
  memory = () => exports.memory;
  if (exports.memory.buffer.byteLength > declared.megabytes * 1024 * 1024 + PAGE) throw refused(`starts past its memory limit of ${declared.megabytes} MB`);
  const engine = new Engine(name, exports, declared.megabytes, random, services.seed);
  if (declared.opens) engine.call("clarity_open", []);
  return engine;
}
