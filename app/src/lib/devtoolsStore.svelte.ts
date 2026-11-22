// DevTools' builds for the whole IDE (a Clarity run from its repository):
// every build of a piece or a release check as it goes, so a tab shows it
// while it builds wherever Clarity settings was left; and an agent's ask to
// press a Build (Clarity's MCP, build_piece), allowed as the person chose
// for agents: never, after asking, or without asking; and an agent's ask to
// remove a build folder of the Disk tab (remove_folder), which cannot be
// undone, so it is always asked first unless agents may never (src-tauri/src/devtools);
// and an agent's ask to pick a piece's own build or the release (swap_piece), allowed as a build is.
// Each ask opens Clarity settings at the DevTools tab it works in.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { BuildAsk } from "@clarity/ui/generated/BuildAsk.ts";
import type { JobView } from "@clarity/ui/generated/JobView.ts";
import { agent } from "./agentStore.svelte";
import { openDevTools } from "./appSettings.svelte";
import { browserFiles } from "./browserFilesStore.svelte";
import { askedTab, reloadsBrowserFiles } from "./devtools";
import { dialogs } from "./dialogStore.svelte";

/** What an agent hears when the person keeps agents from building. */
export const NEVER_BUILDS = "The person does not let agents build (Clarity settings > Agent): ask them to press Build in Clarity settings > DevTools.";
/** What an agent hears when the person said no. */
export const SAID_NO = "The person did not allow this build.";
/** What an agent hears when the person keeps agents from removing. */
export const NEVER_REMOVES = "The person does not let agents act in DevTools (Clarity settings > Agent): ask them to remove the folder in Clarity settings > DevTools > Disk.";
/** What an agent hears when the person said no to a removal. */
export const SAID_NO_REMOVE = "The person did not allow removing this folder.";

/** What an agent hears when the person said no to a swap. */
export const SAID_NO_SWAP = "The person did not allow this swap.";

/** Whether an ask picks a piece's own build or the release. */
export function swaps(ask: BuildAsk): boolean {
  return ask.action === "useOwn" || ask.action === "useRelease";
}

/** The question before an agent's build, swap or removal. */
export function askText(ask: BuildAsk): string {
  if (ask.action === "remove") return `${ask.by} asks to remove ${ask.name} Remove it?`;
  if (ask.action === "useOwn") return `${ask.by} asks to use your own build of ${ask.name} for the programs Clarity builds. Use it?`;
  if (ask.action === "useRelease") return `${ask.by} asks to use the release of ${ask.name} for the programs Clarity builds again. Use it?`;
  if (ask.action === "buildDebug") return `${ask.by} asks to press Build debug of ${ask.name} in DevTools. Build it?`;
  return `${ask.by} asks to press Build of ${ask.name} in DevTools. Build it?`;
}

/** Whether a build is waiting or building. */
export function busy(job: JobView | undefined): boolean {
  return job?.state === "waiting" || job?.state === "running";
}

class DevToolsStore {
  /** Every build so far, by its key. */
  jobs = $state<Record<string, JobView>>({});
  /** The piece an agent swapped last, and how often, so an open DevTools reads how it stands again. */
  swapped = $state<{ key: string; times: number }>({ key: "", times: 0 });
  #listening = false;

  /** Listens for builds and agents' asks; once. */
  init(): void {
    if (this.#listening) return;
    this.#listening = true;
    void listen<JobView>("devtools-job", (e) => {
      const before = this.jobs[e.payload.key];
      this.jobs = { ...this.jobs, [e.payload.key]: e.payload };
      // The browser files built again: every canvas loads them again, whether DevTools is open or not.
      if (reloadsBrowserFiles({ job: e.payload, before })) browserFiles.reload();
    });
    void listen<BuildAsk>("devtools-asked", (e) => void this.#asked(e.payload));
  }

  /** The builds the backend knows, as a view brings them. */
  know(jobs: JobView[]): void {
    const all = { ...this.jobs };
    for (const job of jobs) all[job.key] = job;
    this.jobs = all;
  }

  async #asked(ask: BuildAsk): Promise<void> {
    // The person sees what the agent asks where it happens: its piece's tab, or Disk.
    openDevTools(askedTab(ask));
    if (ask.action === "remove") {
      // Removing cannot be undone: asked first even when agents may run things.
      if (agent.runs === "never") {
        await invoke("devtools_refuse", { id: ask.id, why: NEVER_REMOVES });
        return;
      }
      if (!(await dialogs.confirm({ title: "DevTools", text: askText(ask), action: "Remove", danger: true }))) {
        await invoke("devtools_refuse", { id: ask.id, why: SAID_NO_REMOVE });
        return;
      }
      try {
        await invoke("disk_remove", { key: ask.key, ask: ask.id });
      } catch {
        // The backend answered the agent with why.
      }
      return;
    }
    if (agent.runs === "never") {
      await invoke("devtools_refuse", { id: ask.id, why: NEVER_BUILDS });
      return;
    }
    if (swaps(ask)) {
      if (agent.runs === "ask" && !(await dialogs.confirm({ title: "DevTools", text: askText(ask), action: "Use it" }))) {
        await invoke("devtools_refuse", { id: ask.id, why: SAID_NO_SWAP });
        return;
      }
      try {
        await invoke("devtools_swap", { key: ask.key, mine: ask.action === "useOwn", ask: ask.id });
        this.swapped = { key: ask.key, times: this.swapped.times + 1 };
        // The design and preview draw with the browser files: swapped, every canvas loads them again.
        if (reloadsBrowserFiles({ swapped: ask.key })) browserFiles.reload();
      } catch {
        // The backend answered the agent with why.
      }
      return;
    }
    if (agent.runs === "ask" && !(await dialogs.confirm({ title: "DevTools", text: askText(ask), action: "Build" }))) {
      await invoke("devtools_refuse", { id: ask.id, why: SAID_NO });
      return;
    }
    try {
      await invoke("devtools_build", { key: ask.key, debug: ask.action === "buildDebug", by: ask.by, ask: ask.id });
    } catch {
      // The backend answered the agent with why.
    }
  }
}

export const devtools = new DevToolsStore();
