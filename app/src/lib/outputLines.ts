// A line of toolchain output as a terminal shows it: a carriage return goes
// back to the start of the line, so what follows it is what stays (cargo's
// progress bar is written over by the next "Compiling ..." this way).

/** What a terminal would show of `raw`: the text after its last carriage return that holds something. */
export function shownLine(raw: string): string {
  const parts = raw.split("\r").filter((p) => p.trim() !== "");
  return (parts.at(-1) ?? "").trimEnd();
}
