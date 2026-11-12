// An application's version as the project file takes it:
// major.minor.patch, with an optional pre-release, like 1.2.0 or 2.0.0-beta.1.

/** A version as the project file takes it. */
export const VERSION_PATTERN = /^\d+\.\d+\.\d+(-[A-Za-z0-9]+(\.[A-Za-z0-9]+)*)?$/;

/** What a version field writes: the version, none for an empty field, or undefined for text that is no version (nothing is saved). */
export function versionWritten(text: string): string | null | undefined {
  const written = text.trim();
  if (written === "") return null;
  return VERSION_PATTERN.test(written) ? written : undefined;
}
