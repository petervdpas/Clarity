<script lang="ts">
  import { shownTab } from "./ribbons";
  import { STRIP_WIDTH, sidebarWidth } from "./sidebars";
  // The frame around a window: menu, toolbar and ribbon at the top, sidebars
  // at the sides, the footer at the bottom, and the window body in between.
  // In design mode the window's own regions can be picked; regions that come
  // from the app are shown dimmed.

  import type { Snippet } from "svelte";
  import GridBox from "./GridBox.svelte";
  import type { Command } from "./generated/Command";
  import type { Dock } from "./generated/Dock";
  import type { Page } from "./generated/Page";
  import type { Preview } from "./generated/Preview";
  import type { Sidebar } from "./generated/Sidebar";
  import type { Ribbon } from "./generated/Ribbon";
  import type { ShownFrame } from "./frame";
  import { pageDesigner } from "./pageDesignerStore.svelte";
  import type { PageData } from "./pages";
  import { tr } from "./model";
  import { useLive } from "./live";
  import Splitter from "./Splitter.svelte";
  import type { Side } from "./generated/Side";

  let {
    preview,
    page,
    frame,
    data,
    design,
    onnavigate,
    children,
  }: {
    preview: Preview;
    /** The page (or app) whose widgets the sidebars and footer belong to; null when nothing has a frame. */
    page: Page | null;
    frame: ShownFrame;
    data: PageData;
    design: boolean;
    onnavigate: (target: string, newWindow?: boolean) => void;
    children: Snippet;
  } = $props();

  const DOCKS: Dock[] = ["top", "left", "right", "bottom"];

  /** The open top-level menu item. */
  let openMenu = $state<number | null>(null);
  // A ribbon as the person left it, outside the designer: the open tab (from
  // 1) and whether it is folded; live, the ribbon's part state carries them to
  // the handlers, which also hide its tabs and groups (conformance/ribbons.json).
  let ribbonHere = $state<{ tab?: number; collapsed?: boolean }>({});

  /** Whether a named tab or group shows: designing, always; else while its state is not hidden. */
  function shows(name: string | null): boolean {
    return design || !live || !name || live.parts[name]?.visible !== false;
  }

  function ribbonNow(r: Ribbon): { tab: number | null; collapsed: boolean } {
    const part = live && r.name ? live.parts[r.name] : undefined;
    const asked = part?.tab ?? ribbonHere.tab ?? 1;
    const collapsed = !design && r.collapsible && (part?.collapsed ?? ribbonHere.collapsed ?? r.collapsed);
    return { tab: shownTab(r.tabs.map((t) => shows(t.name)), asked), collapsed };
  }

  function setRibbon(r: Ribbon, change: { tab?: number; collapsed?: boolean }) {
    if (!r.collapsible) delete change.collapsed;
    if (live && r.name && !design) live.setRibbon(r.name, change);
    else ribbonHere = { ...ribbonHere, ...change };
  }

  const regionDrag = $derived(design && pageDesigner.drag?.kind === "region" ? pageDesigner.drag : null);

  function pick(e: Event, id: number) {
    e.stopPropagation();
    pageDesigner.selected = id;
  }

  /** A double click on a command while designing: its code, here or (for the app's, in a window) in the app. */
  function codeOf(e: Event, command: Command, editable: boolean) {
    if (!design || command.props.kind === "separator") return;
    e.stopPropagation();
    if (editable) pageDesigner.openPartCode(command.span.start);
    else pageDesigner.openInApp(command.span.start);
  }

  /** A click on a command: picks it while designing, runs it otherwise. */
  function run(e: Event, command: Command, editable: boolean) {
    if (design) {
      if (editable) pick(e, command.span.start);
      return;
    }
    // A command needs no name to run its handler, as a part does not.
    if (command.props.handler && live) {
      openMenu = null;
      live.event(command.props.name ?? "", "click", command.props.handler);
    } else if (command.props.onClick) {
      openMenu = null;
      onnavigate(command.props.onClick, command.props.newWindow);
    }
  }

  // In a running application, commands report their clicks to the backend.
  const live = useLive();

  // A sidebar as the person left it, outside the designer: the width it was
  // dragged to, and whether it is folded or fills the window. Live, the
  // part's state carries folded and filling to the handlers (conformance/sidebars.json).
  let draggedWidth = $state<Partial<Record<Side, number>>>({});
  let here = $state<Partial<Record<Side, { collapsed?: boolean; maximized?: boolean }>>>({});

  function stateOf(s: Sidebar): { collapsed: boolean; maximized: boolean } {
    const part = live && s.props.name ? live.parts[s.props.name] : undefined;
    const own = here[s.props.side];
    return {
      collapsed: !design && s.props.collapsible && (part?.collapsed ?? own?.collapsed ?? s.props.collapsed),
      maximized: !design && s.props.maximizable && (part?.maximized ?? own?.maximized ?? false),
    };
  }

  function setState(s: Sidebar, change: { collapsed?: boolean; maximized?: boolean }) {
    if (live && s.props.name) live.setSidebar(s.props.name, change);
    else here[s.props.side] = { ...here[s.props.side], ...change };
  }

  function widthOf(s: Sidebar): number {
    return draggedWidth[s.props.side] ?? s.props.width;
  }

  function resizeSidebar(shown: { value: Sidebar } | null, delta: number) {
    if (!shown) return;
    const s = shown.value;
    draggedWidth[s.props.side] = sidebarWidth(widthOf(s) + delta, s.props.min, s.props.max);
  }

  /** The sidebar that fills the window, if one does. */
  const filling = $derived([frame.left, frame.right].find((shown) => shown && stateOf(shown.value).maximized)?.value ?? null);

  function containsPicked(command: Command): boolean {
    const id = pageDesigner.selected;
    return id !== null && (command.span.start === id || command.items.some(containsPicked));
  }

  function iconLetter(icon: string): string {
    return icon.slice(0, 1).toUpperCase();
  }

  function onwindowpointerdown(e: PointerEvent) {
    if (openMenu === null) return;
    if (!(e.target instanceof Element) || !e.target.closest(".menubar")) openMenu = null;
  }
</script>

<svelte:window onpointerdown={onwindowpointerdown} />

{#snippet entries(items: Command[], editable: boolean)}
  {#each items as item (item.span.start)}
    {#if item.props.kind === "separator"}
      <div
        class="separator-line"
        class:picked={design && pageDesigner.selected === item.span.start}
        role="separator"
        onpointerdown={(e) => design && editable && pick(e, item.span.start)}
      ></div>
    {:else}
      <div class="entry" class:sub={item.items.length > 0} class:show={design && containsPicked(item)}>
        <button
          class:picked={design && pageDesigner.selected === item.span.start}
          title={design && !editable ? "From the app: double-click to edit it in the app" : undefined}
          onpointerdown={(e) => design && run(e, item, editable)}
          onclick={(e) => !design && run(e, item, editable)}
          ondblclick={(e) => codeOf(e, item, editable)}
        >
          <span>{item.props.caption ? tr(preview, item.props.caption) : ""}</span>
          {#if item.props.shortcut}<kbd>{item.props.shortcut}</kbd>{/if}
          {#if item.items.length > 0}<span class="arrow">&rsaquo;</span>{/if}
        </button>
        {#if item.items.length > 0}
          <div class="flyout">{@render entries(item.items, editable)}</div>
        {/if}
      </div>
    {/if}
  {/each}
{/snippet}

{#snippet button(item: Command, editable: boolean, large: boolean)}
  {#if item.props.kind === "separator"}
    <span
      class="vsep"
      class:picked={design && pageDesigner.selected === item.span.start}
      role="separator"
      onpointerdown={(e) => design && editable && pick(e, item.span.start)}
    ></span>
  {:else}
    <button
      class="tool"
      class:large={large && item.props.large}
      class:picked={design && pageDesigner.selected === item.span.start}
      title={design && !editable ? "From the app: double-click to edit it in the app" : item.props.onClick ? `Opens ${item.props.onClick}` : undefined}
      onpointerdown={(e) => design && run(e, item, editable)}
      onclick={(e) => !design && run(e, item, editable)}
      ondblclick={(e) => codeOf(e, item, editable)}
    >
      {#if item.props.icon}<span class="icon">{iconLetter(item.props.icon)}</span>{/if}
      {#if item.props.caption}<span>{item.props.caption ? tr(preview, item.props.caption) : ""}</span>{/if}
    </button>
  {/if}
{/snippet}

{#snippet sidebar(shown: { value: Sidebar; inherited: boolean })}
  {@const s = shown.value}
  {@const now = stateOf(s)}
  <div
    class="sidebar {s.props.side}"
    class:inherited={design && shown.inherited}
    class:picked={design && pageDesigner.selected === s.span.start}
    class:collapsed={now.collapsed}
    class:filling={now.maximized}
    style:width={now.maximized ? null : `${now.collapsed ? STRIP_WIDTH : widthOf(s)}px`}
    role="presentation"
    onpointerdown={(e) => design && !shown.inherited && pick(e, s.span.start)}
  >
    {#if design && shown.inherited}<span class="from">from app</span>{/if}
    {#if s.props.collapsible || s.props.maximizable}
      <!-- The person folds it to this button and back, and lets it fill the window. -->
      <div class="sidebar-buttons">
        {#if s.props.maximizable && !now.collapsed}
          <button type="button" disabled={design} title={now.maximized ? "Give the window back" : "Fill the window"} onclick={() => setState(s, { maximized: !now.maximized })}
            >{now.maximized ? "\u2921" : "\u2922"}</button
          >
        {/if}
        {#if s.props.collapsible}
          <button type="button" disabled={design} title={now.collapsed ? "Show the sidebar" : "Fold the sidebar away"} onclick={() => setState(s, { collapsed: !now.collapsed, maximized: false })}
            >{(s.props.side === "left") !== now.collapsed ? "\u2039" : "\u203A"}</button
          >
        {/if}
      </div>
    {/if}
    {#if page && !now.collapsed}
    <GridBox
      {preview}
      {page}
      {data}
      design={design && !shown.inherited}
      widgets={s.widgets}
      columns={s.props.columns}
      container={s.span.start}
      {onnavigate}
    />
    {/if}
  </div>
{/snippet}

<div class="frame">
  {#if frame.menu}
    {@const shown = frame.menu}
    {@const editable = !shown.inherited}
    <div
      class="menubar"
      class:inherited={design && shown.inherited}
      class:picked={design && pageDesigner.selected === shown.value.span.start}
      role="menubar"
      tabindex="-1"
      onpointerdown={(e) => design && editable && pick(e, shown.value.span.start)}
    >
      {#each shown.value.items as item (item.span.start)}
        {#if item.props.kind === "separator"}
          <span class="vsep"></span>
        {:else}
          {@const open = openMenu === item.span.start || (design && containsPicked(item) && item.items.length > 0)}
          <div class="top">
            <button
              class:open
              class:picked={design && pageDesigner.selected === item.span.start}
              onpointerdown={(e) => {
                if (design && editable) pick(e, item.span.start);
              }}
              title={design && !editable ? "From the app: double-click to edit it in the app" : undefined}
              onclick={(e) => {
                if (item.items.length > 0) openMenu = openMenu === item.span.start ? null : item.span.start;
                else run(e, item, editable);
              }}
              ondblclick={(e) => codeOf(e, item, editable)}>{item.props.caption ? tr(preview, item.props.caption) : ""}</button
            >
            {#if open && item.items.length > 0}
              <div class="dropdown">{@render entries(item.items, editable)}</div>
            {/if}
          </div>
        {/if}
      {/each}
      {#if design && shown.inherited}<span class="from">from app</span>{/if}
    </div>
  {/if}

  {#if frame.toolbar}
    {@const shown = frame.toolbar}
    <div
      class="toolbar"
      class:inherited={design && shown.inherited}
      class:picked={design && pageDesigner.selected === shown.value.span.start}
      role="toolbar"
      tabindex="-1"
      onpointerdown={(e) => design && !shown.inherited && pick(e, shown.value.span.start)}
    >
      {#each shown.value.items as item (item.span.start)}
        {@render button(item, !shown.inherited, false)}
      {/each}
      {#if design && shown.inherited}<span class="from">from app</span>{/if}
    </div>
  {/if}

  {#if frame.ribbon}
    {@const shown = frame.ribbon}
    {@const editable = !shown.inherited}
    {@const r = shown.value}
    {@const tabs = r.tabs}
    {@const now = ribbonNow(r)}
    {@const active = now.tab === null ? undefined : tabs[now.tab - 1]}
    <div
      class="ribbon"
      class:inherited={design && shown.inherited}
      class:picked={design && pageDesigner.selected === shown.value.span.start}
      role="presentation"
      onpointerdown={(e) => design && editable && pick(e, shown.value.span.start)}
    >
      <div class="ribbon-tabs" role="presentation" ondblclick={() => !design && r.collapsible && setRibbon(r, { collapsed: !now.collapsed })}>
        {#each tabs as tab, i (tab.span.start)}
          {#if shows(tab.name)}
            <button
              class:active={tab === active}
              class:picked={design && pageDesigner.selected === tab.span.start}
              title={!design && r.collapsible ? "Double-click to fold or unfold the ribbon" : undefined}
              onpointerdown={(e) => {
                // Opening a tab of a folded ribbon unfolds it.
                setRibbon(r, { tab: i + 1, collapsed: false });
                if (design && editable) pick(e, tab.span.start);
              }}>{tr(preview, tab.title)}</button
            >
          {/if}
        {/each}
        {#if design && shown.inherited}<span class="from">from app</span>{/if}
      </div>
      {#if active && !now.collapsed}
        <div class="ribbon-body">
          {#each active.groups.filter((g) => shows(g.name)) as group (group.span.start)}
            <div
              class="ribbon-group"
              class:picked={design && pageDesigner.selected === group.span.start}
              role="group"
              onpointerdown={(e) => design && editable && pick(e, group.span.start)}
            >
              <div class="ribbon-buttons">
                {#each group.items as item (item.span.start)}
                  {@render button(item, editable, true)}
                {/each}
              </div>
              <div class="ribbon-title">{tr(preview, group.title)}</div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  <div class="middle">
    {#if frame.left && (!filling || filling === frame.left.value)}
      {@const now = stateOf(frame.left.value)}
      {@render sidebar(frame.left)}
      {#if !design && frame.left.value.props.resizable && !now.collapsed && !now.maximized}<Splitter axis="x" label="Width of the left sidebar" onmove={(d) => resizeSidebar(frame.left, d)} />{/if}
    {/if}
    <div class="body" class:hidden={filling !== null}>{@render children()}</div>
    {#if frame.right && (!filling || filling === frame.right.value)}
      {@const now = stateOf(frame.right.value)}
      {#if !design && frame.right.value.props.resizable && !now.collapsed && !now.maximized}<Splitter axis="x" label="Width of the right sidebar" onmove={(d) => resizeSidebar(frame.right, -d)} />{/if}
      {@render sidebar(frame.right)}
    {/if}
  </div>

  {#if frame.footer}
    {@const shown = frame.footer}
    {@const f = shown.value}
    <div
      class="footer"
      class:inherited={design && shown.inherited}
      class:picked={design && pageDesigner.selected === f.span.start}
      role="contentinfo"
      onpointerdown={(e) => design && !shown.inherited && pick(e, f.span.start)}
    >
      {#if f.status !== null && f.statusSpan}
        {@const statusId = f.statusSpan.start}
        <span
          class="status"
          class:picked={design && pageDesigner.selected === statusId}
          role="status"
          onpointerdown={(e) => design && !shown.inherited && pick(e, statusId)}>{tr(preview, f.status)}</span
        >
      {/if}
      <div class="footer-grid">
        {#if page}
        <GridBox
          {preview}
          {page}
          {data}
          design={design && !shown.inherited}
          widgets={f.widgets}
          columns={f.columns}
          container={f.span.start}
          fixedRows={1}
          {onnavigate}
        />
        {/if}
      </div>
      {#if design && shown.inherited}<span class="from">from app</span>{/if}
    </div>
  {/if}

  {#if regionDrag}
    {#each DOCKS.filter((d) => regionDrag.docks.includes(d)) as dock (dock)}
      <div class="dock {dock}" class:active={pageDesigner.dock === dock} data-dock={dock}>
        {regionDrag.label} here
      </div>
    {/each}
  {/if}
</div>

<style>
  .frame {
    position: relative;
    display: flex;
    flex-direction: column;
    /* Fills a flex parent (the design canvas); a percentage height would
       resolve against the whole window, title bar included. */
    flex: 1;
    /* May be smaller than its content: the window body scrolls instead. */
    min-height: 0;
  }

  .inherited {
    opacity: 0.55;
    position: relative;
  }

  /* An inherited menu bar fades its own buttons, not the menu that opens from it. */
  .menubar.inherited {
    opacity: 1;
  }

  .menubar.inherited > .top > button,
  .menubar.inherited > .vsep {
    opacity: 0.55;
  }

  .from {
    margin-left: auto;
    font-size: 10px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 0 6px;
  }

  .picked {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  /* Menu bar */

  .menubar {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px 6px;
    background: var(--frame);
    color: var(--frame-text);
    border-bottom: var(--border-width) solid var(--border);
  }

  .top {
    position: relative;
  }

  .menubar button,
  .entry button {
    border: none;
    background: transparent;
    color: inherit;
    border-radius: var(--radius-button);
    padding: calc(var(--padding) - 1px) calc(var(--padding) * 2.5);
    font-size: var(--font-size);
  }

  .menubar .top > button:hover,
  .menubar .top > button.open {
    background: var(--menu-hover-bg, var(--accent-soft));
  }

  .dropdown,
  .flyout {
    position: absolute;
    z-index: 20;
    min-width: 180px;
    background: var(--menu-bg);
    color: var(--menu-text);
    border: var(--border-width) solid var(--menu-border);
    border-radius: var(--menu-radius, var(--radius-panel));
    box-shadow: var(--shadow);
    padding: 4px;
  }

  .dropdown {
    top: 100%;
    left: 0;
  }

  .entry {
    position: relative;
  }

  .entry > button {
    display: flex;
    width: 100%;
    gap: 12px;
    text-align: left;
  }

  .entry > button:hover {
    background: var(--menu-hover-bg, var(--accent-soft));
    color: var(--menu-hover-text, inherit);
  }

  .entry kbd {
    margin-left: auto;
    font-family: var(--mono);
    font-size: var(--font-size-small);
    color: var(--text-muted);
  }

  .arrow {
    margin-left: auto;
  }

  .flyout {
    display: none;
    top: -4px;
    left: 100%;
  }

  .entry.sub:hover > .flyout,
  .entry.show > .flyout {
    display: block;
  }

  .separator-line {
    border-top: var(--border-width) solid var(--border-strong);
    margin: 4px 6px;
    padding-top: 2px;
  }

  /* Toolbar and ribbon buttons */

  .toolbar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 6px;
    background: var(--frame);
    color: var(--frame-text);
    border-bottom: var(--border-width) solid var(--border);
  }

  .tool {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: calc(var(--padding) - 1px) calc(var(--padding) * 2);
    font-size: var(--font-size-small);
  }

  .tool.large {
    flex-direction: column;
    padding: 4px 10px;
  }

  .tool.large .icon {
    width: 24px;
    height: 24px;
    font-size: 13px;
  }

  .icon {
    display: inline-grid;
    place-items: center;
    width: 16px;
    height: 16px;
    border-radius: 3px;
    background: var(--accent-soft);
    font-size: 10px;
    font-weight: 700;
  }

  .vsep {
    width: 1px;
    align-self: stretch;
    background: var(--border-strong);
    margin: 2px 4px;
  }

  .ribbon {
    border-bottom: var(--border-width) solid var(--border);
  }

  .ribbon-tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px 6px 0;
    background: var(--frame);
    color: var(--frame-text);
  }

  .ribbon-tabs button {
    border: none;
    border-radius: var(--radius-button) var(--radius-button) 0 0;
    background: transparent;
    color: inherit;
    padding: calc(var(--padding) - 1px) calc(var(--padding) * 3);
    font-size: var(--font-size-small);
  }

  .ribbon-tabs button.active {
    background: var(--panel);
  }

  .ribbon-body {
    display: flex;
    gap: 4px;
    padding: 6px;
    min-height: 70px;
  }

  .ribbon-group {
    display: flex;
    flex-direction: column;
    border-right: var(--border-width) solid var(--border);
    padding: 0 6px;
  }

  .ribbon-buttons {
    display: flex;
    gap: 4px;
    align-items: flex-start;
    flex: 1;
  }

  .ribbon-title {
    text-align: center;
    font-size: var(--font-size-small);
    color: var(--text-muted);
    margin-top: 4px;
  }

  /* Sidebars, body, footer */

  .middle {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  .body {
    flex: 1;
    min-width: 0;
    /* Passes the height on, so a fluid window fills it. */
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .sidebar {
    flex: none;
    padding: calc(var(--padding) * 2);
    background: var(--frame);
    color: var(--frame-text);
  }

  .sidebar.left {
    border-right: var(--border-width) solid var(--border);
  }

  .sidebar.right {
    border-left: var(--border-width) solid var(--border);
  }

  .sidebar.collapsed {
    padding: 4px 2px;
  }

  /* Filling the window: the window's own parts make room. */
  .sidebar.filling {
    flex: 1;
  }

  .body.hidden {
    display: none;
  }

  .sidebar-buttons {
    display: flex;
    justify-content: flex-end;
    gap: 2px;
    margin-bottom: 4px;
  }

  .sidebar.right .sidebar-buttons {
    justify-content: flex-start;
  }

  .sidebar-buttons button {
    padding: 0 6px;
    line-height: 1.4;
  }

  .footer {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 2px 8px;
    border-top: var(--border-width) solid var(--border);
    background: var(--frame);
    color: var(--frame-text);
    font-size: var(--font-size-small);
  }

  .status {
    flex: none;
    color: var(--text-muted);
    padding: 2px 4px;
  }

  .footer-grid {
    flex: 1;
    min-width: 0;
  }

  /* Dock zones while a region is dragged */

  .dock {
    position: absolute;
    z-index: 30;
    display: grid;
    place-items: center;
    border: 2px dashed var(--accent);
    background: rgb(80 140 255 / 0.12);
    color: var(--text);
    font-size: 12px;
    border-radius: var(--radius);
  }

  .dock.active {
    background: rgb(80 140 255 / 0.35);
  }

  .dock.top {
    top: 0;
    left: 0;
    right: 0;
    height: 40px;
  }

  .dock.bottom {
    bottom: 0;
    left: 0;
    right: 0;
    height: 40px;
  }

  .dock.left {
    top: 44px;
    bottom: 44px;
    left: 0;
    width: 90px;
  }

  .dock.right {
    top: 44px;
    bottom: 44px;
    right: 0;
    width: 90px;
  }
</style>
