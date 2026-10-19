import { mount } from "svelte";
import App from "./App.svelte";
import Manual from "./Manual.svelte";
import "./app.css";
import "./render.css";
import { followSystemScheme } from "./lib/scheme.svelte";

followSystemScheme();

const target = document.getElementById("app");
if (!target) throw new Error("missing #app element");

// The manual opens in a window of its own, on the same page with ?window=manual.
const manual = new URLSearchParams(location.search).get("window") === "manual";
export default mount(manual ? Manual : App, { target });
