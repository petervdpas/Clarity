// What a view tells its window, as the page's Tells list and a content's view line show and write it.
import { test } from "node:test";
import assert from "node:assert/strict";
import { tellHandlerName, tellName, tellText } from "./viewTells.ts";
import type { PartMessage } from "@clarity/ui/generated/PartMessage.ts";

const span = { start: 0, end: 1, line: 1, col: 1 };
const ident = (name: string) => ({ name, span });

test("a tell is named by the message's first word", () => {
  assert.equal(tellName("saved(name: text, count: integer)"), "saved");
  assert.equal(tellName("  closed "), "closed");
  assert.equal(tellName("(x: text)"), "");
});

test("a declared tell is shown as it is written", () => {
  const saved: PartMessage = { name: ident("saved"), params: [{ name: ident("name"), ty: { kind: { kind: "named", name: "text" }, span } }], span } as PartMessage;
  assert.equal(tellText(saved), "saved(name: text)");
  assert.equal(tellText({ name: ident("closed"), params: [], span } as PartMessage), "closed");
});

test("a wired handler is named after the view and the tell", () => {
  assert.equal(tellHandlerName("RecipeEditor", "saved"), "recipeEditorSaved");
});
