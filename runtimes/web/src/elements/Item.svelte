<script lang="ts">
  // An item of a part's menu in a generated window (DESIGN 7.8), its submenu's
  // items inside it: what differs from a new item, in Clarity's words.
  import { untrack, type Snippet } from "svelte";
  import defaults from "../generated/command-defaults.json";
  import type { Command } from "../generated/Command.ts";
  import type { CommandProps } from "../generated/CommandProps.ts";
  import { provideHolder, useHolder } from "./holder.ts";

  let { children, ...given }: { children?: Snippet } & Record<string, unknown> = $props();

  /** The words Clarity writes for a field of an item, where they differ from its name. */
  const FIELDS: Record<string, keyof CommandProps> = { key: "shortcut", opens: "onClick", onclick: "handler" };

  const holder = useHolder();
  const place = holder.next();
  const command: Command = untrack(() => {
    const props = structuredClone(defaults) as CommandProps;
    for (const [word, value] of Object.entries(given)) (props as Record<string, unknown>)[FIELDS[word] ?? word] = value;
    return { props, items: [], span: { start: place, end: place, line: 0, col: 0 } };
  });
  holder.commands?.push(command);
  provideHolder({ widgets: [], next: holder.next, commands: command.items });
</script>

{@render children?.()}
