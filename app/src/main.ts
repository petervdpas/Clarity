import { mount } from "svelte";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./app.css";
import "@clarity/ui/screen/render.css";
import { startIdeKeys } from "./lib/keys.svelte";
import { startIdeTheme } from "./lib/ideTheme.svelte";
import { pageMenuOff } from "@clarity/ui/frame/pageMenu.ts";

// Clarity in the theme chosen in its app settings.
startIdeTheme();
startIdeKeys();
pageMenuOff();

const target = document.getElementById("app");
if (!target) throw new Error("missing #app element");

// The manual opens in a window of its own, on the same page with ?window=manual.
// Each window loads only its own page: the manual does not wait for the IDE's code.
const manual = new URLSearchParams(location.search).get("window") === "manual";
const shown = (manual ? import("./Manual.svelte") : import("./App.svelte")).then((page) => {
  const mounted = mount(page.default, { target });
  // The window starts hidden on a dark ground (tauri.conf.json, manual.rs) and
  // shows once the page has drawn itself in its theme, so it never flashes white.
  requestAnimationFrame(() =>
    requestAnimationFrame(() => {
      if (manual) {
        getCurrentWindow().show().catch(() => {});
        return;
      }
      // Clarity's window shows and its splash screen goes (src-tauri/src/splash.rs),
      // unless a rebuild runs at the start, which shows them when it ends (dev_rebuild.rs).
      invoke("main_drawn").catch(() => getCurrentWindow().show().catch(() => {}));
    }),
  );
  return mounted;
});

export default shown;
