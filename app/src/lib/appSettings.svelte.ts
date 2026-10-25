// Whether Clarity settings show, and at which section: opened from the
// toolbar, or from a part of Clarity that points at one (the project's
// Build tools row at Environment).

import { checkTools } from "./tools.svelte";

export type AppSettingsSection = "display" | "keys" | "folders" | "environment" | "agent" | "system";

export const appSettings = $state<{ open: boolean; section: AppSettingsSection }>({ open: false, section: "display" });

/** Shows Clarity settings, at `section` when given. */
export function openAppSettings(section?: AppSettingsSection) {
  if (section) appSettings.section = section;
  appSettings.open = true;
}

/** Closes Clarity settings; the build tools are looked at again, since the Environment may have changed. */
export function closeAppSettings() {
  appSettings.open = false;
  checkTools(true);
}
