// What a view tells the window it shows in (DESIGN 5.33): the view's page
// declares each tell, and the content's view line that shows it wires a tell
// to a handler of the window. Pure, so it is tested without a browser.

import type { PartMessage } from "@clarity/ui/generated/PartMessage.ts";
import { paramsText } from "@clarity/ui/screen/customParts.ts";

/** The name of the tell a message declares: its first word (`saved` of `saved(name: text)`), as the add_tell tool reads it. */
export function tellName(message: string): string {
  return /^\s*([A-Za-z0-9_]*)/.exec(message)?.[1] ?? "";
}

/** A declared tell as it is written: `saved(name: text)`, or `closed` without values. */
export function tellText(tell: PartMessage): string {
  return tell.params.length === 0 ? tell.name.name : `${tell.name.name}(${paramsText(tell.params)})`;
}

/** The handler a Wire button names for a tell of a view: `recipeEditorSaved` for `saved` of `RecipeEditor`. */
export function tellHandlerName(view: string, tell: string): string {
  return view.charAt(0).toLowerCase() + view.slice(1) + tell.charAt(0).toUpperCase() + tell.slice(1);
}
