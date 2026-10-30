// Errors no code caught, in a click handler or a promise nobody waited for,
// are shown to the person instead of being lost in the console: the IDE in
// its error bar, a running application as an error message.

const reason = (error: unknown) => (error instanceof Error ? error.message : String(error));

/** Shows every error nothing caught with `show`; returns what stops it. */
export function catchUncaught(show: (text: string) => void, target: Window = window): () => void {
  const failed = (event: ErrorEvent) => show(reason(event.error ?? event.message));
  const rejected = (event: PromiseRejectionEvent) => show(reason(event.reason));
  target.addEventListener("error", failed);
  target.addEventListener("unhandledrejection", rejected);
  return () => {
    target.removeEventListener("error", failed);
    target.removeEventListener("unhandledrejection", rejected);
  };
}
