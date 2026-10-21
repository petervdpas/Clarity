import { mount } from "svelte";
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
export default mount(manual ? Manual : App, { target });
