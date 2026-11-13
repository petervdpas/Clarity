// Rebuild in development (src-tauri/src/dev_rebuild.rs): only an IDE run
// from its repository has it. The pieces to rebuild, ticked by the person
// or by an agent in target/dev-rebuild.json, and Restart, which hides the
// IDE, builds them with the splash showing each line and shows the IDE
// again. When a build fails the IDE shows again with why, here and in
// Clarity settings > Rebuild.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { RebuildFailed } from "@clarity/ui/generated/RebuildFailed.ts";
import type { RebuildView } from "@clarity/ui/generated/RebuildView.ts";
import { openAppSettings } from "./appSettings.svelte";

class DevRebuildStore {
  /** The pieces and their ticks; null in an installed Clarity, which has no such section. */
  view = $state<RebuildView | null>(null);
  /** Why the last restart's build failed, until the next restart. */
  failed = $state<RebuildFailed | null>(null);
  /** Why a tick or a restart was refused. */
  error = $state<string | null>(null);
  /** A restart was asked and has not ended. */
  restarting = $state(false);

  constructor() {
    listen<RebuildFailed>("dev-rebuild-failed", (e) => {
      this.restarting = false;
      this.failed = e.payload;
      void this.load();
      openAppSettings("rebuild");
    }).catch(() => {});
  }

  async load(): Promise<void> {
    this.error = null;
    try {
      this.view = await invoke<RebuildView | null>("dev_rebuild");
    } catch (err) {
      this.error = String(err);
    }
  }

  async tick(key: string, ticked: boolean): Promise<void> {
    this.error = null;
    try {
      this.view = await invoke<RebuildView>("set_dev_rebuild", { key, ticked });
    } catch (err) {
      this.error = String(err);
    }
  }

  async restart(): Promise<void> {
    this.error = null;
    this.failed = null;
    try {
      this.restarting = true;
      await invoke("dev_restart");
    } catch (err) {
      this.restarting = false;
      this.error = String(err);
    }
  }
}

export const devRebuild = new DevRebuildStore();
