<script lang="ts">
  import { tr } from "./model";
  import type { Command } from "./generated/Command";
  import type { Edit } from "./generated/Edit";
  import type { Page } from "./generated/Page";
  import type { Preview } from "./generated/Preview";
  import type { Widget } from "./generated/Widget";
  import { pageDesigner } from "./pageDesignerStore.svelte";
  import type { ShownFrame } from "./frame";
  import { widgetInfo } from "./pages";

  let {
    preview,
    page,
    onedit,
    frame,
    rootName = null,
  }: {
    preview: Preview;
    page: Page;
    onedit: (edit: Edit) => void;
    /** The regions the window shows, its own and the app's. */
    frame: ShownFrame;
    /** Overrides the root line, for the app. */
    rootName?: string | null;
  } = $props();

  /** `id` is null for lines that cannot be picked, such as regions from the app. */
  type Line = { id: number | null; key: string; depth: number; kind: string; text: string; removable: boolean };

  const lines = $derived.by(() => {
    const out: Line[] = [];
    const walk = (widgets: Widget[], depth: number) => {
      // Reading order: top to bottom, then left to right.
      const sorted = widgets.toSorted((a, b) => a.props.row - b.props.row || a.props.col - b.props.col);
      for (const w of sorted.filter((w) => !w.props.made)) {
        const p = w.props;
        const text = p.name ?? (p.caption ? tr(preview, p.caption) : null) ?? p.bind ?? "";
        out.push({ id: w.span.start, key: String(w.span.start), depth, kind: widgetInfo(preview, p.kind).name, text, removable: true });
        if (w.menu) {
          out.push({ id: w.menu.span.start, key: String(w.menu.span.start), depth: depth + 1, kind: "Context menu", text: w.menu.name ?? "", removable: true });
          commands(w.menu.items, depth + 2);
        }
        walk(w.children, depth + 1);
        for (const inner of w.pages) {
          out.push({
            id: inner.span.start,
            key: String(inner.span.start),
            depth: depth + 1,
            kind: inner.title === null ? "Pane" : p.kind === "steps" ? "Step" : p.kind === "accordion" ? "Section" : "Tab",
            text: inner.title ? tr(preview, inner.title) : "",
            removable: inner.title !== null,
          });
          walk(inner.children, depth + 2);
        }
      }
    };
    const COMMAND_NAMES = { item: "Item", button: "Button", separator: "Separator" } as const;
    const commands = (items: Command[], depth: number) => {
      for (const c of items) {
        const kind = c.items.length > 0 ? "Submenu" : COMMAND_NAMES[c.props.kind];
        out.push({ id: c.span.start, key: String(c.span.start), depth, kind, text: c.props.name ?? (c.props.caption ? tr(preview, c.props.caption) : ""), removable: true });
        commands(c.items, depth + 1);
      }
    };
    const region = (id: number, kind: string, inherited: boolean, text = "") => {
      out.push(
        inherited
          ? { id: null, key: "app:" + kind, depth: 1, kind, text: "from app", removable: false }
          : { id, key: String(id), depth: 1, kind, text, removable: true },
      );
      return !inherited;
    };
    const f = frame;
    if (f.menu && region(f.menu.value.span.start, "Menu", f.menu.inherited)) commands(f.menu.value.items, 2);
    if (f.toolbar && region(f.toolbar.value.span.start, "Toolbar", f.toolbar.inherited)) commands(f.toolbar.value.items, 2);
    if (f.ribbon && region(f.ribbon.value.span.start, "Ribbon", f.ribbon.inherited)) {
      for (const tab of f.ribbon.value.tabs) {
        out.push({ id: tab.span.start, key: String(tab.span.start), depth: 2, kind: "Tab", text: tab.title, removable: true });
        for (const group of tab.groups) {
          out.push({ id: group.span.start, key: String(group.span.start), depth: 3, kind: "Group", text: group.title, removable: true });
          commands(group.items, 4);
        }
      }
    }
    for (const shown of [f.left, f.right]) {
      if (shown && region(shown.value.span.start, "Sidebar", shown.inherited, shown.value.props.side)) walk(shown.value.widgets, 2);
    }
    if (f.footer && region(f.footer.value.span.start, "Footer", f.footer.inherited)) {
      const status = f.footer.value.statusSpan;
      if (status) out.push({ id: status.start, key: String(status.start), depth: 2, kind: "Status", text: f.footer.value.status ?? "", removable: false });
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
      if (picked?.id != null) onedit({ op: "remove", item: picked.id });
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
