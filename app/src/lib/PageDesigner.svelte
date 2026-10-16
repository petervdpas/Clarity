<script lang="ts">
  import { onMount } from "svelte";
  import FrameView from "./FrameView.svelte";
  import GridBox from "./GridBox.svelte";
  import PageOutline from "./PageOutline.svelte";
  import PageProperties from "./PageProperties.svelte";
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
  import { describeType } from "./model";
  import { pageDesigner, type PageDragSource, type Placement } from "./pageDesigner.svelte";
  import { appAsPage, locateInnerPage, locateWidget, pageData, widgetForField, widgetInfo, withProps } from "./pages";

  let {
    preview,
    page,
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
    { key: "basicInput", name: "Basic input" },
    { key: "containers", name: "Containers" },
    { key: "data", name: "Data" },
    { key: "graphics", name: "Graphics" },
  ];

  let canvas: HTMLElement | undefined = $state();

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
        // A field arrives as a caption label plus its input widget when there
        // is room for both, in one edit.
        const labelWidth = 2;
        const withLabel = source.widget !== "checkbox" && source.widget !== "table" && at.columns - at.col + 1 > labelWidth;
        const col = withLabel ? at.col + labelWidth : at.col;
        const input: Edit = {
          op: "createWidget",
          container: at.container,
          kind: source.widget,
          col,
          row: at.row,
          width: Math.min(source.width, at.columns - col + 1),
          caption: null,
          bind: source.bind,
        };
        const label: Edit = {
          op: "createWidget",
          container: at.container,
          kind: "label",
          col: at.col,
          row: at.row,
          width: labelWidth,
          caption: source.caption,
          bind: null,
        };
        await apply(withLabel ? { op: "batch", edits: [label, input] } : input);
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
        await apply({
          op: "updateWidget",
          item: source.item,
          container: null,
          props: withProps(p, { width: at.width, height: at.height }),
        });
        break;
      }
    }
  }

  function pressPalette(e: PointerEvent, info: WidgetInfo) {
    pageDesigner.press(e, { kind: "new", widget: info.kind, width: info.width, height: info.height, label: info.name });
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
    {@const entity = entities.find((x) => x.name.name === decl.entity.name)}
    <h4>Data <code>{decl.name.name}: {decl.list ? `list<${decl.entity.name}>` : decl.entity.name}</code></h4>
    <div class="tiles">
      {#if decl.list}
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

<div class="designer">
  <div class="stage">

    <div class="window">
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
      <div class="client" bind:this={canvas} onpointerdown={() => (pageDesigner.selected = null)} role="presentation">
        <FrameView {preview} {page} {frame} {data} design={true} onnavigate={() => {}}>
          <div class="body">
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
    </div>
  </div>

  <aside>
    <PageProperties {preview} {page} {app} onedit={edit} subject="window" />
    <PageProperties {preview} {page} {app} onedit={edit} subject="picked" />

    {#if pageDesigner.error}
      <div class="error" role="alert">
        {pageDesigner.error}
        <button onclick={() => (pageDesigner.error = null)}>Dismiss</button>
      </div>
    {/if}
  </aside>
</div>

{#if pageDesigner.drag}
  <div class="drag-label" style:left="{pageDesigner.pointer.x + 12}px" style:top="{pageDesigner.pointer.y + 12}px">
    {pageDesigner.drag.label}
  </div>
{/if}

<style>
  .designer {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 280px;
    gap: 16px;
    align-items: start;
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


  .window {
    background: var(--panel);
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.08);
    overflow: hidden;
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

  .body {
    padding: 16px;
  }

  .main-hint {
    margin: 40px auto;
    max-width: 420px;
    text-align: center;
    color: var(--text-muted);
    font-size: 13px;
  }

  .client {
    min-height: 360px;
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
