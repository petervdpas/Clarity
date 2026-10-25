import { mount } from "svelte";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App.svelte";
import Manual from "./Manual.svelte";
import "./app.css";
import "./render.css";
import { startIdeTheme } from "./lib/ideTheme.svelte";
import { pageMenuOff } from "./lib/pageMenu";

// Clarity in the theme chosen in its app settings.
startIdeTheme();
pageMenuOff();

const target = document.getElementById("app");
if (!target) throw new Error("missing #app element");

// The manual opens in a window of its own, on the same page with ?window=manual.
const manual = new URLSearchParams(location.search).get("window") === "manual";
const shown = mount(manual ? Manual : App, { target });

// The window starts hidden on a dark ground (tauri.conf.json, manual.rs) and
// shows once the page has drawn itself in its theme, so it never flashes white.
requestAnimationFrame(() =>
  requestAnimationFrame(() => {
    getCurrentWindow().show().catch(() => {});
  }),
);

export default shown;
