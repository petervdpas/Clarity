// A day typed key by key into a date input is taken once, when the input is
// left or Enter is pressed, as typed text is; the dates in between (0002,
// 0020, 0202 on the way to 2024) are no changes. A day picked from the
// calendar is taken at once.

import { flushSync, mount, unmount } from "svelte";
import { expect, test } from "vitest";
import DateInput from "./DateInput.svelte";
import DateTimeInput from "./DateTimeInput.svelte";

function shown(component: typeof DateInput | typeof DateTimeInput) {
  const target = document.createElement("div");
  document.body.append(target);
  const changes: (string | null)[] = [];
  const part = mount(component as typeof DateInput, { target, props: { value: null, label: "Released", onchange: (next: string | null) => changes.push(next) } });
  flushSync();
  const input = target.querySelector<HTMLInputElement>("input[type=date]")!;
  const typeDay = (day: string) => {
    input.dispatchEvent(new KeyboardEvent("keydown", { key: "2", bubbles: true }));
    input.value = day;
    input.dispatchEvent(new Event("change", { bubbles: true }));
  };
  return { input, changes, typeDay, done: () => (unmount(part), target.remove()) };
}

for (const [name, component] of [["date", DateInput], ["date and time", DateTimeInput]] as const) {
  test(`a typed day in a ${name} input is taken when the input is left`, () => {
    const d = shown(component);
    d.input.focus();
    d.typeDay("0002-02-14");
    d.typeDay("0202-02-14");
    d.typeDay("2024-02-14");
    expect(d.changes).toEqual([]);
    d.input.dispatchEvent(new FocusEvent("blur"));
    expect(d.changes.length).toBe(1);
    expect(d.changes[0]).toContain("2024-02-14");
    d.done();
  });

  test(`a typed day in a ${name} input is taken on Enter`, () => {
    const d = shown(component);
    d.input.focus();
    d.typeDay("2024-02-14");
    d.input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    expect(d.changes.length).toBe(1);
    d.done();
  });

  test(`Ctrl+Z while typing a day in a ${name} input puts back the day it had`, () => {
    const d = shown(component);
    d.input.focus();
    d.typeDay("2024-02-14");
    d.input.dispatchEvent(new KeyboardEvent("keydown", { key: "z", ctrlKey: true, bubbles: true }));
    expect(d.input.value).toBe("");
    d.input.dispatchEvent(new FocusEvent("blur"));
    expect(d.changes).toEqual([]);
    d.done();
  });

  test(`a day picked from the calendar of a ${name} input is taken at once`, () => {
    const d = shown(component);
    d.input.value = "2024-03-01";
    d.input.dispatchEvent(new Event("change", { bubbles: true }));
    expect(d.changes.length).toBe(1);
    expect(d.changes[0]).toContain("2024-03-01");
    d.done();
  });
}
