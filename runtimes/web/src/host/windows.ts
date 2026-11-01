// The markup of each window and app the application has (DESIGN 7.8): in
// the page itself, where the C and C++ build put it (`<script type="text/xml"
// data-window="Main">`, `data-app="Shop"`), or beside it in
// `windows/Main.cxaml` and `apps/Shop.cxaml`. The frames it gives (an app's, a
// window's own) take the model's place; a window without markup is drawn
// from the model.

import type { Preview } from "../generated/Preview.ts";
import type { Widget } from "../generated/Widget.ts";
import { readApp, readWindow } from "../screen/windowMarkup.ts";

/**
 * The parts of each window that has markup, by the window's name; the
 * frames the markup gives, an app's and each window's own, are put in the
 * model's place, so the screen draws them as it draws the model's.
 */
export async function windowsOf(model: Preview): Promise<{ windows: Record<string, Widget[]>; apps: string[] }> {
  const pages = model.project.files.flatMap((f) => f.pages).filter((p) => p.kind !== "view");
  const apps = model.project.files.flatMap((f) => f.apps);
  const [windowMarkup, appMarkup] = await Promise.all([
    Promise.all(pages.map(async (page) => [page, inPage("data-window", page.name.name) ?? (await beside("windows", page.name.name))] as const)),
    Promise.all(apps.map(async (app) => [app, inPage("data-app", app.name.name) ?? (await beside("apps", app.name.name))] as const)),
  ]);
  const windows: Record<string, Widget[]> = {};
  for (const [page, markup] of windowMarkup) {
    if (markup === null) continue;
    try {
      const read = readWindow(markup, page.span.start);
      windows[page.name.name] = read.widgets;
      page.frame = read.frame;
    } catch (problem) {
      // The window shows as the model has it, and why its markup was not read is said.
      console.error(`windows/${page.name.name}.cxaml: ${problem instanceof Error ? problem.message : String(problem)}`);
    }
  }
  const framed: string[] = [];
  for (const [app, markup] of appMarkup) {
    if (markup === null) continue;
    try {
      app.frame = readApp(markup, app.span.start);
      framed.push(app.name.name);
    } catch (problem) {
      console.error(`apps/${app.name.name}.cxaml: ${problem instanceof Error ? problem.message : String(problem)}`);
    }
  }
  return { windows, apps: framed };
}

/** The markup the page itself holds for a window or app (`data-window`, `data-app`) by its name. */
function inPage(attribute: "data-window" | "data-app", name: string): string | null {
  for (const script of document.querySelectorAll(`script[type="text/xml"][${attribute}]`)) {
    if (script.getAttribute(attribute) === name) return script.textContent;
  }
  return null;
}

/** The markup beside the page, in `windows/` or `apps/`; none where there is no such file. */
async function beside(folder: "windows" | "apps", name: string): Promise<string | null> {
  try {
    const answer = await fetch(`./${folder}/${encodeURIComponent(name)}.cxaml`);
    return answer.ok ? await answer.text() : null;
  } catch {
    return null;
  }
}
