<script lang="ts">
  import IconField from "./IconField.svelte";
  import TextField from "./TextField.svelte";
  import { NARROWEST, WIDEST } from "./sidebars";
  import type { SidebarProps } from "./generated/SidebarProps";
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
    onapplied,
    owner,
  }: {
    preview: Preview;
    element: FrameElement;
    onedit: (edit: Edit) => void;
    /** Takes the project after a text's value or key changed in the language files. */
    onapplied: (preview: Preview) => void;
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

{#snippet nameField(part: { name: string | null; span: { start: number } })}
  <label class="prop">
    <span>Name</span>
    <input value={part.name ?? ""} title="The name the code behind uses" onchange={(e) => e.currentTarget.value.trim() && onedit({ op: "setName", item: part.span.start, name: e.currentTarget.value.trim() })} />
  </label>
{/snippet}

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
      <TextField {preview} {onapplied} text={p.caption} onwrite={(text) => update(p, { caption: text })} />
    </label>
    <label class="prop">
      <span>Icon</span>
      <IconField {preview} icon={p.icon} onwrite={(icon) => update(p, { icon })} />
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
    {@render nameField(element.menu)}
    <p class="hint">Code hides it with <code>{element.menu.name ?? "it"}.visible</code> and turns it off with <code>.enabled</code>.</p>
    <div class="actions">
      <button onclick={() => add(element.menu.span.start, "item")}>Add item</button>
      <button onclick={() => add(element.menu.span.start, "separator")}>Add separator</button>
      <button class="danger" onclick={() => remove(element.menu.span.start)}>Remove menu</button>
    </div>
  {:else if element.kind === "toolbar"}
    {@render nameField(element.toolbar)}
    <p class="hint">Code hides it with <code>{element.toolbar.name ?? "it"}.visible</code> and turns it off with <code>.enabled</code>.</p>
    <div class="actions">
      <button onclick={() => add(element.toolbar.span.start, "button")}>Add button</button>
      <button onclick={() => add(element.toolbar.span.start, "separator")}>Add separator</button>
      <button class="danger" onclick={() => remove(element.toolbar.span.start)}>Remove toolbar</button>
    </div>
  {:else if element.kind === "ribbon"}
    {@const r = element.ribbon}
    <label class="prop">
      <span>Name</span>
      <input value={r.name ?? ""} title="The name the code behind uses" onchange={(e) => onedit({ op: "updateRibbon", item: r.span.start, name: e.currentTarget.value.trim() || null, collapsible: r.collapsible, collapsed: r.collapsed })} />
    </label>
    <fieldset class="checks">
      <legend>The person can</legend>
      <label
        ><input type="checkbox" checked={r.collapsible} onchange={(e) => onedit({ op: "updateRibbon", item: r.span.start, name: r.name, collapsible: e.currentTarget.checked, collapsed: e.currentTarget.checked && r.collapsed })} /> fold
        it to its tabs (double-click a tab)</label
      >
      <label
        ><input type="checkbox" checked={r.collapsed} disabled={!r.collapsible} onchange={(e) => onedit({ op: "updateRibbon", item: r.span.start, name: r.name, collapsible: r.collapsible, collapsed: e.currentTarget.checked })} /> starts
        folded</label
      >
    </fieldset>
    <p class="hint">Code reads and sets <code>{r.name ?? "it"}.tab</code>, the open tab from 1, and <code>.collapsed</code>; a tab or group hides with <code>.visible</code>.</p>
    <div class="actions">
      <button onclick={() => add(element.ribbon.span.start, "tab")}>Add tab</button>
      <button class="danger" onclick={() => remove(element.ribbon.span.start)}>Remove ribbon</button>
    </div>
  {:else if element.kind === "ribbonTab" || element.kind === "ribbonGroup"}
    {@const item = element.kind === "ribbonTab" ? element.tab : element.group}
    {@render nameField(item)}
    <label class="prop">
      <span>Title</span>
      <TextField {preview} {onapplied} text={item.title} onwrite={(text) => onedit({ op: "setTitle", target: item.span.start, title: text ?? "" })} />
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
    {@const p = s.props}
    {@const set = (changes: Partial<SidebarProps>) => onedit({ op: "updateSidebar", item: s.span.start, props: { ...p, ...changes } })}
    {@const pixels = (e: Event) => Math.round(Number((e.currentTarget as HTMLInputElement).value)) || null}
    <label class="prop">
      <span>Name</span>
      <input value={p.name ?? ""} title="The name the code behind uses" onchange={(e) => set({ name: e.currentTarget.value.trim() || null })} />
    </label>
    <label class="prop">
      <span>Side</span>
      <select value={p.side} onchange={(e) => set({ side: e.currentTarget.value as Side })}>
        <option value="left">Left</option>
        <option value="right">Right</option>
      </select>
    </label>
    <label class="prop">
      <span>Width in pixels</span>
      <input type="number" min={NARROWEST} max={WIDEST} value={p.width} onchange={(e) => set({ width: pixels(e) ?? p.width })} />
    </label>
    <label class="prop">
      <span>Grid columns</span>
      <input type="number" min="1" max="24" value={p.columns} onchange={(e) => set({ columns: Math.max(1, pixels(e) ?? 1) })} />
    </label>
    <fieldset class="checks">
      <legend>The person can</legend>
      <label><input type="checkbox" checked={p.resizable} onchange={(e) => set({ resizable: e.currentTarget.checked, min: e.currentTarget.checked ? p.min : null, max: e.currentTarget.checked ? p.max : null })} /> drag its edge to size it</label>
      {#if p.resizable}
        <div class="limits">
          <label>Narrowest <input type="number" min={NARROWEST} max={p.width} placeholder={String(NARROWEST)} value={p.min ?? ""} onchange={(e) => set({ min: pixels(e) })} /></label>
          <label>Widest <input type="number" min={p.width} max={WIDEST} placeholder={String(WIDEST)} value={p.max ?? ""} onchange={(e) => set({ max: pixels(e) })} /></label>
        </div>
      {/if}
      <label><input type="checkbox" checked={p.collapsible} onchange={(e) => set({ collapsible: e.currentTarget.checked, collapsed: e.currentTarget.checked && p.collapsed })} /> fold it to a button</label>
      <label><input type="checkbox" checked={p.collapsed} disabled={!p.collapsible} onchange={(e) => set({ collapsed: e.currentTarget.checked })} /> starts folded</label>
      <label><input type="checkbox" checked={p.maximizable} onchange={(e) => set({ maximizable: e.currentTarget.checked })} /> let it fill the window</label>
    </fieldset>
    <p class="hint">Drag parts onto the sidebar; it has its own snap grid. Code reads and sets <code>{p.name ?? "it"}.collapsed</code> and <code>.maximized</code>.</p>
    <div class="actions">
      <button class="danger" onclick={() => remove(s.span.start)}>Remove sidebar</button>
    </div>
  {:else if element.kind === "footer" || element.kind === "status"}
    {@const f = element.footer}
    {#if f.statusSpan}
      {@const statusId = f.statusSpan.start}
      <label class="prop">
        <span>Status text</span>
        <TextField {preview} {onapplied} text={f.status} onwrite={(text) => onedit({ op: "setTitle", target: statusId, title: text ?? "" })} />
      </label>
    {/if}
    <p class="hint">Drag widgets onto the footer row next to the status text.</p>
    <div class="actions">
      <button class="danger" onclick={() => remove(f.span.start)}>Remove footer</button>
    </div>
  {/if}
{/if}

<style>
  .limits {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
    padding-left: 22px;
    font-size: 12px;
  }

  .limits input {
    width: 100%;
  }

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
