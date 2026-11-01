// The markup of each window the application has (DESIGN 7.8): in the page
// itself, where the C and C++ build put it (`<script type="text/xml"
// data-window="Main">`), or beside it in `windows/Main.xml`. A window without
// markup is drawn from the model.

import type { Preview } from "../generated/Preview.ts";
import type { Widget } from "../generated/Widget.ts";
import { windowFromMarkup } from "../screen/windowMarkup.ts";

/** The parts of each window that has markup, by the window's name. */
export async function windowsOf(model: Preview): Promise<Record<string, Widget[]>> {
  const pages = model.project.files.flatMap((f) => f.pages).filter((p) => p.kind !== "view");
  const found = await Promise.all(pages.map(async (page) => [page, inPage(page.name.name) ?? (await beside(page.name.name))] as const));
  const windows: Record<string, Widget[]> = {};
  for (const [page, markup] of found) {
    if (markup === null) continue;
    try {
      windows[page.name.name] = windowFromMarkup(markup, page.span.start);
    } catch (problem) {
      // The window shows as the model has it, and why its markup was not read is said.
      console.error(`windows/${page.name.name}.xml: ${problem instanceof Error ? problem.message : String(problem)}`);
    }
  }
  return windows;
}

function inPage(name: string): string | null {
  for (const script of document.querySelectorAll('script[type="text/xml"][data-window]')) {
    if (script.getAttribute("data-window") === name) return script.textContent;
  }
  return null;
}

async function beside(name: string): Promise<string | null> {
  try {
    const answer = await fetch(`./windows/${encodeURIComponent(name)}.xml`);
    return answer.ok ? await answer.text() : null;
  } catch {
    return null;
  }
}
