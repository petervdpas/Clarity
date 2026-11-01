// An error nothing caught, thrown or rejected, reaches the person.

import { expect, test } from "vitest";
import { catchUncaught } from "../uncaught";

test("errors thrown and promises rejected that nothing caught are shown, until stopped", () => {
  const shown: string[] = [];
  const stop = catchUncaught((text) => shown.push(text));
  window.dispatchEvent(new ErrorEvent("error", { error: new Error("a click went wrong"), message: "a click went wrong" }));
  const rejected = new Event("unhandledrejection") as PromiseRejectionEvent;
  Object.defineProperty(rejected, "reason", { value: "the backend did not answer" });
  window.dispatchEvent(rejected);
  expect(shown).toEqual(["a click went wrong", "the backend did not answer"]);
  stop();
  // The test catches its own last error, so it shows nothing and escapes nowhere.
  const own = (e: Event) => e.preventDefault();
  window.addEventListener("error", own);
  window.dispatchEvent(new ErrorEvent("error", { error: new Error("later"), cancelable: true }));
  window.removeEventListener("error", own);
  expect(shown).toHaveLength(2);
});

test("the browser's own notice that a resize waited is no error and shows nothing", () => {
  const shown: string[] = [];
  const stop = catchUncaught((text) => shown.push(text));
  window.dispatchEvent(new ErrorEvent("error", { message: "ResizeObserver loop completed with undelivered notifications." }));
  window.dispatchEvent(new ErrorEvent("error", { message: "ResizeObserver loop limit exceeded" }));
  expect(shown).toEqual([]);
  stop();
});
