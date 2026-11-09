<script lang="ts">
  import { designLog } from "./designLog";
  import { onMount, untrack } from "svelte";
  import { commandAction, commandHandler, eventHandler, freePartName, handlerName, wireCommand, wireEvent } from "./codeParts";
  import ShownWindow from "./ShownWindow.svelte";
  import ClarityWindow from "./ClarityWindow.svelte";
  import DesignLayer from "./DesignLayer.svelte";
  import { chosenOf, type Layout } from "./clarityWindow";
  import { languageChoice } from "@clarity/ui/screen/i18n.svelte.ts";
  import PageOutline from "./PageOutline.svelte";
  import PageProperties from "./PageProperties.svelte";
  import CodePanel from "./CodePanel.svelte";
  import Splitter from "@clarity/ui/parts/Splitter.svelte";
  import { layout } from "./layout.svelte";
  import PartTile from "./PartTile.svelte";
  import { portal } from "./portal";
  import { commandsOf, grown, nudged } from "./designLayer";
  import { applyEdit } from "./applyEdit";
  import type { Category } from "@clarity/ui/generated/Category.ts";
  import type { PartDecl } from "@clarity/ui/generated/PartDecl.ts";
  import type { App } from "@clarity/ui/generated/App.ts";
  import type { Dock } from "@clarity/ui/generated/Dock.ts";
  import type { Edit } from "@clarity/ui/generated/Edit.ts";
  import type { RegionInfo } from "@clarity/ui/generated/RegionInfo.ts";
  import type { RegionKind } from "@clarity/ui/generated/RegionKind.ts";
  import type { Page } from "@clarity/ui/generated/Page.ts";
  import type { Widget } from "@clarity/ui/generated/Widget.ts";
  import type { Preview } from "@clarity/ui/generated/Preview.ts";
  import type { SpecFile } from "@clarity/ui/generated/SpecFile.ts";
  import type { Settings } from "@clarity/ui/generated/Settings.ts";
  import type { WidgetInfo } from "@clarity/ui/generated/WidgetInfo.ts";
  import { NO_FRAME, findApp, findFrameElement, shownFrame } from "@clarity/ui/screen/frame.ts";
  import { describeType, themeArea, themeOf, tr } from "@clarity/ui/screen/model.ts";
  import { dataEntity, dataType, isList, valueField } from "@clarity/ui/screen/windowData.ts";
  import type { WidgetKind } from "@clarity/ui/generated/WidgetKind.ts";
  import { pageDesigner, type PageDragSource, type Placement } from "./pageDesignerStore.svelte";
  import {
    windowSize,
    appAsPage,
    resizeEdits,
    findPage,
    locateInnerPage,
    locateWidget,
    locateKey,
    keyAt,
    placeOf,
    shownPage,
    type DesignKey,
    allParts,
    pageData,
    widgetForField,
    widgetInfo,
    withProps,
  } from "@clarity/ui/screen/pages.ts";

  let {
    preview: savedPreview,
    page: savedPage,
    file,
    onapplied,
    app = null,
    settings = null,
    onsavesettings = null,
  }: {
    preview: Preview;
    /** The page, or the app shown as a page (see `appAsPage`). */
    page: Page;
    file: SpecFile;
    onapplied: (preview: Preview) => void;
    /** Set when designing the app: only its frame is edited. */
    app?: App | null;
    /** The project file's settings (the app's version among them), and how to save them. */
    settings?: Settings | null;
    onsavesettings?: ((settings: Settings) => void) | null;
  } = $props();

  const CATEGORIES: { key: Category; name: string }[] = [
    { key: "text", name: "Text" },
    { key: "basicInput", name: "Basic input" },
    { key: "containers", name: "Containers" },
    { key: "data", name: "Data" },
    { key: "graphics", name: "Graphics" },
    { key: "media", name: "Media" },
    { key: "time", name: "Time" },
    { key: "tools", name: "Toolbox" },
  ];

  // A toolbox (`page X: toolbox`) holds its tools, buttons, labels, headings and lines; a window or a view holds no tools.
  function designingToolbox(): boolean {
    return page.kind === "toolbox";
  }
  const TOOLBOX_ALSO = ["button", "label", "heading", "separator"];
  function paletteHolds(category: Category): boolean {
    return designingToolbox() ? category === "tools" || category === "text" || category === "containers" || category === "basicInput" : category !== "tools";
  }
  function paletteOffers(kind: string): boolean {
    return !designingToolbox() || TOOLBOX_ALSO.includes(kind) || preview.catalog.some((w) => w.kind === kind && w.category === "tools");
  }

  let canvas: HTMLElement | undefined = $state();

  // While the Design text is being typed, the designer shows the project as
  // that text would make it: its lists, and the drawing (the draft).
  const preview = $derived(pageDesigner.draft ?? savedPreview);
  // The canvas shows the project's theme, as the application will.
  const area = $derived(themeArea(themeOf(preview)));
  // The window as written, or as the Design text being typed would make it.
  const ownPage = $derived.by(() => {
    const draft = pageDesigner.draft;
    if (!draft) return savedPage;
    if (app) {
      const draftApp = findApp(draft);
      return draftApp ? appAsPage(draftApp) : savedPage;
    }
    return findPage(draft, savedPage.name.name) ?? savedPage;
  });
  // A window's content shows its views' parts, designed in place; each thing is known by its key (shownPage).
  const shown = $derived(shownPage(preview, ownPage, file));
  const page = $derived(shown.page);
  // What is picked: its file, the window, view or app it is in, and its place there; else this window.
  // The inspector and the code show it in its own file, and its edits change that file.
  const picked = $derived(placeOf(preview, pageDesigner.selected));
  const inspected = $derived<{ file: SpecFile; page: Page; at: number | null }>(picked ?? { file, page: ownPage, at: null });

  // The inspector on the right: the picked part's (or the window's)
  // properties, its code, or the window as text, in tabs, as an IDE's object
  // inspector, one width for all tabs. The code tabs are as tall as the room they have.
  type InspectorTab = "properties" | "code" | "text";
  const INSPECTOR_TABS: { key: InspectorTab; name: string; title: string }[] = [
    { key: "properties", name: "Properties", title: "What the picked part or the window is" },
    { key: "code", name: "Code", title: "The code of the picked part, or of the window" },
    { key: "text", name: "Text", title: "The window's .cxaml file, as markup" },
  ];
  const inspectorTab = $derived<InspectorTab>(!pageDesigner.code.open ? "properties" : pageDesigner.code.view === "code" ? "code" : "text");
  const coding = $derived(inspectorTab !== "properties");
  /** The code fills the designer, the canvas set aside, while Code or Text is open. */
  let filling = $state(false);
  const filled = $derived(filling && coding && !layout.folded.properties);
  let codeOpened = $state(false);
  $effect(() => {
    if (coding) codeOpened = true;
  });

  function showTab(tab: InspectorTab) {
    pageDesigner.code = tab === "properties" ? { ...pageDesigner.code, open: false, focus: null } : { open: true, view: tab === "code" ? "code" : "design", focus: null };
  }

  // The code tabs reach down to the bottom of the room the designer has.
  let inspector: HTMLElement | undefined = $state();
  let inspectorHeight = $state(600);
  $effect(() => {
    const room = inspector?.closest("main");
    if (!inspector || !room) return;
    const measure = () => {
      if (inspector) inspectorHeight = Math.max(240, room.getBoundingClientRect().bottom - Math.max(inspector.getBoundingClientRect().top, room.getBoundingClientRect().top) - 12);
    };
    measure();
    const watch = new ResizeObserver(measure);
    watch.observe(room);
    room.addEventListener("scroll", measure);
    return () => {
      watch.disconnect();
      room.removeEventListener("scroll", measure);
    };
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
  /** What has fields: the entities, and the records (DESIGN 5.12) code builds and nothing stores. */
  const entities = $derived(preview.project.files.flatMap((f) => [...f.entities, ...f.records]));
  // Where Clarity.UI drew the window's parts and grids, for the design layer over it.
  let laid = $state<Layout | null>(null);
  // A popover or slide-out shows while it or a part in it is picked, as it opens when the program runs.
  const popovers = $derived(allParts(page).filter((w) => w.props.kind === "popover" || w.props.kind === "slideout"));
  /** Whether `key` is the part `w`, or a part or page in it. */
  function holds(w: Widget, key: DesignKey): boolean {
    return shown.keyOf(w) === key || w.children.some((c) => holds(c, key)) || w.pages.some((p) => shown.keyOf(p) === key || p.children.some((c) => holds(c, key)));
  }
  const openPopovers = $derived(
    popovers.filter((w) => pageDesigner.selected !== null && w.props.name && holds(w, pageDesigner.selected)).map((w) => w.props.name as string),
  );
  // The app's frame buttons a window shows: designed in the app, so known by the app's file.
  const appCommands = $derived.by(() => {
    if (app !== null || page.kind !== "window" || !projectApp) return new Set<DesignKey>();
    const appFile = preview.project.files.find((f) => f.apps.includes(projectApp));
    return new Set(appFile ? commandsOf(projectApp.frame, (t) => keyAt(preview, appFile, t.span.start)) : []);
  });
  // What the drawing shows beyond the saved window: the page each container shows, the draft, the popover picked.
  const showing = $derived({ pages: { ...pageDesigner.activeTabs }, draft: pageDesigner.draft !== null, open: openPopovers });
  const chosen = $derived(chosenOf(languageChoice.language, preview.project.locale.language ?? "en"));

  /** Makes `edit`, in the places of the file of `key` (this window's when null). */
  async function apply(edit: Edit, key: DesignKey | null = null): Promise<void> {
    if (pageDesigner.draft) {
      pageDesigner.error = "The window shows unsaved Design text; save or revert it before changing the window here.";
      return;
    }
    pageDesigner.saving = true;
    pageDesigner.error = null;
    try {
      // A view's part drawn in this window changes the view's own file.
      const target = key === null ? file : placeOf(preview, key)?.file;
      if (!target) throw new Error("what this changes is no longer in the project");
      const result = await applyEdit(target, edit);
      onapplied(result.preview);
      pageDesigner.selected = result.focus === null ? null : keyAt(result.preview, target, result.focus);
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

  function openPartCode(key: DesignKey) {
    const placed = placeOf(preview, key);
    if (!placed) return;
    const { page: itsPage, at: item } = placed;
    const owner = itsPage.name.name;
    const element = findFrameElement(itsPage.frame, item, itsPage.widgets);
    if (element?.kind === "command") {
      const props = element.command.props;
      const does = commandAction(props);
      if (props.handler) pageDesigner.showCode(props.handler);
      else if (does) pageDesigner.error = `${props.caption ?? "It"} ${does}: it runs no code. To give it code, wire a handler under Events in its properties.`;
      else {
        const handler = commandHandler(props);
        edit(wireCommand({ item, props, owner, handler }), key);
        pageDesigner.showCode(handler);
      }
      return;
    }
    const location = locateWidget(preview, itsPage, item);
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
    const name = p.name ?? freePartName(allParts(itsPage).flatMap((w) => (w.props.name ? [w.props.name] : [])), info.keyword);
    const handler = eventHandler(name, event);
    edit(wireEvent({ item, props: { ...p, name }, events: info.events, owner, event, handler }), key);
    pageDesigner.showCode(handler);
  }

  /** Makes `e` in the places of the file of `key` (this window's when null). */
  function edit(e: Edit, key: DesignKey | null = null) {
    if (!pageDesigner.saving) apply(e, key);
  }

  async function drop(source: PageDragSource, at: Placement) {
    if (pageDesigner.saving) return;
    // The container dropped into, in its own file: the window's, or the view's a line of its content shows.
    const into = placeOf(preview, at.container);
    if (!into) return;
    switch (source.kind) {
      case "new":
        await apply({
          op: "createWidget",
          container: into.at,
          kind: source.widget,
          col: at.col,
          row: at.row,
          width: at.width === source.width && !source.part ? null : at.width,
          caption: null,
          bind: null,
          // A custom part arrives at its declared size (DESIGN 5.19).
          part: source.part ?? null,
          height: source.part ? at.height : null,
        }, at.container);
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
            container: into.at,
            kind: source.widget,
            col: at.col,
            row: at.row,
            width: Math.min(source.width, room),
            caption: null,
            bind: source.bind,
            part: null,
            height: null,
          }, at.container);
        } else {
          await apply({ op: "placeField", container: into.at, col: at.col, row: at.row, room, bind: source.bind }, at.container);
        }
        break;
      }
      case "move": {
        const p = source.props;
        if (at.container === source.from && at.col === p.col && at.row === p.row) return;
        const moved = placeOf(preview, source.item);
        if (!moved) return;
        if (moved.file.path !== into.file.path) {
          pageDesigner.error = "A part moves within its own window or view. To have it in the other, cut it there and paste it here.";
          return;
        }
        await apply({
          op: "updateWidget",
          item: moved.at,
          container: at.container === source.from ? null : into.at,
          props: withProps(p, { col: at.col, row: at.row }),
        }, source.item);
        break;
      }
      case "resize": {
        const p = source.props;
        designLog("resize drop", `${p.width}x${p.height} -> ${at.width}x${at.height}`);
        if (at.width === p.width && at.height === p.height) return designLog("resize drop: same size, no edit");
        // A container's parts scale across with it, in the same edit.
        const found = locateKey(preview, shown, source.item);
        const sized = placeOf(preview, source.item);
        if (!found && !sized) return;
        const edits = found
          ? resizeEdits(found.widget, at.width, at.height)
          : [{ op: "updateWidget" as const, item: sized!.at, container: null, props: withProps(p, { width: at.width, height: at.height }) }];
        await apply(edits.length === 1 ? edits[0] : { op: "batch", edits }, source.item);
        break;
      }
    }
  }

  function pressPalette(e: PointerEvent, info: WidgetInfo) {
    pageDesigner.press(e, { kind: "new", widget: info.kind, width: info.width, height: info.height, label: info.name });
  }

  // The custom parts the project declares (DESIGN 5.19), dropped at their declared size.
  const customParts = $derived(preview.project.files.flatMap((f) => f.parts));

  function pressCustom(e: PointerEvent, part: PartDecl) {
    const [width, height] = part.size ?? [4, 3];
    pageDesigner.press(e, { kind: "new", widget: "custom", width, height, label: part.name.name, part: part.name.name });
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
      const placed = placeOf(preview, selected);
      if (!placed) return;
      const { page: itsPage, at } = placed;
      if (locateWidget(preview, itsPage, at) || locateInnerPage(itsPage, at) || findFrameElement(itsPage.frame, at, itsPage.widgets)) {
        e.preventDefault();
        edit({ op: "remove", item: at }, selected);
      }
    } else if (e.key === "Escape") {
      pageDesigner.selected = null;
    } else if (e.key.startsWith("Arrow") && selected !== null && !e.altKey && !e.ctrlKey && !e.metaKey) {
      const found = locateKey(preview, shown, selected);
      if (!found) return;
      const p = found.widget.props;
      if (e.shiftKey) {
        // Shift: the selected part grows or shrinks one cell, a container's parts scaling with it, as a drag of its corner does.
        const size = grown(p, found.container.columns, e.key);
        if (!size) return;
        e.preventDefault();
        const edits = resizeEdits(found.widget, size.width, size.height);
        edit(edits.length === 1 ? edits[0] : { op: "batch", edits }, selected);
      } else {
        // The selected part moves one cell that way, on any page's grid; undo takes it back.
        const to = nudged(p, found.container.columns, e.key);
        if (!to) return;
        e.preventDefault();
        edit({ op: "updateWidget", item: found.widget.span.start, container: null, props: withProps(p, to) }, selected);
      }
    }
  }

  // Drop a selection that no longer exists after an outside edit.
  $effect(() => {
    const s = pageDesigner.selected;
    if (s !== null && !placeOf(preview, s)) {
      pageDesigner.selected = null;
    }
  });

  onMount(() => {
    if (!canvas) return;
    return pageDesigner.attach(canvas, drop, dock, (edit, key) => void apply(edit, key));
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
  {#if customParts.length > 0}
    <h4>Custom parts</h4>
    <div class="tiles">
      {#each customParts as part (part.name.name)}
        <PartTile kind="custom" name={part.name.name} title="Drawn by its library; drag onto the window." onpointerdown={(e) => pressCustom(e, part)} />
      {/each}
    </div>
  {/if}
  {#each CATEGORIES.filter((c) => paletteHolds(c.key)) as c (c.key)}
    <h4>{c.name}</h4>
    <div class="tiles">
      {#each preview.catalog.filter((w) => w.category === c.key && w.kind !== "custom" && paletteOffers(w.kind)) as info (info.kind)}
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
            name={tr(preview, field.label)}
            detail={describeType(field)}
            title="Drag onto the window: a label and a bound input"
            onpointerdown={(e) => pressField(e, decl.name.name, field.name.name, tr(preview, field.label), false)}
          />
        {/each}
      {/if}
    </div>
  {:else}
    <p class="hint">Declare data in the window properties to drag bound fields onto the window.</p>
  {/each}
</section>

<div use:portal={"design-outline"}>
  <PageOutline {preview} {shown} {frame} rootName={app ? "App" : null} onedit={edit} />
</div>
</div>

<div
  class="designer"
  class:filled
  style:grid-template-columns={filled ? "minmax(0, 1fr)" : `minmax(0, 1fr) auto ${layout.folded.properties ? "auto" : layout.sizes.properties + "px"}`}
>
  <div class="stage" class:hidden={filled}>
    <ShownWindow {area} clientWidth={shownSize.width} clientHeight={shownSize.height} onresize={app ? undefined : resize}>
      <div
        class="titlebar"
        class:selected={pageDesigner.selected === null}
        role="button"
        tabindex="-1"
        onpointerdown={() => (pageDesigner.selected = null)}
      >
        <span>{page.title ? tr(preview, page.title) : page.name.name}</span>
        <code>{app ? "app " + app.name.name : page.name.name + ": " + page.kind + " | " + page.columns + " columns"}</code>
      </div>
      <div
        class="client"
        style:height="{shownSize.height}px"
        bind:this={canvas}
        onpointerdown={() => (pageDesigner.selected = null)}
        role="presentation"
      >
        <!-- The window as Clarity.UI draws it, as it runs; the design layer over it takes the pointer. -->
        <ClarityWindow shown={app ? { app: app.name.name } : { window: page.name.name }} design={true} project={pageDesigner.draft ?? savedPreview} {showing} {chosen} onlayout={(l) => (laid = l)} />
        <DesignLayer layout={laid} {shown} {preview} {appCommands} />
      </div>
      {#if app}
        <p class="main-hint">
          Each window's content goes in the window{app.main ? "; the app starts with " + app.main.name : ""}. Drag menus,
          toolbars, a ribbon, sidebars and a footer onto the edges: every window shows them unless it has its own.
        </p>
      {/if}
    </ShownWindow>
    {#if !app}
      <!-- Parts not seen (sounds, timers) have no place in the window: they wait here, as on C++ Builder's forms (DESIGN 5.22, 5.26).
           A popover or slide-out is hidden until opened: it waits here too, marked so, and shows on the window while it is picked. -->
      {@const unplaced = [...page.widgets.filter((w) => w.props.kind === "sound" || w.props.kind === "timer"), ...popovers]}
      {#if unplaced.length > 0}
        <div class="tray" role="listbox" aria-label="Parts without a place in the window">
          {#each unplaced as w (shown.keyOf(w) ?? w.span.start)}
            <span role="option" aria-selected={pageDesigner.selected === shown.keyOf(w)} class:selected={pageDesigner.selected === shown.keyOf(w)} class:hidden-part={w.props.kind === "popover" || w.props.kind === "slideout"} data-hidden-part={w.props.kind === "popover" || w.props.kind === "slideout" ? w.props.name : undefined}>
              <PartTile
                kind={w.props.kind}
                name={w.props.name ?? w.props.kind}
                title="{w.props.kind}{w.props.source ? ', plays ' + w.props.source : ''}{w.props.every ? ', every ' + w.props.every + ' s' : ''}{w.props.kind === 'popover' || w.props.kind === 'slideout' ? ', hidden until opened: picked, it shows on the window' : ''}. Pick it to set it in the inspector."
                onpointerdown={(e) => {
                  e.stopPropagation();
                  pageDesigner.selected = shown.keyOf(w);
                }}
              />
            </span>
          {/each}
        </div>
      {/if}
    {/if}
  </div>

  {#if filled}
    <!-- The canvas and the splitter wait while the code fills the designer. -->
  {:else if layout.folded.properties}
    <span></span>
  {:else}
    <!-- As tall as the room in view and kept there, so its grip is in the middle of what is seen. -->
    <div class="splitter-room" style:height="{inspectorHeight}px">
      <Splitter axis="x" label="Width of the inspector" onmove={(d) => layout.resize("properties", -d)} />
    </div>
  {/if}
  <aside bind:this={inspector} class:coding class:folded={layout.folded.properties} style:height={coding && !layout.folded.properties ? `${inspectorHeight}px` : null}>
    {#if layout.folded.properties}
      <button class="dock" title="Show the inspector" onclick={() => layout.toggle("properties")}>&lsaquo; Inspector</button>
    {:else}
      <div class="inspector-bar">
        <div class="inspector-tabs" role="tablist">
          {#each INSPECTOR_TABS as t (t.key)}
            <button role="tab" aria-selected={inspectorTab === t.key} class:active={inspectorTab === t.key} title={t.title} onclick={() => showTab(t.key)}>{t.name}</button>
          {/each}
        </div>
        <span class="bar-buttons">
        {#if coding}
          <button class="undock" title={filled ? "Give the canvas back" : "Let the code fill the designer"} onclick={() => (filling = !filling)}
            >{filled ? "\u2921 Canvas" : "\u2922 Fill"}</button
          >
        {/if}
        {#if !filled}
          <button class="undock" title="Fold the inspector away" onclick={() => layout.toggle("properties")}>Hide &rsaquo;</button>
        {/if}
        </span>
      </div>
      <div class="properties" class:hidden={coding}>
        <PageProperties {preview} page={inspected.page} selected={inspected.at} {app} {settings} {onsavesettings} onedit={(e) => edit(e, pageDesigner.selected)} {onapplied} />
      </div>
      <!-- Mounted once opened, so switching tabs keeps unsaved text. -->
      {#if codeOpened}
        <div class="code" class:hidden={!coding}>
          <CodePanel
            {preview}
            page={inspected.page}
            file={inspected.file}
            owner={inspected.page.name.name}
            item={inspected.page.span.start}
            selected={inspected.at}
            {onapplied}
            onclose={() => showTab("properties")}
          />
        </div>
      {/if}
    {/if}

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

    gap: 16px;
    align-items: start;
  }

  .splitter-room {
    display: flex;
    align-self: start;
    position: sticky;
    top: 0;
  }

  .stage.hidden {
    display: none;
  }

  .stage {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
  }

  .tray {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 6px;
    border: 1px dashed var(--border);
    border-radius: 6px;
    background: var(--panel);
  }

  .tray .hidden-part {
    opacity: 0.6;
    font-style: italic;
  }

  .tray .selected {
    outline: 2px solid var(--code-keyword);
    border-radius: 4px;
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

  .main-hint {
    margin: 4px auto;
    max-width: 420px;
    text-align: center;
    color: var(--text-muted);
    font-size: 13px;
  }

  .client {
    position: relative;
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
    min-width: 0;
  }

  /* Folded: the button keeps its own narrow size at the right, whatever else the column holds. */
  aside.folded {
    align-items: flex-end;
  }

  aside.folded .error {
    max-width: 220px;
  }

  aside.coding {
    gap: 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: var(--panel);
    overflow: hidden;
  }

  .inspector-bar {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 8px;
    border-bottom: 1px solid var(--border);
  }

  .bar-buttons {
    display: flex;
    gap: 4px;
  }

  aside.coding .inspector-bar {
    padding: 4px 8px 0;
  }

  .inspector-tabs {
    display: flex;
    gap: 2px;
  }

  .inspector-tabs button {
    border: none;
    border-radius: var(--radius) var(--radius) 0 0;
    background: none;
    padding: 5px 12px;
    color: var(--text-muted);
  }

  .inspector-tabs button.active {
    background: var(--panel-alt);
    color: var(--text);
    font-weight: 600;
  }

  .properties {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .code {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .hidden {
    display: none;
  }

  .inspector-bar .undock {
    margin: 0 0 4px;
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
