// The load the backend reads once a second (system.rs): the last reading,
// for the status bar's meters, and while the System page is open, the
// minute of full readings its graphs show. Nothing more is kept.

import { listen } from "@tauri-apps/api/event";
import { invoke } from "./backend";
import type { LoadReading } from "./generated/LoadReading";
import { kept } from "./systemLoad";

export const systemLoad = $state<{ last: LoadReading | null; history: LoadReading[] }>({ last: null, history: [] });

let watching = false;

/** Starts listening; once, when Clarity starts. Gives the way to stop. */
export function listenToLoad(): Promise<() => void> {
  return listen<LoadReading>("system-load", (e) => {
    systemLoad.last = e.payload;
    if (watching && e.payload.full) systemLoad.history = kept(systemLoad.history, e.payload);
  });
}

/** The System page opened or closed: the backend reads in full only while it is open. */
export function watchSystem(on: boolean) {
  watching = on;
  systemLoad.history = [];
  invoke("watch_system", { watched: on }).catch(() => {});
}
