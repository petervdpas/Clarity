// An application's keys set by the person (DESIGN 5.28), where its project
// file says `keys editable`: the model's catalog as commands per place (the
// app's frame is the place "", a window its name), the person's keys over the
// design's, kept in the settings file as `keys`, only what differs.

import type { Command, KeyChoices } from "./keys";
import type { KeyLayer } from "./generated/KeyLayer";

/** The catalog as commands, each named by its caption as the person reads it, or else its part name. */
export function appCommands(layers: KeyLayer[], caption: (text: string | null, name: string) => string): Command[] {
  return layers.flatMap((layer) => layer.commands.map((c) => ({ id: c.id, name: caption(c.caption, c.name), layers: [layer.id], keys: c.keys })));
}

/** The keys a part answers: the person's, or the one its design gives. */
export function keysOfCommand(id: string, designed: string | null, choices: KeyChoices): string[] {
  return choices[id] ?? (designed ? [designed] : []);
}

/** The person's keys as the settings file kept them: only lists of keys. */
export function personKeys(kept: unknown): KeyChoices {
  if (!kept || typeof kept !== "object") return {};
  return Object.fromEntries(Object.entries(kept).filter(([, keys]) => Array.isArray(keys) && keys.every((k) => typeof k === "string"))) as KeyChoices;
}
