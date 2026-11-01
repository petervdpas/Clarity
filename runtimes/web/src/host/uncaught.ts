// Errors no code caught, in a click handler or a promise nobody waited for,
// are shown to the person instead of being lost in the console: the IDE in
// its error bar, a running application as an error message.

const reason = (error: unknown) => (error instanceof Error ? error.message : String(error));

/** The browser's notice that a resize observer waited a frame: no error, nothing to show. */
const RESIZE_NOTICE = /^ResizeObserver loop/;

/** Shows every error nothing caught with `show`; returns what stops it. */
export function catchUncaught(show: (text: string) => void, target: Window = window): () => void {
  const failed = (event: ErrorEvent) => {
    const said = reason(event.error ?? event.message);
    if (!RESIZE_NOTICE.test(said)) show(said);
  };
  const rejected = (event: PromiseRejectionEvent) => show(reason(event.reason));
  target.addEventListener("error", failed);
  target.addEventListener("unhandledrejection", rejected);
  return () => {
    target.removeEventListener("error", failed);
    target.removeEventListener("unhandledrejection", rejected);
  };
}
