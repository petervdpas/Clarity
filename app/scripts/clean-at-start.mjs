// Removes the folders Clarity settings > DevTools > Disk marked for the next
// start (target/remove-at-start, written by clarity_gen's `disk::mark`),
// before the IDE builds or runs: on Windows a running IDE holds its own
// files (target/debug), which Remove then cannot take. dev.sh and dev.bat
// run it; node, as Clarity's own command may be in a folder that goes. Each
// line names a folder inside the repository; nothing outside it is removed,
// and a folder still held keeps its mark for the start after.
import { existsSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

/** Removes what `root`'s marks name; says each as it goes. Gives the folders removed. */
export function cleanAtStart(root, say = console.log) {
  const marks = join(root, "target", "remove-at-start");
  if (!existsSync(marks)) return [];
  const removed = [];
  const kept = [];
  for (const line of readFileSync(marks, "utf8").split(/\r?\n/)) {
    const folder = line.trim();
    if (!folder) continue;
    const path = resolve(root, folder);
    const inside = relative(root, path);
    if (!inside || inside.startsWith("..") || isAbsolute(inside)) {
      say(`Clarity's start cleanup: ${folder} is not a folder inside the repository; left alone`);
      continue;
    }
    if (!existsSync(path)) continue;
    try {
      rmSync(path, { recursive: true, force: true });
      removed.push(folder);
      say(`Removed ${folder}, marked in Clarity settings > DevTools > Disk; it builds again`);
    } catch (err) {
      kept.push(folder);
      say(`Clarity's start cleanup: cannot remove ${folder} yet (${err.message}); marked for the next start`);
    }
  }
  if (kept.length) writeFileSync(marks, kept.map((f) => `${f}\n`).join(""));
  else rmSync(marks, { force: true });
  return removed;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  cleanAtStart(resolve(dirname(fileURLToPath(import.meta.url)), "..", ".."));
}
