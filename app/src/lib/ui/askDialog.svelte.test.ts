// A question a handler asked, in the application's own dialog (DESIGN
// 5.14), run through the real AskDialog: what it shows, and what it answers.

import { flushSync, mount, unmount } from "svelte";
import { expect, test } from "vitest";
import type { Preview } from "../generated/Preview";
import type { JsonValue } from "../generated/serde_json/JsonValue";
import type { Ask } from "../asks";
import AskDialog from "./AskDialog.svelte";

const model = {
  project: {
    translations: { en: { deleteIt: "Delete it?", delete: "Delete", keep: "Keep", ok: "OK", cancel: "Cancel", lendTo: "Lend it to", howMany: "How many?" } },
    locale: { language: "en", currency: null },
    resource_keys: {},
    files: [],
  },
} as unknown as Preview;

/** Shows `ask` and gives what it answered: a value, null for a yes, or "cancelled". */
async function asked(ask: Ask, act: (dialog: HTMLElement) => void): Promise<{ answer: JsonValue | "cancelled" | "nothing"; text: string }> {
  let answer: JsonValue | "cancelled" | "nothing" = "nothing";
  const target = document.createElement("div");
  document.body.append(target);
  const dialog = mount(AskDialog, {
    target,
    props: {
      model,
      ask,
      options: async () => [
        { id: 3, text: "Ada" },
        { id: 5, text: "Grace" },
        { id: 9, text: "Linus" },
      ],
      onanswer: (value: JsonValue | null) => (answer = value),
      oncancel: () => (answer = "cancelled"),
    },
  });
  flushSync();
  await new Promise((done) => setTimeout(done, 0));
  flushSync();
  const text = target.textContent ?? "";
  act(target);
  flushSync();
  unmount(dialog);
  target.remove();
  return { answer, text };
}

const click = (target: HTMLElement, selector: string) => target.querySelector<HTMLElement>(selector)!.click();

test("a question with yes and no says its own words; yes answers, no cancels", async () => {
  const ask: Ask = { question: { key: "deleteIt" }, form: { kind: "confirm", yes: { key: "delete" }, no: { key: "keep" } }, handler: "gone", values: [] };
  const yes = await asked(ask, (d) => click(d, "button.ok"));
  expect(yes.text).toContain("Delete it?");
  expect(yes.text).toContain("Keep");
  expect(yes.answer).toBe(null);
  const no = await asked(ask, (d) => click(d, "button:not(.ok)"));
  expect(no.answer).toBe("cancelled");
});

test("a number starts at its default and answers what is typed", async () => {
  const ask: Ask = { question: { key: "howMany" }, form: { kind: "value", type: "integer", of: null, only: null, default: 2 }, handler: "setCopies", values: [] };
  const typed = await asked(ask, (d) => {
    const input = d.querySelector<HTMLInputElement>("input")!;
    expect(input.value).toBe("2");
    input.value = "5";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.dispatchEvent(new Event("change", { bubbles: true }));
    flushSync();
    click(d, "button.ok");
  });
  expect(typed.answer).toBe(5);
});

/** Enter in the box answers, as OK does: a person types and presses Enter. */
test("Enter in a number or a text answers what is typed", async () => {
  const ask: Ask = { question: { key: "howMany" }, form: { kind: "value", type: "integer", of: null, only: null, default: 2 }, handler: "setCopies", values: [] };
  const typed = await asked(ask, (d) => {
    const input = d.querySelector<HTMLInputElement>("input")!;
    input.value = "7";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
    flushSync();
  });
  expect(typed.answer).toBe(7);
});

test("a record is chosen among those `from` gave, the first chosen to start with", async () => {
  const ask: Ask = { question: { key: "lendTo" }, form: { kind: "value", type: "record", of: "Friend", only: [3, 5], default: null }, handler: "lend", values: [{ id: 7 }] };
  const chosen = await asked(ask, (d) => {
    const labels = [...d.querySelectorAll("option")].map((o) => o.textContent);
    expect(labels).toEqual(["Ada", "Grace"]);
    click(d, "button.ok");
  });
  expect(chosen.answer).toBe(3);
});
