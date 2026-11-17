/** What a start needs: opening a project (failing with why), forgetting
    the one remembered from last time, saying what went wrong (an error),
    warning of what the person can simply carry on from, and opening the
    project browser to pick another. */
export interface Starting {
  open(path: string): Promise<void>;
  forget(): void;
  say(text: string): void;
  warn(text: string): void;
  choose(): void;
}

/** Opens the project the IDE starts with: the one given (dev.sh's or the
    command line's), else the one remembered from last time. A remembered
    project that is missing (moved, deleted, a folder gone with it) opens
    nothing and is forgotten, so the next start is clean, with a warning
    instead of the system's error and the project browser open to pick
    another; one given is said with why, as whoever gave it can give another. */
export async function startProject(given: string | null, remembered: string | null, how: Starting): Promise<void> {
  const path = given ?? remembered;
  if (!path) return;
  try {
    await how.open(path);
  } catch (err) {
    if (given) {
      how.say(String(err));
      return;
    }
    how.forget();
    how.warn(`Project missing: ${path}`);
    how.choose();
  }
}
