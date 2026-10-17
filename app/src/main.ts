import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";
import { followSystemScheme } from "./lib/scheme.svelte";

followSystemScheme();

const target = document.getElementById("app");
if (!target) throw new Error("missing #app element");

export default mount(App, { target });
