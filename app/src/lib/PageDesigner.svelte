<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { commandAction, commandHandler, freePartName, handlerName, wireCommand, wireEvent } from "./codeParts";
  import FrameView from "./FrameView.svelte";
  import GridBox from "./GridBox.svelte";
  import ShownWindow from "./ShownWindow.svelte";
  import PageOutline from "./PageOutline.svelte";
  import PageProperties from "./PageProperties.svelte";
  import CodePanel from "./CodePanel.svelte";
  import Splitter from "./Splitter.svelte";
  import { layout } from "./layout.svelte";
  import PartTile from "./PartTile.svelte";
  import { portal } from "./portal";
  import { applyEdit } from "./applyEdit";
  import type { Category } from "./generated/Category";
  import type { App } from "./generated/App";
  import type { Dock } from "./generated/Dock";
  import type { Edit } from "./generated/Edit";
  import type { RegionInfo } from "./generated/RegionInfo";
  import type { RegionKind } from "./generated/RegionKind";
  import type { Page } from "./generated/Page";
  import type { Preview } from "./generated/Preview";
  import type { SpecFile } from "./generated/SpecFile";
  import type { WidgetInfo } from "./generated/WidgetInfo";
  import { NO_FRAME, findApp, findFrameElement, shownFrame } from "./frame";
  import { describeType, themeArea, themeOf } from "./model";
  import { dataEntity, dataType, isList, valueField } from "./windowData";
  import type { WidgetKind } from "./generated/WidgetKind";
  import { pageDesigner, type PageDragSource, type Placement } from "./pageDesignerStore.svelte";
  import {
    CONTENT_PADDING,
    windowSize,
    appAsPage,
    resizeEdits,
    findPage,
    locateInnerPage,
    locateWidget,
    allParts,
    pageData,
    widgetForField,
    widgetInfo,
    withProps,
  } from "./pages";

  let {
    preview: savedPreview,
    page: savedPage,
    file,
    onapplied,
    app = null,
  }: {
    preview: Preview;
    /** The page, or the app shown as a page (see `appAsPage`). */
    page: Page;
    file: SpecFile;
    onapplied: (preview: Preview) => void;
    /** Set when designing the app: only its frame is edited. */
    app?: App | null;
  } = $props();

  const CATEGORIES: { key: Category; name: string }[] = [
    { key: "text", name: "Text" },
    { key: "basicInput", name: "Basic input" },
    { key: "containers", name: "Containers" },
    { key: "data", name: "Data" },
    { key: "graphics", name: "Graphics" },
  ];

  let canvas: HTMLElement | undefined = $state();

  // While the Design text is being typed, the designer shows the project as
  // that text would make it.
  const preview = $derived(pageDesigner.draft ?? savedPreview);
  // The canvas shows the project's theme, as the application will.
  const area = $derived(themeArea(themeOf(preview)));
  const page = $derived.by(() => {
    const draft = pageDesigner.draft;
    if (!draft) return savedPage;
    if (app) {
      const draftApp = findApp(draft);
      return draftApp ? appAsPage(draftApp) : savedPage;
    }
    return findPage(draft, savedPage.name.name) ?? savedPage;
  });

  // The size a window's edge is being dragged to, kept until the window has
  // it; null while none is.
  let sizing = $state<{ width: number; height: number } | null>(null);
  const shownSize = $derived(sizing ?? windowSize(page));

  async function resize(size: { width: number; height: number }, done: boolean) {
    sizing = size;
    if (!done) return;
    const now = windowSize(page);
    if ((size.width !== now.width || size.height !== now.height) && !pageDesigner.saving) {
      await apply({ op: "setSize", page: page.span.start, width: size.width, height: size.height });
    }
    sizing = null;
  }

  const projectApp = $derived(findApp(preview));
  // The app's data is visible to the widgets in its sidebars and footer.
  const data = $derived({ ...(projectApp ? pageData(preview, appAsPage(projectApp)) : {}), ...pageData(preview, page) });
  /** Windows dock regions; views and dialogs do not. */
  const docks = $derived(app !== null || page.kind === "window");
  const frame = $derived(
    app !== null
      ? shownFrame(null, page.frame)
      : page.kind === "dialog"
        ? NO_FRAME
        : shownFrame(projectApp, page.kind === "window" ? page.frame : null),
  );
  const entities = $derived(preview.project.files.flatMap((f) => f.entities));

  async function apply(edit: Edit): Promise<void> {
    if (pageDesigner.draft) {
      pageDesigner.error = "The window shows unsaved Design text; save or revert it before changing the window here.";
      return;
    }
    pageDesigner.saving = true;
    pageDesigner.error = null;
    try {
      const result = await applyEdit(file, edit);
      onapplied(result.preview);
      pageDesigner.selected = result.focus;
    } catch (err) {
      pageDesigner.error = String(err);
    } finally {
      pageDesigner.saving = false;
    }
  }

  // A part double-clicked on the window opens the code of its main event (its
  // first in the catalog: click for a button, draw for a canvas), writing and
  // wiring the handler first when there is none, as an IDE does.
  let handledPartCode: object | null = null;
  $effect(() => {
    const request = pageDesigner.partCode;
    if (!request || request === handledPartCode) return;
    handledPartCode = request;
    untrack(() => openPartCode(request.item));
  });

  function openPartCode(item: number) {
    const owner = app ? app.name.name : page.name.name;
    const element = findFrameElement(page.frame, item);
    if (element?.kind === "command") {
      const props = element.command.props;
      const does = commandAction(props);
      if (props.handler) pageDesigner.showCode(props.handler);
      else if (does) pageDesigner.error = `${props.caption ?? "It"} ${does}: it runs no code. To give it code, wire a handler under Events in its properties.`;
      else {
        const handler = commandHandler(props);
        edit(wireCommand({ item, props, owner, handler }));
        pageDesigner.showCode(handler);
      }
      return;
    }
    const location = locateWidget(preview, page, item);
    if (!location) return;
    const p = location.widget.props;
    const info = widgetInfo(preview, p.kind);
    const event = info.events[0];
    const wired = event ? p.handlers.find((h) => h.event === event) : undefined;
    if (!event || wired) {
      pageDesigner.showCode(wired ? handlerName(wired.handler) : null);
      return;
    }
    // Every part has a name; one written without gets it now, with its handler.
    const name = p.name ?? freePartName(allParts(page).flatMap((w) => (w.props.name ? [w.props.name] : [])), info.keyword);
    const handler = `${name}_${event}`;
    edit(wireEvent({ item, props: { ...p, name }, events: info.events, owner, event, handler }));
    pageDesigner.showCode(handler);
  }

  function edit(e: Edit) {
    if (!pageDesigner.saving) apply(e);
  }

  async function drop(source: PageDragSource, at: Placement) {
    if (pageDesigner.saving) return;
    switch (source.kind) {
      case "new":
        await apply({
          op: "createWidget",
          container: at.container,
          kind: source.widget,
          col: at.col,
          row: at.row,
          width: at.width === source.width ? null : at.width,
          caption: null,
          bind: null,
        });
        break;
      case "field": {
        // A list arrives as a table. A field arrives as its input, with a
        // label for it and its help text when there is room; the backend
        // lays these out the same way the form template does.
        // A list or a plain value arrives as one part bound to it by name.
        const room = at.columns - at.col + 1;
        if (source.bind && !source.bind.includes(".")) {
          await apply({
            op: "createWidget",
            container: at.container,
            kind: source.widget,
            col: at.col,
            row: at.row,
            width: Math.min(source.width, room),
            caption: null,
            bind: source.bind,
          });
        } else {
          await apply({ op: "placeField", container: at.container, col: at.col, row: at.row, room, bind: source.bind });
        }
        break;
      }
      case "move": {
        const p = source.props;
        if (at.container === source.from && at.col === p.col && at.row === p.row) return;
        await apply({
          op: "updateWidget",
          item: source.item,
          container: at.container === source.from ? null : at.container,
          props: withProps(p, { col: at.col, row: at.row }),
        });
        break;
      }
      case "resize": {
        const p = source.props;
        if (at.width === p.width && at.height === p.height) return;
        // A container's parts scale across with it, in the same edit.
        const found = locateWidget(preview, page, source.item);
        const edits = found
          ? resizeEdits(found.widget, at.width, at.height)
          : [{ op: "updateWidget" as const, item: source.item, container: null, props: withProps(p, { width: at.width, height: at.height }) }];
        await apply(edits.length === 1 ? edits[0] : { op: "batch", edits });
        break;
      }
    }
  }

  function pressPalette(e: PointerEvent, info: WidgetInfo) {
    pageDesigner.press(e, { kind: "new", widget: info.kind, width: info.width, height: info.height, label: info.name });
  }

  /** A plain value arrives as one part bound to it. */
  function pressValue(e: PointerEvent, dataName: string, kind: WidgetKind) {
    const info = widgetInfo(preview, kind);
    pageDesigner.press(e, {
      kind: "field",
      widget: kind,
      bind: dataName,
      caption: dataName,
      width: info.width,
      height: info.height,
      label: dataName,
    });
  }

  function pressField(e: PointerEvent, dataName: string, fieldName: string, label: string, list: boolean) {
    if (list) {
      const table = widgetInfo(preview, "table");
      pageDesigner.press(e, {
        kind: "field",
        widget: "table",
        bind: dataName,
        caption: label,
        width: table.width,
        height: table.height,
        label: `Table of ${dataName}`,
      });
      return;
    }
    const entry = data[dataName];
    const field = entry?.entity.fields.find((f) => f.name.name === fieldName);
    if (!field) return;
    const kind = widgetForField(preview, field);
    const info = widgetInfo(preview, kind);
    pageDesigner.press(e, {
      kind: "field",
      widget: kind,
      bind: `${dataName}.${fieldName}`,
      caption: label,
      width: info.width,
      height: info.height,
      label,
    });
  }

  function dock(region: RegionKind, at: Dock) {
    const side = at === "left" || at === "right" ? at : null;
    edit({ op: "addRegion", owner: page.span.start, region, side });
  }

  function pressRegion(e: PointerEvent, info: RegionInfo) {
    pageDesigner.press(e, { kind: "region", region: info.region, docks: info.docks, label: info.name });
  }

  function onkeydown(e: KeyboardEvent) {
    const target = e.target as HTMLElement | null;
    if (target?.closest("input, textarea, select")) return;
    const selected = pageDesigner.selected;
    if ((e.key === "Delete" || e.key === "Backspace") && selected !== null) {
      if (locateWidget(preview, page, selected) || locateInnerPage(page, selected) || findFrameElement(page.frame, selected)) {
        e.preventDefault();
        edit({ op: "remove", item: selected });
      }
    } else if (e.key === "Escape") {
      pageDesigner.selected = null;
    }
  }

  // Drop a selection that no longer exists after an outside edit.
  $effect(() => {
    const s = pageDesigner.selected;
    const gone = !locateWidget(preview, page, s ?? -1) && !locateInnerPage(page, s ?? -1) && !findFrameElement(page.frame, s ?? -1);
    if (s !== null && s !== page.span.start && gone) {
      pageDesigner.selected = null;
    }
  });

  onMount(() => {
    if (!canvas) return;
    return pageDesigner.attach(canvas, drop, dock);
  });
</script>

<svelte:window {onkeydown} />

<!-- Stays in place while its children move to the sidebar (see portal.ts). -->
<div hidden>
<section class="parts" use:portal={"design-parts"}>
  <h3>Parts</h3>
  {#if docks}
    <h4>Frame</h4>
    <div class="tiles">
      {#each preview.frame as info (info.region)}
        <PartTile kind={info.region} name={info.name} title="{info.holds}. Drag onto an edge of the window." onpointerdown={(e) => pressRegion(e, info)} />
      {/each}
    </div>
  {/if}
  {#each CATEGORIES as c (c.key)}
    <h4>{c.name}</h4>
    <div class="tiles">
      {#each preview.catalog.filter((w) => w.category === c.key) as info (info.kind)}
        <PartTile kind={info.kind} name={info.name} onpointerdown={(e) => pressPalette(e, info)} />
      {/each}
    </div>
  {/each}

  {#each page.data as decl (decl.span.start)}
    {@const entity = entities.find((x) => x.name.name === dataEntity(decl))}
    {@const value = valueField(decl)}
    <h4>Data <code>{decl.name.name}: {dataType(decl)}</code></h4>
    <div class="tiles">
      {#if value}
        <PartTile
          kind={widgetForField(preview, value)}
          name={decl.name.name}
          detail={dataType(decl)}
          title="Drag onto the window: a part bound to this value"
          onpointerdown={(e) => pressValue(e, decl.name.name, widgetForField(preview, value))}
        />
      {:else if isList(decl)}
        <PartTile kind="table" name="Table of {decl.name.name}" onpointerdown={(e) => pressField(e, decl.name.name, "", decl.name.name, true)} />
      {:else}
        {#each (entity?.fields ?? []).filter((f) => f.type.kind !== "list") as field (field.name.name)}
          <PartTile
            kind={widgetForField(preview, field)}
            name={field.label}
            detail={describeType(field)}
            title="Drag onto the window: a label and a bound input"
            onpointerdown={(e) => pressField(e, decl.name.name, field.name.name, field.label, false)}
          />
        {/each}
      {/if}
    </div>
  {:else}
    <p class="hint">Declare data in the window properties to drag bound fields onto the window.</p>
  {/each}
</section>

<div use:portal={"design-outline"}>
  <PageOutline {preview} {page} {frame} rootName={app ? "App" : null} onedit={edit} />
</div>
</div>

<div class="designer" style:grid-template-columns="minmax(0, 1fr) auto {layout.folded.properties ? 'auto' : layout.sizes.properties + 'px'}">
  <div class="stage">
    <ShownWindow {area} clientWidth={shownSize.width} clientHeight={shownSize.height} onresize={app ? undefined : resize}>
      <div
        class="titlebar"
        class:selected={pageDesigner.selected === null}
        role="button"
        tabindex="-1"
        onpointerdown={() => (pageDesigner.selected = null)}
      >
        <span>{page.title ?? page.name.name}</span>
        <code>{app ? "app " + app.name.name : page.name.name + ": " + page.kind + " | " + page.columns + " columns"}</code>
      </div>
      <div
        class="client"
        style:height="{shownSize.height}px"
        bind:this={canvas}
        onpointerdown={() => (pageDesigner.selected = null)}
        role="presentation"
      >
        <FrameView {preview} {page} {frame} {data} design={true} onnavigate={() => {}}>
          <div class="body" style:padding="{CONTENT_PADDING}px">
            {#if app}
              <p class="main-hint">
                Each window's content goes here{app.main ? "; the app starts with " + app.main.name : ""}. Drag menus,
                toolbars, a ribbon, sidebars and a footer onto the edges: every window shows them unless it has its own.
              </p>
            {:else}
              <GridBox
                {preview}
                {page}
                {data}
                design={true}
                widgets={page.widgets}
                columns={page.columns}
                container={page.span.start}
                onnavigate={() => {}}
              />
            {/if}
          </div>
        </FrameView>
      </div>
    </ShownWindow>
  </div>

  {#if layout.folded.properties}
    <span></span>
  {:else}
    <Splitter axis="x" label="Width of the properties" onmove={(d) => layout.resize("properties", -d)} />
  {/if}
  <aside>
    {#if layout.folded.properties}
      <button class="dock" title="Show the properties" onclick={() => layout.toggle("properties")}>&lsaquo; Properties</button>
    {:else}
      <button class="undock" title="Fold the properties away" onclick={() => layout.toggle("properties")}>Hide &rsaquo;</button>
      <PageProperties {preview} {page} {app} onedit={edit} subject="window" />
      <PageProperties {preview} {page} {app} onedit={edit} subject="picked" />
    {/if}

    {#if pageDesigner.error}
      <div class="error" role="alert">
        {pageDesigner.error}
        <button onclick={() => (pageDesigner.error = null)}>Dismiss</button>
      </div>
    {/if}
  </aside>
</div>

<div class="code-anchor">
  {#if pageDesigner.code.open}
    <div use:portal={"code-drawer"}>
      <CodePanel
        {preview}
        {page}
        {file}
        owner={app ? app.name.name : savedPage.name.name}
        item={app ? app.span.start : savedPage.span.start}
        {onapplied}
        onclose={() => (pageDesigner.code = { ...pageDesigner.code, open: false, focus: null })}
      />
    </div>
  {/if}
</div>

{#if pageDesigner.drag}
  <div class="drag-label" style:left="{pageDesigner.pointer.x + 12}px" style:top="{pageDesigner.pointer.y + 12}px">
    {pageDesigner.drag.label}
  </div>
{/if}

<style>
  .designer {
    display: grid;

    gap: 16px;
    align-items: start;
  }

  .designer > :global(.splitter) {
    align-self: stretch;
  }

  .stage {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
  }

  .parts {
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  h4 {
    margin: 6px 0 0;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }

  h4 code {
    font-family: var(--mono);
    text-transform: none;
    letter-spacing: 0;
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 6px;
  }



  .titlebar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    background: var(--titlebar);
    color: var(--titlebar-text);
    padding: 8px 14px;
    font-weight: 600;
  }

  .titlebar.selected {
    box-shadow: inset 0 -2px 0 var(--brand);
  }

  .titlebar code {
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 400;
    opacity: 0.7;
  }

  /* As the running window's client: its grid fills the height. */
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: auto;
  }


  .main-hint {
    margin: 40px auto;
    max-width: 420px;
    text-align: center;
    color: var(--text-muted);
    font-size: 13px;
  }

  .client {
    display: flex;
    flex-direction: column;
    user-select: none;
  }

  aside {
    display: flex;
    flex-direction: column;
    gap: 12px;
    position: sticky;
    top: 0;
  }

  .undock {
    align-self: flex-end;
    margin-bottom: -8px;
    padding: 2px 8px;
    font-size: 11px;
    border: none;
    background: none;
    color: var(--text-muted);
  }

  .undock:hover,
  .dock:hover {
    color: var(--text);
  }

  .dock {
    writing-mode: vertical-rl;
    padding: 10px 4px;
    font-size: 12px;
    color: var(--text-muted);
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
  }

  section {
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  h3 {
    margin: 0;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .error {
    background: var(--error-soft);
    color: var(--error);
    border-radius: 8px;
    padding: 10px 12px;
    font-size: 13px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: flex-start;
  }

  .drag-label {
    position: fixed;
    pointer-events: none;
    background: var(--accent);
    color: #fff;
    padding: 3px 10px;
    border-radius: var(--radius);
    font-size: 12px;
    box-shadow: 0 4px 12px rgb(0 0 0 / 0.25);
    z-index: 10;
  }
</style>
