// Whether Clarity settings show, and at which section: opened from the
// toolbar, or from a part of Clarity that points at one (the project's
// Build tools row at Environment, an agent's ask at its DevTools tab).

import { checkTools } from "./tools.svelte";

export type AppSettingsSection = "display" | "keys" | "folders" | "environment" | "agent" | "logging" | "system" | "devTools";

export const appSettings = $state<{ open: boolean; section: AppSettingsSection; devToolsTab: string | null }>({ open: false, section: "display", devToolsTab: null });

/** Shows Clarity settings, at `section` when given. */
export function openAppSettings(section?: AppSettingsSection) {
  if (section) appSettings.section = section;
  appSettings.open = true;
}

/** Shows Clarity settings at DevTools with the tab `tab` (a piece's key, Disk or Release planning), as an agent's ask does. */
export function openDevTools(tab: string) {
  appSettings.devToolsTab = tab;
  openAppSettings("devTools");
}

/** Closes Clarity settings; the build tools are looked at again, since the Environment may have changed. */
export function closeAppSettings() {
  appSettings.open = false;
  checkTools(true);
}
