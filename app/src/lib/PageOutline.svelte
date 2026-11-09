<script lang="ts">
  import { tr } from "@clarity/ui/screen/model.ts";
  import type { Command } from "@clarity/ui/generated/Command.ts";
  import type { Edit } from "@clarity/ui/generated/Edit.ts";
  import type { Preview } from "@clarity/ui/generated/Preview.ts";
  import type { Widget } from "@clarity/ui/generated/Widget.ts";
  import { pageDesigner } from "./pageDesignerStore.svelte";
  import type { ShownFrame } from "@clarity/ui/screen/frame.ts";
  import { placeOf, widgetInfo, type DesignKey, type ShownPage } from "@clarity/ui/screen/pages.ts";

  let {
    preview,
    shown,
    onedit,
    frame,
    rootName = null,
  }: {
    preview: Preview;
    /** The window as designed, with the key of each thing it shows. */
    shown: ShownPage;
    /** An edit in the places of the file of `key`. */
    onedit: (edit: Edit, key: DesignKey) => void;
    /** The regions the window shows, its own and the app's. */
    frame: ShownFrame;
    /** Overrides the root line, for the app. */
    rootName?: string | null;
  } = $props();

  /** `id` is the thing's key; null for lines that cannot be picked, such as regions from the app. */
  type Line = { id: DesignKey | null; key: string; depth: number; kind: string; text: string; removable: boolean };

  const page = $derived(shown.page);
  /** A line's key in the list: the thing's key, or its kind and place for one without. */
  const keyed = (thing: { span: { start: number } }) => {
    const id = shown.keyOf(thing);
    return { id, key: id ?? `at ${thing.span.start}` };
  };

  const lines = $derived.by(() => {
    const out: Line[] = [];
    const walk = (widgets: Widget[], depth: number) => {
      // Reading order: top to bottom, then left to right.
      const sorted = widgets.toSorted((a, b) => a.props.row - b.props.row || a.props.col - b.props.col);
      for (const w of sorted.filter((w) => !w.props.made)) {
        const p = w.props;
        const text = p.name ?? (p.caption ? tr(preview, p.caption) : null) ?? p.bind ?? "";
        out.push({ ...keyed(w), depth, kind: widgetInfo(preview, p.kind).name, text, removable: true });
        if (w.menu) {
          out.push({ ...keyed(w.menu), depth: depth + 1, kind: "Context menu", text: w.menu.name ?? "", removable: true });
          commands(w.menu.items, depth + 2);
        }
        walk(w.children, depth + 1);
        for (const inner of w.pages) {
          out.push({
            ...keyed(inner),
            depth: depth + 1,
            kind: p.kind === "content" ? "View" : inner.title === null ? "Pane" : p.kind === "steps" ? "Step" : p.kind === "accordion" ? "Section" : "Tab",
            text: p.kind === "content" ? (inner.view ?? "") : inner.title ? tr(preview, inner.title) : "",
            removable: inner.title !== null || p.kind === "content",
          });
          walk(inner.children, depth + 2);
        }
      }
    };
    const COMMAND_NAMES = { item: "Item", button: "Button", separator: "Separator" } as const;
    const commands = (items: Command[], depth: number) => {
      for (const c of items) {
        const kind = c.items.length > 0 ? "Submenu" : COMMAND_NAMES[c.props.kind];
        out.push({ ...keyed(c), depth, kind, text: c.props.name ?? (c.props.caption ? tr(preview, c.props.caption) : ""), removable: true });
        commands(c.items, depth + 1);
      }
    };
    const region = (thing: { span: { start: number } }, kind: string, inherited: boolean, text = "") => {
      out.push(
        inherited
          ? { id: null, key: "app:" + kind, depth: 1, kind, text: "from app", removable: false }
          : { ...keyed(thing), depth: 1, kind, text, removable: true },
      );
      return !inherited;
    };
    const f = frame;
    if (f.menu && region(f.menu.value, "Menu", f.menu.inherited)) commands(f.menu.value.items, 2);
    if (f.toolbar && region(f.toolbar.value, "Toolbar", f.toolbar.inherited)) commands(f.toolbar.value.items, 2);
    if (f.ribbon && region(f.ribbon.value, "Ribbon", f.ribbon.inherited)) {
      for (const tab of f.ribbon.value.tabs) {
        out.push({ ...keyed(tab), depth: 2, kind: "Tab", text: tab.title, removable: true });
        for (const group of tab.groups) {
          out.push({ ...keyed(group), depth: 3, kind: "Group", text: group.title, removable: true });
          commands(group.items, 4);
        }
      }
    }
    for (const side of [f.left, f.right]) {
      if (side && region(side.value, "Sidebar", side.inherited, side.value.props.side)) walk(side.value.widgets, 2);
    }
    if (f.footer && region(f.footer.value, "Footer", f.footer.inherited)) {
      const status = f.footer.value.statusSpan;
      if (status) out.push({ ...keyed({ span: status }), depth: 2, kind: "Status", text: f.footer.value.status ?? "", removable: false });
      walk(f.footer.value.widgets, 2);
    }
    walk(page.widgets, 1);
    return out;
  });

  const picked = $derived(lines.find((l) => l.id === pageDesigner.selected) ?? null);
</script>

<section>
  <h3>Window</h3>
  <div class="tree" role="tree">
    <button
      class="line"
      class:selected={pageDesigner.selected === null}
      role="treeitem"
      aria-selected={pageDesigner.selected === null}
      onclick={() => (pageDesigner.selected = null)}
    >
      <span class="kind">{rootName ?? (page.kind === "window" ? "Window" : page.kind === "dialog" ? "Dialog" : "View")}</span>
      <span class="muted">"{page.title ? tr(preview, page.title) : page.name.name}"</span>
    </button>
    {#each lines as line (line.key)}
      <button
        disabled={line.id === null}
        class="line"
        class:selected={pageDesigner.selected === line.id}
        style:padding-left="{line.depth * 14 + 6}px"
        role="treeitem"
        aria-selected={pageDesigner.selected === line.id}
        onclick={() => line.id !== null && (pageDesigner.selected = line.id)}
      >
        <span class="kind">{line.kind}</span>
        {#if line.text}<span class="muted">{line.text}</span>{/if}
      </button>
    {/each}
  </div>
  <button
    class="danger"
    disabled={!picked?.removable}
    onclick={() => {
      const placed = placeOf(preview, picked?.id ?? null);
      if (picked?.id != null && placed) onedit({ op: "remove", item: placed.at }, picked.id);
      pageDesigner.selected = null;
    }}>Remove</button
  >
</section>

<style>
  section {
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  h3 {
    margin: 0;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }

  .tree {
    display: flex;
    flex-direction: column;
  }

  .line {
    border: none;
    border-radius: var(--radius);
    background: transparent;
    text-align: left;
    padding: 3px 6px;
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .line:hover {
    background: var(--panel-alt);
  }

  .line.selected {
    background: var(--accent-soft);
  }

  .kind {
    font-weight: 600;
  }

  .muted {
    color: var(--text-muted);
  }

  .danger {
    color: var(--error);
  }
</style>
