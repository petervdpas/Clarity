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
  window.dispatchEvent(new ErrorEvent("error", { error: new Error("later") }));
  expect(shown).toHaveLength(2);
});
