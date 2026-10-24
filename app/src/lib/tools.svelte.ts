// The build tools of every stack on this machine, looked for once and again
// on request ("Check again" after installing something).

import { invoke } from "@tauri-apps/api/core";
import type { StackTools } from "./generated/StackTools";

export const stackTools = $state<{ all: StackTools[] | null; checking: boolean }>({ all: null, checking: false });

export function checkTools(again = false) {
  if (stackTools.checking || (stackTools.all && !again)) return;
  stackTools.checking = true;
  invoke<StackTools[]>("stack_tools")
    .then((all) => (stackTools.all = all))
    .catch(() => (stackTools.all = null))
    .finally(() => (stackTools.checking = false));
}

/** The manual's page on installing the build tools (clarity_gen::tools::SETUP_TOPIC). */
export const SETUP_TOPIC = "setup.tools";

/** Opens the manual at the heading of the setup page that tells how to install something. */
export function openSetupGuide(anchor: string | null) {
  invoke("open_manual", { topic: SETUP_TOPIC, anchor }).catch(() => {});
}
