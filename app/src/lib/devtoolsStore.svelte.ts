// DevTools' builds for the whole IDE (a Clarity run from its repository):
// every build of a piece or a release check as it goes, so a tab shows it
// while it builds wherever Clarity settings was left; and an agent's ask to
// press a Build (Clarity's MCP, build_piece), allowed as the person chose
// for agents: never, after asking, or without asking; and an agent's ask to
// remove a build folder of the Disk tab (remove_folder), which cannot be
// undone, so it is always asked first unless agents may never (src-tauri/src/devtools).
// Each ask opens Clarity settings at the DevTools tab it works in.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { BuildAsk } from "@clarity/ui/generated/BuildAsk.ts";
import type { JobView } from "@clarity/ui/generated/JobView.ts";
import { agent } from "./agentStore.svelte";
import { openDevTools } from "./appSettings.svelte";
import { askedTab } from "./devtools";
import { dialogs } from "./dialogStore.svelte";

/** What an agent hears when the person keeps agents from building. */
export const NEVER_BUILDS = "The person does not let agents build (Clarity settings > Agent): ask them to press Build in Clarity settings > DevTools.";
/** What an agent hears when the person said no. */
export const SAID_NO = "The person did not allow this build.";
/** What an agent hears when the person keeps agents from removing. */
export const NEVER_REMOVES = "The person does not let agents act in DevTools (Clarity settings > Agent): ask them to remove the folder in Clarity settings > DevTools > Disk.";
/** What an agent hears when the person said no to a removal. */
export const SAID_NO_REMOVE = "The person did not allow removing this folder.";

/** The question before an agent's build or removal. */
export function askText(ask: BuildAsk): string {
  if (ask.action === "remove") return `${ask.by} asks to remove ${ask.name} Remove it?`;
  return `${ask.by} asks to press Build of ${ask.name} in DevTools. Build it?`;
}

/** Whether a build is waiting or building. */
export function busy(job: JobView | undefined): boolean {
  return job?.state === "waiting" || job?.state === "running";
}

class DevToolsStore {
  /** Every build so far, by its key. */
  jobs = $state<Record<string, JobView>>({});
  #listening = false;

  /** Listens for builds and agents' asks; once. */
  init(): void {
    if (this.#listening) return;
    this.#listening = true;
    void listen<JobView>("devtools-job", (e) => {
      this.jobs = { ...this.jobs, [e.payload.key]: e.payload };
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
    if (agent.runs === "ask" && !(await dialogs.confirm({ title: "DevTools", text: askText(ask), action: "Build" }))) {
      await invoke("devtools_refuse", { id: ask.id, why: SAID_NO });
      return;
    }
    try {
      await invoke("devtools_build", { key: ask.key, by: ask.by, ask: ask.id });
    } catch {
      // The backend answered the agent with why.
    }
  }
}

export const devtools = new DevToolsStore();
