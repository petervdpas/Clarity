// What the design canvas and preview say in their place when Clarity.UI's
// browser files (which they draw with) are missing.
// The backend says how the files stand (the command browser_files,
// clarity_gen::clarity_ui::browser_files); this says it to the person.

/** How the browser files stand, as the backend says. */
export type BrowserFilesState = "fine" | "missing" | "stale";

/** The backend's answer: how the files stand, and whether this Clarity runs from its repository. */
export type BrowserFiles = { state: BrowserFilesState; repository: boolean };

/** What a canvas says instead of drawing. */
export type BrowserFilesNotice = { text: string };

/**
 * What to say about the browser files, or null when there is nothing to
 * say. A run from its repository uses the release's, fetched once, or the
 * cDdev's own swapped in (Clarity settings > DevTools, which builds them and
 * signals when that build is behind its source); an installed
 * Clarity carries its own, so there they come back by installing it again.
 */
export function browserFilesNotice(files: BrowserFiles): BrowserFilesNotice | null {
  if (files.state === "missing" && files.repository) {
    return { text: "Clarity.UI's browser files are not here, so the design and preview cannot draw: the release's could not be fetched. Connect to fetch them, or swap in your own build in Clarity settings > DevTools." };
  }
  if (files.state === "missing") {
    return { text: "Clarity.UI's browser files are missing from this installation, so the design and preview cannot draw. Install Clarity again to bring them back." };
  }
  return null;
}
