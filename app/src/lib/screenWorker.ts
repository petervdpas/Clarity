// The Web Worker an engine beside the screen runs in (DESIGN 5.12): it
// loads the engine, then answers every frame with the canvas's shapes. It
// draws nothing itself. The page stops it from outside when a call runs
// past its limit (screenEngine.ts).

import { EngineFailed, loadEngine, type Engine } from "./engineHost";
import type { FromWorker, ToWorker } from "./screenEngine";

const scope = self as unknown as { onmessage: ((e: MessageEvent<ToWorker>) => void) | null; postMessage(m: FromWorker): void };
let engine: Engine | null = null;
let language = "";

const send = (message: FromWorker) => scope.postMessage(message);
const why = (e: unknown) => (e instanceof EngineFailed || e instanceof Error ? e.message : String(e));

/** A fresh seed for every call, as every host gives one. */
function seed(): bigint {
  const words = crypto.getRandomValues(new Uint32Array(2));
  return (BigInt(words[0]) << 32n) | BigInt(words[1]);
}

scope.onmessage = async (event) => {
  const message = event.data;
  language = message.language;
  try {
    if (message.kind === "start") {
      const { engine: declared } = message;
      const bytes = Uint8Array.from(atob(declared.wasm), (c) => c.charCodeAt(0));
      const services = { clock: () => Date.now(), seed, log: (line: string) => send({ kind: "log", line }), language: () => language };
      engine = await loadEngine(declared.name, bytes, { ...declared, screen: true }, services);
      send({ kind: "ready" });
    } else if (engine) {
      // Its time limit is per call: the page hears when the tick is done.
      const shapes = engine.frame(message.ms, () => send({ kind: "ticked" }));
      send({ kind: "drawn", shapes });
    }
  } catch (e) {
    engine = null;
    send({ kind: "failed", message: why(e) });
  }
};
