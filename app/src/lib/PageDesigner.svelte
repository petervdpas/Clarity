<script lang="ts">
  import { dialogs } from "./dialogStore.svelte";
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
  import { guided, signinTiles, signinViews, type SigninTile } from "./signinView";
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
  import type { Selected } from "./selection";
  import type { WidgetInfo } from "@clarity/ui/generated/WidgetInfo.ts";
  import { NO_FRAME, findApp, findFrameElement, shownFrame } from "@clarity/ui/screen/frame.ts";
  import { themeArea, themeOf, tr } from "@clarity/ui/screen/model.ts";
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
    onopen = null,
    onnewpart = null,
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
    /** Opens another part of the project in the main area, as the project tree does. */
    onopen?: ((selected: Selected) => void) | null;
    /** Makes a new custom part in Clarity's dialog and gives its name; null when the person cancels. */
    onnewpart?: (() => Promise<string | null>) | null;
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
    { key: "signin", name: "Sign-in" },
  ];

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

  // The sign-in window holds its content and decoration, a sign-in view its sign-in parts and decoration (the checker's rules too).
  const pages = $derived(preview.project.files.flatMap((f) => f.pages));
  const guiding = $derived(app ? null : guided(page, pages));
  /** On a sign-in view: its sign-in parts, made from the project's sign-in setup. */
  const tiles = $derived(guiding === "view" ? signinTiles(preview.project.signin ?? null, page.name.name, signinViews(pages)) : []);

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
      case "new": {
        // A Custom part is one of the project's: which, asked here, arriving at its declared size (DESIGN 5.19).
        let part = source.part ?? null;
        let height: number | null = null;
        if (source.widget === "custom" && !part) {
          const decl = await pickCustom();
          if (!decl) return;
          part = decl.name.name;
          height = (decl.size ?? [4, 3])[1];
        }
        await apply({
          op: "createWidget",
          container: into.at,
          kind: source.widget,
          col: at.col,
          row: at.row,
          width: at.width === source.width && !part ? null : at.width,
          caption: null,
          bind: null,
          part,
          height: height ?? (source.part ? at.height : null),
          input: source.input ?? null,
          strategy: source.strategy ?? null,
          shows: source.shows ?? null,
        }, at.container);
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

  function pressSignin(e: PointerEvent, tile: SigninTile) {
    pageDesigner.press(e, { kind: "new", widget: tile.kind, width: tile.width, height: tile.height, label: tile.label, input: tile.input, strategy: tile.strategy, shows: tile.shows });
  }

  // The custom parts the project declares (DESIGN 5.19): a dropped Custom part is one of them, or a new one.
  const customParts = $derived(preview.project.files.flatMap((f) => f.parts));

  /** Which custom part a dropped Custom part is: picked from the project's, or made new; none when the person cancels. */
  async function pickCustom(): Promise<PartDecl | null> {
    const NEW = "";
    let name: string | null = NEW;
    if (customParts.length > 0) {
      name = await dialogs.choose({
        title: "Which custom part",
        text: "A custom part is drawn by a library of the project's language. Pick one the project declares, or make a new one.",
        choices: [...customParts.map((d) => ({ value: d.name.name, label: d.name.name })), { value: NEW, label: "New custom part..." }],
      });
      if (name === null) return null;
    }
    if (name === NEW) {
      if (!onnewpart) return null;
      name = await onnewpart();
      if (!name) return null;
    }
    return preview.project.files.flatMap((f) => f.parts).find((d) => d.name.name === name) ?? null;
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
  {#if tiles.length > 0}
    <h4>Sign-in</h4>
    <div class="tiles">
      {#each tiles as tile (tile.kind + (tile.input ?? tile.strategy ?? tile.shows ?? ""))}
        <PartTile kind={tile.kind} name={tile.label} title="Signs in as the project's sign-in setup says; drag onto the view." onpointerdown={(e) => pressSignin(e, tile)} />
      {/each}
    </div>
  {/if}
  <!-- Every part type Clarity has, the same in every project: what a part means is settled where it is dropped. -->
  {#each CATEGORIES as c (c.key)}
    <h4>{c.name}</h4>
    <div class="tiles">
      {#each preview.catalog.filter((w) => w.category === c.key) as info (info.kind)}
        <PartTile
          kind={info.kind}
          name={info.name}
          title={info.kind === "custom" ? "Drag onto the window: pick one of the project's custom parts, or make a new one" : "Drag onto the window"}
          onpointerdown={(e) => pressPalette(e, info)}
        />
      {/each}
    </div>
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
        <PageProperties {preview} page={inspected.page} selected={inspected.at} {app} {settings} {onsavesettings} {onopen} onedit={(e) => edit(e, pageDesigner.selected)} {onapplied} />
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
