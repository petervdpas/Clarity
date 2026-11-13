// What the design canvas and preview say in their place when Clarity.UI's
// browser files (which they draw with) are missing or behind its source.
// The backend says how the files stand (the command browser_files,
// clarity_gen::clarity_ui::browser_files); this says it to the person.

/** How the browser files stand, as the backend says. */
export type BrowserFilesState = "fine" | "missing" | "stale";

/** The backend's answer: how the files stand, and whether this Clarity runs from its repository. */
export type BrowserFiles = { state: BrowserFilesState; repository: boolean };

/** What a canvas says instead of drawing, and whether it offers Rebuild. */
export type BrowserFilesNotice = { text: string; rebuild: boolean };

/** The title of the question before a rebuild. */
export const BROWSER_FILES_TITLE = "Clarity.UI's browser files";

/**
 * What to say about the browser files, or null when there is nothing to
 * say. A Clarity run from its repository builds them again (Rebuild); an
 * installed Clarity carries its own, so there they come back by installing
 * it again. An installed Clarity has no source to be behind.
 */
export function browserFilesNotice(files: BrowserFiles): BrowserFilesNotice | null {
  if (files.state === "missing" && files.repository) {
    return { text: "Clarity.UI's browser files are missing, so the design and preview cannot draw. Rebuild them from the repository's source (runtimes/rust-ui-browser/build.sh).", rebuild: true };
  }
  if (files.state === "missing") {
    return { text: "Clarity.UI's browser files are missing from this installation, so the design and preview cannot draw. Install Clarity again to bring them back.", rebuild: false };
  }
  if (files.state === "stale" && files.repository) {
    return { text: "Clarity.UI's browser files are older than its source, so the design and preview do not show what changed since. Rebuild them (runtimes/rust-ui-browser/build.sh).", rebuild: true };
  }
  return null;
}
