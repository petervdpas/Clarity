<script lang="ts">
  import { commandHandler, wireCommand } from "./codeParts";
  // Properties of a picked frame element: a region, a ribbon tab or group,
  // a menu item, a button, a separator or the footer status.

  import type { CommandProps } from "./generated/CommandProps";
  import type { Edit } from "./generated/Edit";
  import type { FrameItem } from "./generated/FrameItem";
  import type { Preview } from "./generated/Preview";
  import type { Side } from "./generated/Side";
  import type { FrameElement } from "./frame";
  import { pageDesigner } from "./pageDesignerStore.svelte";
  import { allPages } from "./pages";

  let {
    preview,
    element,
    onedit,
    owner,
  }: {
    preview: Preview;
    element: FrameElement;
    onedit: (edit: Edit) => void;
    /** The window or app whose code behind holds the handlers. */
    owner: string;
  } = $props();

  const REGION_NAMES = {
    menu: "Menu",
    toolbar: "Toolbar",
    ribbon: "Ribbon",
    sidebar: "Sidebar",
    footer: "Footer",
    status: "Status text",
    ribbonTab: "Ribbon tab",
    ribbonGroup: "Ribbon group",
  } as const;

  const COMMAND_NAMES = {
    item: "Menu item",
    button: "Button",
    separator: "Separator",
  } as const;

  const targets = $derived([
    ...allPages(preview).map((p) => p.name.name),
    ...preview.commands,
  ]);

  function textOf(e: Event): string | null {
    const value = (e.currentTarget as HTMLInputElement).value.trim();
    return value === "" ? null : value;
  }

  function add(parent: number, item: FrameItem) {
    onedit({ op: "addFrameItem", parent, item, caption: null });
  }

  function update(props: CommandProps, changes: Partial<CommandProps>) {
    if (element.kind !== "command") return;
    onedit({ op: "updateCommand", item: element.command.span.start, props: { ...props, ...changes } });
  }

  /** Wires a click handler, together with an empty handler in the code behind, or unwires it. */
  function setHandler(props: CommandProps, handler: string | null) {
    if (element.kind !== "command") return;
    onedit(wireCommand({ item: element.command.span.start, props, owner, handler }));
    if (handler) pageDesigner.showCode(handler);
  }

  function remove(item: number) {
    onedit({ op: "remove", item });
    pageDesigner.selected = null;
  }

  function move(item: number, delta: number) {
    onedit({ op: "moveSibling", item, delta });
  }
</script>

{#if element.kind === "command"}
  {@const c = element.command}
  {@const p = c.props}
  <div class="kind">{COMMAND_NAMES[p.kind]}</div>
  {#if p.kind !== "separator"}
    <label class="prop">
      <span>Name</span>
      <input value={p.name ?? ""} placeholder="the name the code behind uses" onchange={(e) => update(p, { name: textOf(e) })} />
    </label>
    <label class="prop">
      <span>Caption</span>
      <input value={p.caption ?? ""} onchange={(e) => update(p, { caption: textOf(e) })} />
    </label>
    <label class="prop">
      <span>Icon</span>
      <input value={p.icon ?? ""} placeholder="icon name" onchange={(e) => update(p, { icon: textOf(e) })} />
    </label>
    {#if p.kind === "item"}
      <label class="prop">
        <span>Shortcut</span>
        <input value={p.shortcut ?? ""} placeholder="Ctrl+N" onchange={(e) => update(p, { shortcut: textOf(e) })} />
      </label>
    {/if}
    {#if p.kind === "button" && element.parent === "group"}
      <label class="inline">
        <input type="checkbox" checked={p.large} onchange={(e) => update(p, { large: e.currentTarget.checked })} />
        Large button
      </label>
    {/if}
    {#if c.items.length === 0}
      <fieldset class="events">
        <legend>Events</legend>
        <div class="event-row">
          <span class="event-name">click</span>
          <input
            value={p.handler ?? ""}
            placeholder="handler"
            onchange={(e) => setHandler(p, textOf(e))}
          />
          {#if p.handler}
            <button title="Show the handler in the code behind" onclick={() => pageDesigner.showCode(p.handler)}>Code</button>
            <button title="Unwire" onclick={() => setHandler(p, null)}>&times;</button>
          {:else}
            <button
              title="Wire a handler named after the item"
              onclick={() => setHandler(p, commandHandler(p))}>Wire</button
            >
          {/if}
        </div>
        <p class="hint">Handlers are written in Clarity, in the window's code behind.</p>
      </fieldset>
      <label class="prop">
        <span>Or on click, open</span>
        <select
          value={p.onClick ?? ""}
          onchange={(e) => update(p, { onClick: textOf(e), handler: textOf(e) ? null : p.handler, newWindow: textOf(e) ? p.newWindow : false })}
        >
          <option value="">(nothing)</option>
          {#each targets as target (target)}
            <option value={target}>{target}</option>
          {/each}
        </select>
      </label>
      {#if p.onClick && !preview.commands.includes(p.onClick)}
        <label class="inline" title="Opens in a desktop window of its own. Without it, the window opens in place, on top of this one.">
          <input type="checkbox" checked={p.newWindow} onchange={(e) => update(p, { newWindow: e.currentTarget.checked })} /> In a new window
        </label>
      {/if}
    {:else}
      <p class="hint">Opens its submenu.</p>
    {/if}
  {/if}
  <div class="actions">
    {#if p.kind === "item" && p.onClick === null}
      <button onclick={() => add(c.span.start, "item")}>Add submenu item</button>
    {/if}
    <button disabled={element.index === 0} onclick={() => move(c.span.start, -1)}>Move up</button>
    <button disabled={element.index === element.count - 1} onclick={() => move(c.span.start, 1)}>Move down</button>
    <button class="danger" onclick={() => remove(c.span.start)}>Remove</button>
  </div>
{:else}
  <div class="kind">{REGION_NAMES[element.kind]}</div>
  {#if element.kind === "menu"}
    <div class="actions">
      <button onclick={() => add(element.menu.span.start, "item")}>Add item</button>
      <button onclick={() => add(element.menu.span.start, "separator")}>Add separator</button>
      <button class="danger" onclick={() => remove(element.menu.span.start)}>Remove menu</button>
    </div>
  {:else if element.kind === "toolbar"}
    <div class="actions">
      <button onclick={() => add(element.toolbar.span.start, "button")}>Add button</button>
      <button onclick={() => add(element.toolbar.span.start, "separator")}>Add separator</button>
      <button class="danger" onclick={() => remove(element.toolbar.span.start)}>Remove toolbar</button>
    </div>
  {:else if element.kind === "ribbon"}
    <div class="actions">
      <button onclick={() => add(element.ribbon.span.start, "tab")}>Add tab</button>
      <button class="danger" onclick={() => remove(element.ribbon.span.start)}>Remove ribbon</button>
    </div>
  {:else if element.kind === "ribbonTab" || element.kind === "ribbonGroup"}
    {@const item = element.kind === "ribbonTab" ? element.tab : element.group}
    <label class="prop">
      <span>Title</span>
      <input value={item.title} onchange={(e) => onedit({ op: "setTitle", target: item.span.start, title: e.currentTarget.value })} />
    </label>
    <div class="actions">
      {#if element.kind === "ribbonTab"}
        <button onclick={() => add(item.span.start, "group")}>Add group</button>
      {:else}
        <button onclick={() => add(item.span.start, "button")}>Add button</button>
        <button onclick={() => add(item.span.start, "separator")}>Add separator</button>
      {/if}
      <button disabled={element.index === 0} onclick={() => move(item.span.start, -1)}>Move left</button>
      <button disabled={element.index === element.count - 1} onclick={() => move(item.span.start, 1)}>Move right</button>
      <button class="danger" onclick={() => remove(item.span.start)}>Remove</button>
    </div>
  {:else if element.kind === "sidebar"}
    {@const s = element.sidebar}
    <label class="prop">
      <span>Side</span>
      <select value={s.side} onchange={(e) => onedit({ op: "setSidebar", item: s.span.start, side: e.currentTarget.value as Side, width: s.width })}>
        <option value="left">Left</option>
        <option value="right">Right</option>
      </select>
    </label>
    <label class="prop">
      <span>Width in columns</span>
      <input
        type="number"
        min="1"
        max="24"
        value={s.width}
        onchange={(e) => onedit({ op: "setSidebar", item: s.span.start, side: s.side, width: Math.max(1, Number(e.currentTarget.value) || 1) })}
      />
    </label>
    <p class="hint">Drag widgets onto the sidebar; it has its own snap grid.</p>
    <div class="actions">
      <button class="danger" onclick={() => remove(s.span.start)}>Remove sidebar</button>
    </div>
  {:else if element.kind === "footer" || element.kind === "status"}
    {@const f = element.footer}
    {#if f.statusSpan}
      {@const statusId = f.statusSpan.start}
      <label class="prop">
        <span>Status text</span>
        <input value={f.status ?? ""} onchange={(e) => onedit({ op: "setTitle", target: statusId, title: e.currentTarget.value })} />
      </label>
    {/if}
    <p class="hint">Drag widgets onto the footer row next to the status text.</p>
    <div class="actions">
      <button class="danger" onclick={() => remove(f.span.start)}>Remove footer</button>
    </div>
  {/if}
{/if}

<style>
  .kind {
    font-weight: 600;
  }

  .prop {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .inline {
    display: flex;
    gap: 6px;
    align-items: center;
    font-size: 13px;
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .events {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    margin: 0;
    padding: 6px 10px 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .events legend {
    font-size: 12px;
    color: var(--text-muted);
    padding: 0 4px;
  }

  .event-row {
    display: grid;
    grid-template-columns: 48px minmax(0, 1fr) auto;
    gap: 6px;
    align-items: center;
  }

  .event-name {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--text-muted);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .danger {
    color: var(--error);
  }
</style>
