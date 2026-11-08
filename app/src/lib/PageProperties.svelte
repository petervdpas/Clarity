<script lang="ts">
  import { VERSION_PATTERN, versionWritten } from "./version";
  import type { Settings } from "@clarity/ui/generated/Settings.ts";
  import IconField from "./IconField.svelte";
  import FrameProperties from "./FrameProperties.svelte";
  import TextField from "./TextField.svelte";
  import type { App } from "@clarity/ui/generated/App.ts";
  import type { Literal } from "@clarity/ui/generated/Literal.ts";
  import { literalShown, paramsText, writtenValue } from "@clarity/ui/screen/customParts.ts";
  import type { Edit } from "@clarity/ui/generated/Edit.ts";
  import type { Event as PartEvent } from "@clarity/ui/generated/Event.ts";
  import { eventHandler, freePartName, partGrowsEdit, wireEvent } from "./codeParts";
  import { applyEdit } from "./applyEdit";
  import type { Orientation } from "@clarity/ui/generated/Orientation.ts";
  import type { Page } from "@clarity/ui/generated/Page.ts";
  import type { Preview } from "@clarity/ui/generated/Preview.ts";
  import type { Surface } from "@clarity/ui/generated/Surface.ts";
  import type { WidgetProps } from "@clarity/ui/generated/WidgetProps.ts";
  import { findFrameElement } from "@clarity/ui/screen/frame.ts";
  import { WINDOW_HEIGHT, WINDOW_MAX_HEIGHT, WINDOW_MAX_WIDTH, WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH, WINDOW_WIDTH } from "@clarity/ui/screen/grid.ts";
  import { cellEdits, describeType, themeStyles } from "@clarity/ui/screen/model.ts";
  import { pageDesigner } from "./pageDesignerStore.svelte";
  import { allPages, allParts, locateInnerPage, locateWidget, widgetInfo, withProps } from "@clarity/ui/screen/pages.ts";
  import { DEFAULT_DIGITS } from "@clarity/ui/screen/display.ts";
  import { numberRange } from "@clarity/ui/screen/ranges.ts";
  import { PLAIN_TYPES, dataEntity, dataType, isList, valueField } from "@clarity/ui/screen/windowData.ts";
  import { resourceChoices } from "@clarity/ui/screen/resources.ts";
  import { resourceKinds } from "./resourceKinds.svelte";
  import { besideEngines, canvasEvents, runEngine } from "./engines";

  let {
    preview,
    page,
    onedit,
    onapplied,
    app = null,
    settings = null,
    onsavesettings = null,
  }: {
    preview: Preview;
    page: Page;
    onedit: (edit: Edit) => void;
    /** Takes the project after a text's value or key changed in the language files. */
    onapplied: (preview: Preview) => void;
    /** Set when the page stands for the app (see `appAsPage`). */
    app?: App | null;
    /** The project file's settings, which the app's version is one of; with how to save them. */
    settings?: Settings | null;
    onsavesettings?: ((settings: Settings) => void) | null;
  } = $props();

  const selected = $derived(pageDesigner.selected);
  const location = $derived(selected === null ? null : locateWidget(preview, page, selected));
  /** The styles a part can take: those of the project's theme. */
  const styles = $derived(themeStyles(preview));
  const inner = $derived(selected === null || location ? null : locateInnerPage(page, selected));
  const p = $derived(location?.widget.props ?? null);
  const frameElement = $derived(selected === null || location || inner ? null : findFrameElement(page.frame, selected, page.widgets));
  const info = $derived(p ? widgetInfo(preview, p.kind) : null);

  /** What has fields: the entities, and the records (DESIGN 5.12) code builds and nothing stores. */
  const entities = $derived(preview.project.files.flatMap((f) => [...f.entities, ...f.records]));
  const targets = $derived([
    // A splash page is the application's to show as it starts: nothing opens it.
    ...allPages(preview).filter((x) => x.kind !== "splash").map((x) => x.name.name).filter((n) => n !== page.name.name),
  ]);

  /** Fields a widget can bind to, as `data.field`, from the page's single-record data. */
  const fieldOptions = $derived.by(() => {
    if (!info || info.bind !== "field") return [];
    return page.data
      .filter((d) => !isList(d))
      .flatMap((d) => {
        // A plain value binds by its name.
        const value = valueField(d);
        if (value) {
          const fits = info.fieldTypes.length === 0 || info.fieldTypes.includes(value.type.kind);
          return fits ? [{ value: d.name.name, label: `${d.name.name} (${dataType(d)})` }] : [];
        }
        const entity = entities.find((e) => e.name.name === dataEntity(d));
        return (entity?.fields ?? [])
          .filter((f) => info.fieldTypes.length === 0 || info.fieldTypes.includes(f.type.kind))
          .map((f) => ({ value: `${d.name.name}.${f.name.name}`, label: `${d.name.name}.${f.name.name} (${describeType(f)})` }));
      });
  });

  const listOptions = $derived(page.data.filter(isList).map((d) => d.name.name));
  const recordOptions = $derived(page.data.filter((d) => d.of.kind === "record").map((d) => d.name.name));

  /** Named inputs of the window, which a label or help text can belong to. */
  const inputNames = $derived(
    allParts(page)
      .filter((w) => w.props.name !== null && widgetInfo(preview, w.props.kind).input)
      .map((w) => w.props.name as string),
  );
  /** Named parts of the window a popover can open next to. */
  const partNames = $derived(
    allParts(page)
      .filter((w) => w.props.name !== null && w.props.kind !== "popover" && w.props.kind !== "slideout")
      .map((w) => w.props.name as string),
  );

  /** Fields of the entity behind a collection binding, for table columns and list items. */
  const collectionFields = $derived.by(() => {
    if (!p?.bind) return [];
    // A search dropdown shows fields of the records its reference points at.
    if (p.bind.includes(".")) {
      const [data, name] = p.bind.split(".");
      const decl = page.data.find((d) => d.name.name === data);
      const field = entities.find((e) => e.name.name === (decl ? dataEntity(decl) : null))?.fields.find((f) => f.name.name === name);
      const target = field?.type.kind === "ref" ? field.type.entity.name : null;
      return (entities.find((e) => e.name.name === target)?.fields ?? []).filter((f) => !["list", "image", "file"].includes(f.type.kind));
    }
    const decl = page.data.find((d) => d.name.name === p.bind);
    const entity = entities.find((e) => e.name.name === (decl ? dataEntity(decl) : null));
    return (entity?.fields ?? []).filter((f) => f.type.kind !== "list");
  });

  /** A slider's or a spin box's range when its ends are not written: the bound field's own. */
  const shownRange = $derived.by(() => {
    if (!p) return { min: null, max: null, step: "1" };
    const [data, name] = (p.bind ?? "").split(".");
    const decl = page.data.find((d) => d.name.name === data);
    const entity = entities.find((e) => e.name.name === (decl ? dataEntity(decl) : null));
    const field = entity?.fields.find((f) => f.name.name === name);
    return numberRange({ ...p, min: null, max: null }, field?.modifiers);
  });

  /** Whose code behind the handlers go into. */
  const owner = $derived(app ? app.name.name : page.name.name);

  // The window's (or app's) own events, as the backend lists them.
  const ownerEvents = $derived(app ? preview.appEvents : preview.windowEvents);
  const ownerHandlers = $derived(app ? app.handlers : page.handlers);

  /** The handler name Wire suggests for each own event. */
  const OWNER_HANDLERS: Partial<Record<PartEvent, string>> = {
    open: "windowOpened",
    close: "windowClosed",
    return: "windowReturned",
    start: "appStarted",
  };

  /** Wires one of the window's or app's own events, or unwires it with `null`. */
  function setOwnerHandler(event: PartEvent, handler: string | null) {
    const target = app ? app.span.start : page.span.start;
    wire({ op: "setOwnerEvent", owner: target, event, handler }, handler);
  }

  /** Parts with no place in the window's grid, so no `bottom`. */
  const UNPLACED: string[] = ["popover", "slideout", "sound", "timer"];
  // A slide-out at the top or bottom has a height; at the left or right a width. A new side drops the other one.
  const slidesAcross = $derived(p?.side === "top" || p?.side === "bottom");
  function sideEdit(value: string): Partial<WidgetProps> {
    const side = value === "left" || value === "top" || value === "bottom" ? value : null;
    const across = side === "top" || side === "bottom";
    return across ? { side, slideWidth: null } : { side, slideHeight: null };
  }

  function update(changes: Partial<WidgetProps>) {
    if (!location || !p) return;
    onedit({ op: "updateWidget", item: location.widget.span.start, container: null, props: withProps(p, changes) });
  }

  /** Sends an edit that wires `handler`, together with an empty handler in the code behind, and shows it. */
  function wire(edit: Edit, handler: string | null) {
    if (handler === null) {
      onedit(edit);
      return;
    }
    onedit({ op: "batch", edits: [edit, { op: "addHandler", owner, name: handler }] });
    pageDesigner.showCode(handler);
  }

  function toggle(list: string[], name: string, on: boolean): string[] {
    const next = on ? [...list, name] : list.filter((n) => n !== name);
    // Keep the entity's field order.
    return collectionFields.map((f) => f.name.name).filter((n) => next.includes(n));
  }

  /** Sets the window's size; an emptied field goes back to the standard size. */
  function setSize(width: string | number, height: string | number) {
    const w = Math.floor(Number(width));
    const h = Math.floor(Number(height));
    const standard = String(width).trim() === "" || String(height).trim() === "";
    onedit({ op: "setSize", page: page.span.start, width: standard ? null : w, height: standard ? null : h });
  }

  function numberOf(e: Event): number {
    return Math.max(1, Math.floor(Number((e.currentTarget as HTMLInputElement).value) || 1));
  }

  /** Wires a handler to an event, or unwires it with `null`. Handlers stay in catalog order. */
  /** The part's name, or the one it gets when it is changed: every part has one. */
  const partName = $derived(
    p?.name ?? (info ? freePartName(allParts(page).flatMap((w) => (w.props.name ? [w.props.name] : [])), info.keyword) : ""),
  );

  function setHandler(event: PartEvent, handler: string | null) {
    if (!p || !info || !location) return;
    onedit(wireEvent({ item: location.widget.span.start, props: { ...p, name: partName }, events: info.events, owner, event, handler }));
    if (handler) pageDesigner.showCode(handler);
  }

  // A custom part's declaration (DESIGN 5.19), when the part picked is one.
  const declared = $derived(p?.kind === "custom" ? preview.project.files.flatMap((f) => f.parts).find((d) => d.name.name === p.part) : undefined);

  // The engines a canvas can run, and the one it runs (DESIGN 5.12).
  const besides = $derived(besideEngines(preview.project));
  const running = $derived(p?.runs ? besides.find((e) => e.name.name === p.runs) : undefined);
  /** Sets whether the custom part grows with its window: its declaration's `grows`, in the module that declares it, for every place it is used. */
  async function setGrows(grows: boolean): Promise<void> {
    const set = p?.part ? partGrowsEdit(preview.project.files, p.part, grows) : null;
    if (!set || pageDesigner.saving) return;
    pageDesigner.saving = true;
    pageDesigner.error = null;
    try {
      onapplied((await applyEdit(set.file, set.edit)).preview);
    } catch (err) {
      pageDesigner.error = String(err);
    } finally {
      pageDesigner.saving = false;
    }
  }

  // What tells through the part: a custom part's declared tells, or its engine's.
  const tellsOffered = $derived(declared?.tells ?? running?.tells ?? []);
  // The events offered: none to draw a canvas its engine draws, connect and resize only on one its code draws.
  const events = $derived(info && p ? canvasEvents(info.events, p.runs, p.mode) : []);

  function setRuns(name: string | null) {
    if (!p) return;
    const engine = besides.find((e) => e.name.name === name) ?? null;
    update(runEngine(p, engine));
  }

  /** Writes a property's value on the part's line, or leaves it out with `null`. */
  function setPartValue(name: string, value: Literal | null) {
    if (!p) return;
    const values = { ...p.values };
    if (value) values[name] = value;
    else delete values[name];
    update({ values });
  }

  /** Wires a handler, taking `params`, to a tell; `null` unwires it. */
  function setTell(tell: string, handler: string | null, params: string) {
    if (!p || !location) return;
    const tells = p.tells.filter((t) => t.tell !== tell);
    if (handler) tells.push({ tell, handler });
    const edit: Edit = { op: "updateWidget", item: location.widget.span.start, container: null, props: withProps(p, { tells, name: partName }) };
    if (handler === null) {
      onedit(edit);
      return;
    }
    onedit({ op: "batch", edits: [edit, { op: "addHandlerWith", owner, name: handler, params }] });
    pageDesigner.showCode(handler);
  }

  function textOf(e: Event): string | null {
    const value = (e.currentTarget as HTMLInputElement).value.trim();
    return value === "" ? null : value;
  }

  // New data declaration form.
  let dataName = $state("");
  /** What new data holds: an entity's name, an enum's, or a plain type. */
  let dataOf = $state("");
  let dataList = $state(false);
  const enums = $derived(preview.project.files.flatMap((f) => f.enums).map((e) => e.name.name));
  /** The project's views, which a slide-out, tab, step or section can show. */
  const views = $derived(preview.project.files.flatMap((f) => f.pages).filter((page) => page.kind === "view").map((page) => page.name.name));
  const toolboxPages = $derived(preview.project.files.flatMap((f) => f.pages).filter((page) => page.kind === "toolbox").map((page) => page.name.name));
  const splashPages = $derived(preview.project.files.flatMap((f) => f.pages).filter((page) => page.kind === "splash").map((page) => page.name.name));
  /** Only records come as a list. */
  const listable = $derived(entities.some((e) => e.name.name === dataOf));

  function addData() {
    const name = dataName.trim();
    if (!name || !dataOf) return;
    onedit({ op: "addData", page: page.span.start, name, entity: dataOf, list: dataList && listable });
    dataName = "";
  }

  function withoutKey(widths: Record<string, number>, field: string): Record<string, number> {
    const next = { ...widths };
    delete next[field];
    return next;
  }

  /** A column's width as typed: a number from 20 to 2000, or empty for none. */
  function withWidth(widths: Record<string, number>, field: string, typed: string): Record<string, number> {
    const width = Math.round(Number(typed));
    if (typed.trim() === "" || !Number.isFinite(width)) return withoutKey(widths, field);
    return { ...widths, [field]: Math.min(2000, Math.max(20, width)) };
  }
</script>

<section>
  <!-- What is selected: a part, a tab or pane, a menu item, or else the window itself. -->
  {#if location && p && info}
    <div class="kind">{p.kind === "custom" ? p.part : info.name}</div>

    <label class="prop">
      <span>Name</span>
      <input value={p.name ?? partName} title="The name the code behind uses" onchange={(e) => update({ name: textOf(e) ?? partName })} />
    </label>

    {#if p.kind === "markdown" || p.kind === "image"}
      <!-- A key whose value in each language is a file of the resources, set under i18n. -->
      <label class="prop">
        <span>Key</span>
        <input
          value={p.caption?.startsWith("@") ? p.caption.slice(1) : ""}
          placeholder={p.kind === "markdown" ? "gameRules" : "bannerPicture"}
          title="A key whose value in each language is a file of the resources; point it per language under i18n"
          spellcheck="false"
          onchange={(e) => {
            const key = textOf(e);
            update({ caption: key ? `@${key.replace(/^@/, "")}` : null });
          }}
        />
      </label>
    {:else if info.caption !== null || p.caption !== null || info.container === "plain" || info.properties.includes("for")}
      <label class="prop">
        <span>Caption</span>
        <TextField {preview} {onapplied} text={p.caption} placeholder={p.kind === "help" ? "the field's help text" : ""} onwrite={(text) => update({ caption: text })} />
      </label>
    {/if}

    <!-- A sound and a timer have no place in the grid (DESIGN 5.22, 5.26). -->
    {#if p.kind !== "sound" && p.kind !== "timer"}
    <div class="position">
      <label><span>Column</span><input type="number" min="1" value={p.col} onchange={(e) => update({ col: numberOf(e) })} /></label>
      <label><span>Row</span><input type="number" min="1" value={p.row} onchange={(e) => update({ row: numberOf(e) })} /></label>
      <label><span>Width</span><input type="number" min="1" value={p.width} onchange={(e) => update({ width: numberOf(e) })} /></label>
      <label><span>Height</span><input type="number" min="1" value={p.height} onchange={(e) => update({ height: numberOf(e) })} /></label>
    </div>
    {/if}

    {#if info.bind === "field"}
      <label class="prop">
        <span>Bound to</span>
        <select value={p.bind ?? ""} onchange={(e) => update({ bind: textOf(e) })}>
          <option value="">(not bound)</option>
          {#each fieldOptions as option (option.value)}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
      </label>
      {#if fieldOptions.length === 0}
        <p class="hint">Declare data on the page first (select the page background).</p>
      {/if}
    {:else if info.bind === "record"}
      <label class="prop">
        <span>Bound to</span>
        <select value={p.bind ?? ""} onchange={(e) => update({ bind: textOf(e) })}>
          <option value="">(not bound)</option>
          {#each recordOptions as name (name)}
            <option value={name}>{name}</option>
          {/each}
        </select>
      </label>
    {:else if info.bind === "collection"}
      <label class="prop">
        <span>Bound to</span>
        <select value={p.bind ?? ""} onchange={(e) => update({ bind: textOf(e), columns: [], edit: [], switches: [], show: [], widths: {}, order: null })}>
          <option value="">(not bound)</option>
          {#each listOptions as name (name)}
            <option value={name}>{name}</option>
          {/each}
        </select>
      </label>
    {/if}

    {#if p.kind === "chart"}
      {@const numbers = collectionFields.filter((f) => ["integer", "decimal", "money", "duration"].includes(f.type.kind))}
      {@const measure = p.measure}
      <label class="prop">
        <span>Chart</span>
        <select value={p.chart ?? "bar"} onchange={(e) => update({ chart: e.currentTarget.value as "bar" | "line" | "pie" })}>
          <option value="bar">Bars</option>
          <option value="line">Line</option>
          <option value="pie">Pie</option>
        </select>
      </label>
      <label class="prop" title="The field whose values become the bars, points or slices">
        <span>Grouped by</span>
        <select value={p.by ?? ""} onchange={(e) => update({ by: e.currentTarget.value || null })}>
          <option value="">(none)</option>
          {#each collectionFields.filter((f) => !["image", "file", "values"].includes(f.type.kind)) as field (field.name.name)}
            <option value={field.name.name}>{field.label}</option>
          {/each}
        </select>
      </label>
      <label class="prop" title="Each group's number: its rows counted, or a number summed or averaged">
        <span>Number</span>
        <select
          value={measure ? (measure.kind === "count" ? "count" : `${measure.kind}:${measure.field}`) : ""}
          onchange={(e) => {
            const [kind, field] = e.currentTarget.value.split(":");
            update({ measure: kind === "count" ? { kind: "count" } : kind === "sum" || kind === "average" ? { kind, field } : null });
          }}
        >
          <option value="">(none)</option>
          <option value="count">Count of rows</option>
          {#each numbers as field (field.name.name)}
            <option value="sum:{field.name.name}">Sum of {field.label}</option>
            <option value="average:{field.name.name}">Average of {field.label}</option>
          {/each}
        </select>
      </label>
    {/if}
    {#if info.properties.includes("children")}
      {@const own = (() => {
        const decl = page.data.find((d) => d.name.name === p.bind);
        return entities.find((e) => e.name.name === (decl ? dataEntity(decl) : null));
      })()}
      {@const keys = [
        ...(own && !own.record ? ["id"] : []),
        ...(own?.fields ?? []).filter((f) => f.type.kind === "text" || f.type.kind === "integer").map((f) => f.name.name),
      ]}
      {#if !p.rowKey && !p.parentKey}
        <label class="prop" title="The list field of the rows' own kind: a row shows under the row its reference to its own kind names">
          <span>Children</span>
          <select value={p.children ?? ""} onchange={(e) => update({ children: e.currentTarget.value || null })}>
            <option value="">(none)</option>
            {#each (own?.fields ?? []).filter((f) => f.type.kind === "list" && f.type.item.name === own?.name.name) as field (field.name.name)}
              <option value={field.name.name}>{field.name.name}</option>
            {/each}
          </select>
        </label>
      {/if}
      {#if !p.children}
        <!-- A tree by key: rows of any kind, records built in code among them, under the row their parent key names. -->
        <label class="prop" title="The field that names a row, text or integer: the rows whose parent field holds it show under it">
          <span>Key</span>
          <select value={p.rowKey ?? ""} onchange={(e) => update({ rowKey: e.currentTarget.value || null })}>
            <option value="">(none)</option>
            {#each keys as key (key)}
              <option value={key}>{key}</option>
            {/each}
          </select>
        </label>
        <label class="prop" title="The field that holds the key of the row a row shows under; a row whose parent key names no row shows at the top">
          <span>Parent</span>
          <select value={p.parentKey ?? ""} onchange={(e) => update({ parentKey: e.currentTarget.value || null })}>
            <option value="">(none)</option>
            {#each keys as key (key)}
              <option value={key}>{key}</option>
            {/each}
          </select>
        </label>
      {/if}
    {/if}
    {#if info.properties.includes("cards")}
      <label class="prop" title="Each row a small card: the first field shown is its title, the others lines below it">
        <span>Cards</span>
        <input type="checkbox" checked={p.cards} onchange={(e) => update(e.currentTarget.checked ? { cards: true, striped: false } : { cards: false })} />
      </label>
    {/if}
    <!-- Cards stand apart, so a list of cards has no stripes. -->
    {#if info.properties.includes("striped") && !p.cards}
      <label class="prop" title="Every other row has a faint stripe; a chosen row shows as without it">
        <span>Striped rows</span>
        <input type="checkbox" checked={p.striped} onchange={(e) => update({ striped: e.currentTarget.checked })} />
      </label>
    {/if}
    {#if info.properties.includes("frame")}
      <label class="prop" title="Its own border inside a tab page, pane or panel, which otherwise frames it alone">
        <span>Frame</span>
        <input type="checkbox" checked={p.frame} onchange={(e) => update({ frame: e.currentTarget.checked })} />
      </label>
    {/if}
    {#if info.properties.includes("typed")}
      <label class="prop" title="Any text may be typed; the values the field already has are offered">
        <span>Typed</span>
        <input type="checkbox" checked={p.typed} onchange={(e) => update({ typed: e.currentTarget.checked, search: false, show: [] })} />
      </label>
    {/if}
    {#if info.properties.includes("search")}
      <label class="prop" title="The records filter as text is typed, each showing the fields chosen below">
        <span>Search</span>
        <input type="checkbox" checked={p.search} onchange={(e) => update({ search: e.currentTarget.checked, typed: false, show: e.currentTarget.checked ? p.show : [] })} />
      </label>
    {/if}
    {#if info.properties.includes("columns") || (info.properties.includes("show") && (p.kind !== "dropdown" || p.search))}
      {@const key = info.properties.includes("columns") ? "columns" : "show"}
      <fieldset class="checks">
        <legend>{key === "columns" ? "Columns" : "Shown fields"}</legend>
        {#each collectionFields as field (field.name.name)}
          <label>
            <input
              type="checkbox"
              checked={p[key].includes(field.name.name)}
              onchange={(e) =>
                update(
                  key === "columns" && !e.currentTarget.checked
                    ? {
                        columns: toggle(p.columns, field.name.name, false),
                        edit: toggle(p.edit, field.name.name, false),
                        switches: toggle(p.switches, field.name.name, false),
                        widths: withoutKey(p.widths, field.name.name),
                        order: p.order?.field === field.name.name ? null : p.order,
                      }
                    : { [key]: toggle(p[key], field.name.name, e.currentTarget.checked) },
                )}
            />
            {field.label}
            {#if key === "columns" && p.columns.includes(field.name.name)}
              <input
                class="width"
                type="number"
                min="20"
                max="2000"
                placeholder="auto"
                aria-label="Width of {field.label} in pixels"
                title="Width in pixels; empty shares what is left. The header edges on the canvas set it too."
                value={p.widths[field.name.name] ?? ""}
                onchange={(e) => update({ widths: withWidth(p.widths, field.name.name, e.currentTarget.value) })}
              />
            {/if}
          </label>
        {:else}
          <p class="hint">Bind to a list first.</p>
        {/each}
      </fieldset>
    {/if}

    {#if info.properties.includes("edit") && p.columns.length > 0}
      {@const editable = collectionFields.filter((f) => p.columns.includes(f.name.name) && cellEdits(f))}
      <fieldset class="checks" title="These cells are inputs. Wire 'on change' to save the row, as in: save grid.selected">
        <legend>Edited in the table</legend>
        {#each editable as field (field.name.name)}
          <label>
            <input
              type="checkbox"
              checked={p.edit.includes(field.name.name)}
              onchange={(e) =>
                update(
                  e.currentTarget.checked
                    ? { edit: toggle(p.edit, field.name.name, true) }
                    : { edit: toggle(p.edit, field.name.name, false), switches: toggle(p.switches, field.name.name, false) },
                )}
            />
            {field.label}
            {#if field.type.kind === "bool" && p.edit.includes(field.name.name)}
              <select
                class="cell-look"
                aria-label="How {field.label} shows"
                value={p.switches.includes(field.name.name) ? "switch" : "checkbox"}
                onchange={(e) => update({ switches: toggle(p.switches, field.name.name, e.currentTarget.value === "switch") })}
              >
                <option value="checkbox">Checkbox</option>
                <option value="switch">Switch</option>
              </select>
            {/if}
          </label>
        {:else}
          <p class="hint">None of the columns can be edited in a cell.</p>
        {/each}
      </fieldset>
    {/if}

    {#if info.properties.includes("reorder")}
      <!-- Reordering (DESIGN 5.30): the person puts the rows in their own order, kept in an integer field. -->
      {@const numbers = collectionFields.filter((f) => f.type.kind === "integer")}
      <fieldset class="checks" title="The person drags rows, or moves the chosen one with Alt and the arrows; wire 'on reorder' to save the rows">
        <legend>Order</legend>
        {#if numbers.length > 0}
          <label>
            <input
              type="checkbox"
              checked={p.reorder !== null}
              onchange={(e) =>
                update(
                  e.currentTarget.checked
                    ? { reorder: (numbers.find((f) => ["position", "order", "rank"].includes(f.name.name)) ?? numbers[0]).name.name, sortable: false, order: null }
                    : { reorder: null, reorderFrom: null },
                )}
            />
            Reorder by the person
          </label>
          {#if p.reorder !== null}
            <label class="order" title="The integer field the rows are numbered in, from the first number at each level">
              <span>Kept in</span>
              <select value={p.reorder} onchange={(e) => update({ reorder: e.currentTarget.value })}>
                {#each numbers as field (field.name.name)}<option value={field.name.name}>{field.label}</option>{/each}
              </select>
            </label>
            <label class="order" title="The number the first row at each level gets: 1, or 0 for a list that starts at 0">
              <span>From</span>
              <input type="number" min="0" value={p.reorderFrom ?? 1} onchange={(e) => update({ reorderFrom: e.currentTarget.valueAsNumber === 1 || Number.isNaN(e.currentTarget.valueAsNumber) ? null : e.currentTarget.valueAsNumber })} />
            </label>
          {/if}
        {:else}
          <p class="hint">Add an integer field, such as position, to keep the person's order in.</p>
        {/if}
      </fieldset>
    {/if}

    {#if info.properties.includes("add")}
      <fieldset class="checks">
        <legend>Rows</legend>
        <label title="An Add button above the table, at the right: it runs the table's add event (Insert does too)">
          <input type="checkbox" checked={p.add} onchange={(e) => update({ add: e.currentTarget.checked })} /> Add button above it
        </label>
        <label title="A trash button on every row, in a last column: it picks the row and runs the table's remove event (Delete does too)">
          <input type="checkbox" checked={p.remove} onchange={(e) => update(e.currentTarget.checked ? { remove: true } : { remove: false, confirm: false })} /> Trash button on every row
        </label>
        <label title="The trash button asks first, in Clarity's dialog, and removes the row on yes">
          <input type="checkbox" checked={p.confirm} disabled={!p.remove} onchange={(e) => update({ confirm: e.currentTarget.checked })} /> Ask first
        </label>
      </fieldset>
    {/if}

    {#if info.properties.includes("sortable") && p.columns.length > 0}
      <fieldset class="checks">
        <legend>Table</legend>
        <label title="The person drags the header edges; the widths are remembered for them">
          <input type="checkbox" checked={p.resizable} onchange={(e) => update({ resizable: e.currentTarget.checked })} /> Resizable columns
        </label>
        <label title="A click on a header sorts by that column; another click the other way">
          <input type="checkbox" checked={p.sortable} onchange={(e) => update(e.currentTarget.checked ? { sortable: true, reorder: null } : { sortable: false })} /> Sortable by a header click
        </label>

        <label class="order">
          <span>First order</span>
          <select value={p.order?.field ?? ""} onchange={(e) => update(e.currentTarget.value ? { order: { field: e.currentTarget.value, desc: p.order?.desc ?? false }, reorder: null } : { order: null })}>
            <option value="">as the data has them</option>
            {#each p.columns as column (column)}<option value={column}>{column}</option>{/each}
          </select>
        </label>
        {#if p.order}
          {@const order = p.order}
          <label><input type="checkbox" checked={order.desc} onchange={(e) => update({ order: { ...order, desc: e.currentTarget.checked } })} /> Descending</label>
        {/if}
        <label class="order" title="Shows this many rows at a time, with a bar to go through the pages; empty shows them all">
          <span>Rows per page</span>
          <input
            type="number"
            min="1"
            max="1000"
            placeholder="all"
            value={p.page ?? ""}
            onchange={(e) => {
              const size = Math.round(Number(e.currentTarget.value));
              update({ page: e.currentTarget.value === "" || !(size >= 1 && size <= 1000) ? null : size });
            }}
          />
        </label>
      </fieldset>
    {/if}

    {#if info.clickable}
      <label class="prop">
        <span>On click, open</span>
        <select value={p.onClick ?? ""} onchange={(e) => update({ onClick: textOf(e), newWindow: textOf(e) ? p.newWindow : false, openWith: textOf(e) ? p.openWith : [] })}>
          <option value="">(nothing)</option>
          {#each targets as target (target)}
            <option value={target}>{target}</option>
          {/each}
        </select>
      </label>
      {#if p.onClick}
        <label class="inline" title="Opens in a desktop window of its own. Without it, the window opens in place, on top of this one.">
          <input type="checkbox" checked={p.newWindow} onchange={(e) => update({ newWindow: e.currentTarget.checked })} /> In a new window
        </label>
        <label class="prop" title="Data of this window the opened window is filled with, each by its name there: book, chapter">
          <span>With data</span>
          <input class="mono" value={p.openWith.join(", ")} placeholder="(none)" onchange={(e) => update({ openWith: e.currentTarget.value.split(",").map((n) => n.trim()).filter((n) => n) })} />
        </label>
      {/if}
    {/if}

    {#if info.properties.includes("for")}
      <label class="prop">
        <span>{p.kind === "popover" ? "Next to" : "For input"}</span>
        <select value={p.for ?? ""} onchange={(e) => update({ for: textOf(e) })}>
          <option value="">(none)</option>
          {#each p.kind === "popover" ? partNames : inputNames as name (name)}
            <option value={name}>{name}</option>
          {/each}
        </select>
      </label>
    {/if}
    {#if info.properties.includes("digits")}
      <label class="prop">
        <span>Digits</span>
        <input
          type="number"
          min="1"
          max="24"
          value={p.digits ?? ""}
          placeholder={String(DEFAULT_DIGITS)}
          onchange={(e) => update({ digits: textOf(e) === null ? null : numberOf(e) })}
        />
      </label>
    {/if}
    {#if info.properties.includes("placeholder")}
      <label class="prop">
        <span>Placeholder</span>
        <TextField {preview} {onapplied} text={p.placeholder} placeholder="shown while the input is empty" onwrite={(text) => update({ placeholder: text })} />
      </label>
    {/if}
    {#if info.properties.includes("code")}
      <label class="prop" title="Writes in the mono font and colors the words of the language's code">
        <span>Code</span>
        <select value={p.code ?? ""} onchange={(e) => update({ code: e.currentTarget.value || null })}>
          <option value="">plain text</option>
          {#each info.codeLanguages as language (language)}
            <option value={language}>{language}</option>
          {/each}
        </select>
      </label>
    {/if}
    {#if info.properties.includes("markdown")}
      <label title="An editor with a preview: Write and Preview on its top, its Markdown colored while written, Ctrl+B and Ctrl+I for bold and italic"><input type="checkbox" checked={p.markdown} disabled={!!p.code} onchange={(e) => update({ markdown: e.currentTarget.checked })} /> Markdown</label>
    {/if}
    {#if info.properties.includes("copy")}
      <label title="A button in its corner puts its text on the clipboard, read only too"><input type="checkbox" checked={p.copy} onchange={(e) => update({ copy: e.currentTarget.checked })} /> Copy button</label>
    {/if}
    {#if info.properties.includes("to")}
      <label class="prop">
        <span>Opens</span>
        <input value={p.to ?? ""} placeholder="https://..." onchange={(e) => update({ to: textOf(e) })} />
      </label>
    {/if}
    {#if info.properties.includes("labels")}
      <label class="prop" title="Each input in the form gets its field's label in these first columns of its row">
        <span>Label column</span>
        <input
          type="number"
          min="0"
          max={p.width - 1}
          value={p.labels ?? 0}
          onchange={(e) => {
            const n = Number(e.currentTarget.value);
            update({ labels: n > 0 ? n : null });
          }}
        />
      </label>
    {/if}
    {#if info.properties.includes("side")}
      <label class="prop" title="The edge of the window it slides in from">
        <span>Side</span>
        <select value={p.side ?? "right"} onchange={(e) => update(sideEdit(e.currentTarget.value))}>
          <option value="right">Right</option>
          <option value="left">Left</option>
          <option value="top">Top</option>
          <option value="bottom">Bottom</option>
        </select>
      </label>
    {/if}
    {#if info.properties.includes("toolbox")}
      {@const memo = p.kind === "memo"}
      <label class="prop" title={memo ? "Tools that work on the memo's text where the person is: a toolbox page (page X: toolbox) of coded tools" : "The canvas's tools in a toolbox: Clarity's own for its mode, or a toolbox page (page X: toolbox) designed for it"}>
        <span>Toolbox</span>
        <select
          value={p.toolbox === null ? "" : (p.toolboxPage ?? "*")}
          onchange={(e) => {
            const v = e.currentTarget.value;
            update(v === "" ? { toolbox: null, toolboxPage: null } : { toolbox: p.toolbox ?? (memo ? "caret" : "popup"), toolboxPage: v === "*" ? null : v });
          }}
        >
          <option value="">{memo ? "(none)" : "(tools on the canvas)"}</option>
          {#if !memo}<option value="*">Clarity's own</option>{/if}
          {#each toolboxPages as name (name)}<option value={name}>{name}</option>{/each}
        </select>
      </label>
      {#if p.toolbox !== null}
        <label class="prop" title="Floating over the window, or a slide-out against the part's edge">
          <span>Shows as</span>
          <select value={p.toolbox} onchange={(e) => update({ toolbox: e.currentTarget.value as "popup" | "slideout" | "caret" })}>
            {#if memo}<option value="caret">At the caret (Ctrl+Space, right-click)</option>{/if}
            <option value="popup">Floating</option>
            <option value="slideout">Slide-out</option>
          </select>
        </label>
      {/if}
    {/if}
    {#if info.properties.includes("beside")}
      <label class="prop" title="The part it lies against, at its side of that part, instead of the window's edge; a part of the same container">
        <span>Beside</span>
        <select value={p.beside ?? ""} onchange={(e) => update({ beside: textOf(e) })}>
          <option value="">(the window's edge)</option>
          {#each partNames.filter((n) => n !== p.name) as name (name)}
            <option value={name}>{name}</option>
          {/each}
        </select>
      </label>
    {/if}
    {#if info.properties.includes("slideWidth") && !slidesAcross}
      <label class="prop" title="How wide it is, in pixels: 100 to 1600">
        <span>Width (px)</span>
        <input type="number" min="100" max="1600" value={p.slideWidth ?? ""} placeholder="420" onchange={(e) => update({ slideWidth: textOf(e) === null ? null : numberOf(e) })} />
      </label>
    {/if}
    {#if info.properties.includes("slideHeight") && slidesAcross}
      <label class="prop" title="How high it is, in pixels: 100 to 1600">
        <span>Height (px)</span>
        <input type="number" min="100" max="1600" value={p.slideHeight ?? ""} placeholder="300" onchange={(e) => update({ slideHeight: textOf(e) === null ? null : numberOf(e) })} />
      </label>
    {/if}
    {#if p.kind === "slideout"}
      <fieldset class="checks">
        <legend>Slide-out</legend>
        <label><input type="checkbox" checked={p.resizable} onchange={(e) => update({ resizable: e.currentTarget.checked })} /> The person can resize it</label>
        <label class="prop">
          <span>{slidesAcross ? "Lowest" : "Narrowest"}</span>
          <input value={p.min ?? ""} placeholder="100" inputmode="numeric" disabled={!p.resizable} onchange={(e) => update({ min: textOf(e) })} />
        </label>
        <label class="prop">
          <span>{slidesAcross ? "Highest" : "Widest"}</span>
          <input value={p.max ?? ""} placeholder="1600" inputmode="numeric" disabled={!p.resizable} onchange={(e) => update({ max: textOf(e) })} />
        </label>
        <label><input type="checkbox" checked={p.pinnable} onchange={(e) => update({ pinnable: e.currentTarget.checked })} /> The person can pin it open</label>
        <label class="prop" title="A view it shows: its parts, data and handlers join this window's, so this window's code sets what it shows">
          <span>Shows view</span>
          <select value={p.view ?? ""} onchange={(e) => update({ view: e.currentTarget.value || null })}>
            <option value="">its own parts</option>
            {#each views as view (view)}
              <option value={view}>{view}</option>
            {/each}
          </select>
        </label>
        <p class="hint">A button opens it with <code>on click -&gt; {partName}</code>; code with <code>{partName}.visible</code>{#if p.pinnable} and pins it with <code>{partName}.pinned</code>{/if}.</p>
      </fieldset>
    {/if}
    {#if info.properties.includes("min") && p.kind !== "slideout"}
      <label class="prop">
        <span>Lowest</span>
        <input value={p.min ?? ""} placeholder={shownRange.min ?? "none"} inputmode="decimal" onchange={(e) => update({ min: textOf(e) })} />
      </label>
    {/if}
    {#if info.properties.includes("max") && p.kind !== "slideout"}
      <label class="prop">
        <span>{p.kind === "progress" ? "Full at" : p.kind === "trend" || p.kind === "chart" ? "Top" : "Highest"}</span>
        <input value={p.max ?? ""} placeholder={p.kind === "progress" ? "100" : p.kind === "trend" || p.kind === "chart" ? "follows" : (shownRange.max ?? "none")} inputmode="decimal" onchange={(e) => update({ max: textOf(e) })} />
      </label>
    {/if}
    {#if info.properties.includes("step")}
      <label class="prop">
        <span>Step</span>
        <input value={p.step ?? ""} placeholder="1" inputmode="decimal" onchange={(e) => update({ step: textOf(e) })} />
      </label>
    {/if}
    {#if p.kind === "icon" || info.properties.includes("icon")}
      <label class="prop">
        <span>Icon</span>
        <IconField {preview} icon={p.icon} onwrite={(icon) => update({ icon: icon ?? (p.kind === "icon" ? p.icon : null) })} />
      </label>
    {/if}
    <label class="prop">
      <span>Tooltip</span>
      <TextField {preview} {onapplied} text={p.tooltip} placeholder="shown when the pointer rests on it" onwrite={(text) => update({ tooltip: text })} />
    </label>
    <label class="prop">
      <span>Style</span>
      {#if styles.length > 0 || p.style}
        <select value={p.style ?? ""} onchange={(e) => update({ style: e.currentTarget.value || null })}>
          <option value="">none</option>
          {#each styles as s (s)}<option value={s}>{s}</option>{/each}
          {#if p.style && !styles.includes(p.style)}<option value={p.style}>{p.style} (not in the theme)</option>{/if}
        </select>
      {:else}
        <span class="muted">Name a theme with styles in the project file to style parts.</span>
      {/if}
    </label>
    {#if info.properties.includes("tabOrder")}
      <label class="prop">
        <span>Tab order</span>
        <input
          type="number"
          min="1"
          value={p.tabOrder ?? ""}
          placeholder="reading order"
          onchange={(e) => update({ tabOrder: textOf(e) === null ? null : numberOf(e) })}
        />
      </label>
    {/if}
    {#if info.properties.includes("shortcut")}
      <label class="prop">
        <span>Shortcut</span>
        <input value={p.shortcut ?? ""} placeholder="Ctrl+S" onchange={(e) => update({ shortcut: textOf(e) })} />
      </label>
    {/if}
    {#if info.properties.includes("foldable")}
      <fieldset class="checks">
        <legend>Folding</legend>
        <label
          ><input type="checkbox" checked={p.foldable} onchange={(e) => update({ foldable: e.currentTarget.checked, folded: e.currentTarget.checked && p.folded })} /> The person
          can fold it by its header</label
        >
        <label><input type="checkbox" checked={p.folded} disabled={!p.foldable} onchange={(e) => update({ folded: e.currentTarget.checked })} /> Starts folded</label>
        <p class="hint">Code reads and sets it as <code>{partName}.folded</code>.</p>
      </fieldset>
    {/if}

    {#if info.properties.includes("readonly") || info.properties.includes("noUndo") || info.properties.includes("default") || info.properties.includes("cancel")}
      <fieldset class="checks">
        <legend>Behavior</legend>
        {#if info.properties.includes("readonly")}
          <label><input type="checkbox" checked={p.readonly} onchange={(e) => update({ readonly: e.currentTarget.checked })} /> Read-only</label>
        {/if}
        {#if info.properties.includes("noUndo")}
          <label title="Ctrl+Z takes back the person's edits of this part; untick for no undo"><input type="checkbox" checked={!p.noUndo} onchange={(e) => update({ noUndo: !e.currentTarget.checked })} /> Undo (Ctrl+Z)</label>
        {/if}
        {#if info.properties.includes("default")}
          <label><input type="checkbox" checked={p.default} onchange={(e) => update({ default: e.currentTarget.checked })} /> Enter clicks it (default)</label>
        {/if}
        {#if info.properties.includes("cancel")}
          <label><input type="checkbox" checked={p.cancel} onchange={(e) => update({ cancel: e.currentTarget.checked })} /> Escape clicks it (cancel)</label>
        {/if}
      </fieldset>
    {/if}

    {#if events.length > 0}
      <fieldset class="checks">
        <legend>Events</legend>
        {#each events as ev (ev)}
          {@const handler = p.handlers.find((h) => h.event === ev)?.handler ?? ""}
          <div class="event-row">
            <span class="event-name">{ev}</span>
            <input value={handler} placeholder="handler" onchange={(e) => setHandler(ev, textOf(e))} />
            {#if handler}
              <span class="event-buttons">
                <button class="small" title="Show the handler in the code behind" onclick={() => pageDesigner.showCode(handler)}>Code</button>
                <button class="small" title="Unwire" onclick={() => setHandler(ev, null)}>&times;</button>
              </span>
            {:else}
              <button class="small" title="Wire a handler named after the part and the event" onclick={() => setHandler(ev, eventHandler(partName, ev))}>Wire</button>
            {/if}
          </div>
        {/each}
        <p class="hint">Handlers are written in Clarity, in the window's code behind.</p>
      </fieldset>
    {/if}

    <!-- Its place in the grid: a custom part's grows (DESIGN 5.19), and bottom for any part with a place. -->
    {#if !UNPLACED.includes(p.kind)}
      <fieldset class="checks">
        <legend>Layout</legend>
        {#if p.kind === "custom" && declared}
          <label title="It takes the height the window has left, as a table or a canvas does; set in the part's declaration, so for every place it is used"
            ><input type="checkbox" checked={declared.grows} disabled={pageDesigner.saving} onchange={(e) => setGrows(e.currentTarget.checked)} /> Grows with the window</label
          >
        {/if}
        <label title="When nothing in its grid grows, it moves down with the parts on its row and below, so they sit at the bottom"
          ><input type="checkbox" checked={p.bottom} onchange={(e) => update({ bottom: e.currentTarget.checked })} /> Stick to bottom</label
        >
      </fieldset>
    {/if}

    <!-- A custom part (DESIGN 5.19): its declared properties, and its tells wired like events. -->
    {#if p.kind === "custom" && declared}
      {#if declared.properties.length > 0}
        <fieldset class="checks">
          <legend>Properties</legend>
          {#each declared.properties as property (property.name.name)}
            {@const name = property.name.name}
            {#if property.ty.kind.kind === "bool"}
              <label
                ><input
                  type="checkbox"
                  checked={p.values[name]?.kind === "bool" && p.values[name]?.value === true}
                  onchange={(e) => setPartValue(name, e.currentTarget.checked ? { kind: "bool", value: true } : null)}
                />
                {name}</label
              >
            {:else}
              <label class="prop">
                <span>{name}</span>
                <input
                  value={literalShown(p.values[name])}
                  placeholder={property.default ? "as declared" : ""}
                  onchange={(e) => setPartValue(name, writtenValue(property.ty.kind, e.currentTarget.value))}
                />
              </label>
            {/if}
          {/each}
          <p class="hint">Code reads and sets them as <code>{partName}.{declared.properties[0].name.name}</code>.</p>
        </fieldset>
      {/if}
    {/if}

    <!-- A custom part's tells, or those of the engine a canvas runs, wired like events. -->
    {#if tellsOffered.length > 0}
      <fieldset class="checks">
        <legend>Tells</legend>
        {#each tellsOffered as tell (tell.name.name)}
          {@const handler = p.tells.find((t) => t.tell === tell.name.name)?.handler ?? ""}
          <div class="event-row">
            <span class="event-name">{tell.name.name}</span>
            <input value={handler} placeholder="handler" onchange={(e) => setTell(tell.name.name, textOf(e), paramsText(tell.params))} />
            {#if handler}
              <span class="event-buttons">
                <button class="small" title="Show the handler in the code behind" onclick={() => pageDesigner.showCode(handler)}>Code</button>
                <button class="small" title="Unwire" onclick={() => setTell(tell.name.name, null, "")}>&times;</button>
              </span>
            {:else}
              <button class="small" title="Wire a handler named after the part and the tell" onclick={() => setTell(tell.name.name, partName + tell.name.name[0].toUpperCase() + tell.name.name.slice(1), paramsText(tell.params))}
                >Wire</button
              >
            {/if}
          </div>
        {/each}
        <p class="hint">A handler wired to a tell takes the tell's values.</p>
      </fieldset>
    {/if}

    {#if info.properties.includes("orientation")}
      <label class="prop">
        <span>Orientation</span>
        <!-- An accordion stacks its sections unless written horizontal, so stacked writes nothing. -->
        <select
          value={p.orientation ?? (p.kind === "accordion" ? "vertical" : "horizontal")}
          onchange={(e) => {
            const chosen = e.currentTarget.value as Orientation;
            update({ orientation: p.kind === "accordion" && chosen === "vertical" ? null : chosen });
          }}
        >
          <option value="horizontal">Side by side</option>
          <option value="vertical">Stacked</option>
        </select>
      </label>
    {/if}
    {#if info.properties.includes("runs")}
      <label class="prop" title="An engine beside the screen draws the canvas, frame by frame, and hears its pointer and keys">
        <span>Engine</span>
        <select value={p.runs ?? ""} onchange={(e) => setRuns(textOf(e))}>
          <option value="">(none: a draw handler draws it)</option>
          {#each besides as engine (engine.name.name)}<option value={engine.name.name}>{engine.name.name}</option>{/each}
          {#if p.runs && !running}<option value={p.runs}>{p.runs} (no engine beside the screen)</option>{/if}
        </select>
      </label>
      {#if running && running.hears.length > 0}
        <p class="hint">Code tells it {#each running.hears as hear, i (hear.name.name)}{i > 0 ? ", " : ""}<code>{partName}.{hear.name.name}({hear.params.map((v) => v.name.name).join(", ")})</code>{/each}.</p>
      {/if}
    {/if}
    {#if info.properties.includes("surface")}
      <label class="prop">
        <span>Surface</span>
        <select value={p.surface ?? "svg"} onchange={(e) => update({ surface: e.currentTarget.value as Surface })}>
          <option value="svg">SVG (vector)</option>
          <option value="bitmap">Bitmap</option>
          <option value="gpu">GPU</option>
        </select>
      </label>
    {/if}
    {#if info.properties.includes("source")}
      <!-- A file of the resources, of the kind the part shows; a key points each language at one instead. -->
      {@const choices = resourceChoices(preview.project.resources?.files, p.kind === "markdown" ? "markdown" : p.kind === "sound" ? "sound" : "picture", resourceKinds.list)}
      <label class="prop">
        <span>Source</span>
        <select value={p.source ?? ""} onchange={(e) => update({ source: textOf(e) })}>
          <option value="">(none)</option>
          {#each choices as name (name)}<option value={name}>{name}</option>{/each}
          {#if p.source && !choices.includes(p.source)}<option value={p.source}>{p.source} (not in the resources)</option>{/if}
        </select>
      </label>
    {/if}

    {#if info.properties.includes("every")}
      <label class="prop" title={p.kind === "timer" ? "Seconds between its ticks, from 0.1 to 86400" : "Seconds between the values it takes; 1 without it"}>
        <span>Every (s)</span>
        <input value={p.every ?? ""} placeholder={p.kind === "timer" ? "1" : "1"} inputmode="decimal" onchange={(e) => update({ every: textOf(e) })} />
      </label>
    {/if}
    {#if info.properties.includes("points")}
      <label class="prop" title={p.kind === "trend" ? "How many values it keeps, from 2 to 1000" : "How many of the list's last numbers it shows; all without it"}>
        <span>Points</span>
        <input type="number" min="2" max="1000" value={p.points ?? ""} placeholder={p.kind === "trend" ? "60" : "all"} onchange={(e) => update({ points: textOf(e) === null ? null : numberOf(e) })} />
      </label>
    {/if}
    {#if info.properties.includes("volume")}
      <label class="prop" title="How loud it plays, from 0 to 100">
        <span>Volume</span>
        <input type="number" min="0" max="100" value={p.volume ?? ""} placeholder="100" onchange={(e) => update({ volume: textOf(e) === null ? null : numberOf(e) })} />
      </label>
    {/if}
    {#if info.properties.includes("repeat")}
      <label class="prop" title="Plays its source over and over until code stops it">
        <span>Repeat</span>
        <input type="checkbox" checked={p.repeat} onchange={(e) => update({ repeat: e.currentTarget.checked })} />
      </label>
    {/if}
    {#if info.properties.includes("split")}
      <label class="prop" title="An arrow beside the button opens its menu">
        <span>Split</span>
        <input type="checkbox" checked={p.split} onchange={(e) => update({ split: e.currentTarget.checked })} />
      </label>
    {/if}
    {#if !location.widget.menu && info.container !== "steps"}
      <button
        title={p.split ? "The menu behind the split button's arrow" : "A menu a right click on the part opens"}
        onclick={() => onedit({ op: "addMenu", item: location.widget.span.start })}>{p.split ? "Add its menu" : "Add a context menu"}</button
      >
    {/if}

    {#if info.container === "tabs"}
      <button onclick={() => onedit({ op: "addTab", item: location.widget.span.start, title: `Tab ${location.widget.pages.length + 1}` })}>
        Add tab
      </button>
    {:else if info.container === "accordion"}
      <button onclick={() => onedit({ op: "addTab", item: location.widget.span.start, title: `Section ${location.widget.pages.length + 1}` })}>
        Add section
      </button>
    {:else if info.container === "steps"}
      <button onclick={() => onedit({ op: "addTab", item: location.widget.span.start, title: `Step ${location.widget.pages.length + 1}` })}>
        Add step
      </button>
    {/if}

    <div class="actions">
      <button class="danger" onclick={() => onedit({ op: "remove", item: location.widget.span.start })}>Remove</button>
    </div>
  {:else if inner}
    {@const tab = inner.widget.pages[inner.index]}
    {#if tab.title !== null}
      {@const step = inner.widget.props.kind === "steps"}
      {@const section = inner.widget.props.kind === "accordion"}
      <div class="kind">{step ? "Step" : section ? "Section" : "Tab"}</div>
      <label class="prop">
        <span>Title</span>
        <TextField {preview} {onapplied} text={tab.title} onwrite={(text) => onedit({ op: "setTitle", target: tab.span.start, title: text ?? "" })} />
      </label>
      {#if section}
        <label class="prop" title="Its header in a color of its own, its text black or white as reads; without one it takes the theme's look">
          <span>Color</span>
          <span class="color-pick">
            <input type="color" value={tab.color?.startsWith("#") ? tab.color : "#2e86de"} onchange={(e) => onedit({ op: "setPageColor", target: tab.span.start, color: e.currentTarget.value })} />
            {#if tab.color}
              <span class="color-name">{tab.color}</span>
              <button type="button" onclick={() => onedit({ op: "setPageColor", target: tab.span.start, color: null })}>Theme's</button>
            {:else}
              <span class="color-name">the theme's</span>
            {/if}
          </span>
        </label>
      {/if}
      <label class="prop" title="A view it shows: its parts, data and handlers join this window's, so this window's code sets what it shows">
        <span>Shows view</span>
        <select
          value={tab.view ?? ""}
          disabled={tab.view === null && tab.children.length > 0}
          onchange={(e) => onedit({ op: "setPageView", target: tab.span.start, view: e.currentTarget.value || null })}
        >
          <option value="">its own parts</option>
          {#each views as view (view)}
            <option value={view}>{view}</option>
          {/each}
        </select>
      </label>
      {#if tab.view === null && tab.children.length > 0}
        <p class="hint">It holds parts of its own; remove them to show a view here.</p>
      {/if}
      <div class="actions">
        <button class="danger" onclick={() => onedit({ op: "remove", item: tab.span.start })}>{step ? "Remove step" : section ? "Remove section" : "Remove tab"}</button>
      </div>
    {:else}
      {@const splitter = inner.widget.props}
      <div class="kind">Pane</div>
      {#if inner.index === 0 && splitter.orientation !== "vertical"}
        <!-- The columns the first pane starts with (`pane span 4`); the second has the rest, and the divider moves them. -->
        <label class="prop">
          <span>Width</span>
          <input
            type="number"
            min="1"
            max={splitter.width - 1}
            value={tab.columns ?? ""}
            placeholder="half"
            title="The columns this pane starts with; the other pane has the rest. Empty: the panes start alike."
            onchange={(e) => {
              const typed = (e.currentTarget as HTMLInputElement).value.trim();
              const n = typed === "" ? 0 : numberOf(e);
              onedit({ op: "setPaneSpan", target: tab.span.start, columns: n >= 1 && n < splitter.width ? n : null });
            }}
          />
        </label>
      {/if}
    {/if}
  {:else if frameElement}
    <FrameProperties {preview} element={frameElement} {onedit} {onapplied} {owner} />
  {:else}
    {#if app}
      <div class="kind">App <code>{app.name.name}</code></div>
    {:else}
      <div class="kind">Page <code>{page.name.name}</code> <span class="muted">{page.kind}</span></div>
    {/if}
    <label class="prop">
      <span>Title</span>
      <TextField {preview} {onapplied} text={page.title} placeholder={page.name.name} onwrite={(text) => onedit({ op: "setTitle", target: page.span.start, title: text ?? "" })} />
    </label>
    {#if app}
      <label class="prop">
        <span>Main window</span>
        <select value={app.main?.name ?? ""} onchange={(e) => onedit({ op: "setMain", app: app.span.start, main: e.currentTarget.value })}>
          <option value="">(none)</option>
          {#each targets as target (target)}
            <option value={target}>{target}</option>
          {/each}
        </select>
      </label>
      <!-- The application's version, a line of the project file; optional. -->
      {#if settings && onsavesettings}
        <label class="prop">
          <span>Version</span>
          <input
            class="mono"
            value={settings.version ?? ""}
            placeholder="none"
            spellcheck="false"
            pattern={VERSION_PATTERN.source}
            onchange={(e) => {
              const version = versionWritten(e.currentTarget.value);
              if (version !== undefined && settings && onsavesettings) onsavesettings({ ...settings, version });
            }}
          />
        </label>
      {/if}
      <!-- Optional: an app without a splash starts with its main window. -->
      <label class="prop">
        <span>Splash screen</span>
        <select value={app.splash?.name ?? ""} onchange={(e) => onedit({ op: "setSplash", app: app.span.start, splash: e.currentTarget.value, ms: app.splashFor })}>
          <option value="">(none)</option>
          {#each splashPages as splash (splash)}
            <option value={splash}>{splash}</option>
          {/each}
        </select>
      </label>
      {#if app.splash}
        <label class="prop">
          <span>Shows at least (s)</span>
          <input
            type="number"
            min="0.1"
            max="30"
            step="0.1"
            value={app.splashFor === null ? "" : app.splashFor / 1000}
            placeholder="until the main window shows"
            onchange={(e) => {
              const seconds = e.currentTarget.valueAsNumber;
              onedit({ op: "setSplash", app: app.span.start, splash: app.splash?.name ?? "", ms: Number.isFinite(seconds) ? Math.round(seconds * 1000) : null });
            }}
          />
        </label>
      {/if}
    {:else}
    <label class="prop">
      <span>Grid columns</span>
      <input type="number" min="1" max="96" value={page.columns} onchange={(e) => onedit({ op: "setGrid", page: page.span.start, columns: numberOf(e), rows: page.cell })} />
    </label>
    <!-- The page's own exact grid: rows of this many pixels, cells without gaps, for a designed page such as a splash. Empty for the project's rows. -->
    <label class="prop">
      <span>Grid rows (px)</span>
      <input
        type="number"
        min="8"
        max="64"
        value={page.cell ?? ""}
        placeholder="the project's rows"
        onchange={(e) => onedit({ op: "setGrid", page: page.span.start, columns: page.columns, rows: textOf(e) === null ? null : numberOf(e) })}
      />
    </label>
    <!-- The size its desktop window opens at; both empty for the standard size. -->
    <div class="prop size">
      <span>Size</span>
      <input type="number" min={WINDOW_MIN_WIDTH} max={WINDOW_MAX_WIDTH} aria-label="Width in pixels" placeholder={String(WINDOW_WIDTH)} value={page.size?.width ?? ""} onchange={(e) => setSize(e.currentTarget.value, page.size?.height ?? WINDOW_HEIGHT)} />
      <span class="times">x</span>
      <input type="number" min={WINDOW_MIN_HEIGHT} max={WINDOW_MAX_HEIGHT} aria-label="Height in pixels" placeholder={String(WINDOW_HEIGHT)} value={page.size?.height ?? ""} onchange={(e) => setSize(page.size?.width ?? WINDOW_WIDTH, e.currentTarget.value)} />
    </div>
    <!-- The window's own style: its background and the text of its parts, as a splash screen's deep color. -->
    <label class="prop">
      <span>Style</span>
      {#if styles.length > 0 || page.style}
        <select value={page.style ?? ""} onchange={(e) => onedit({ op: "setPageStyle", page: page.span.start, style: e.currentTarget.value })}>
          <option value="">none</option>
          {#each styles as s (s)}<option value={s}>{s}</option>{/each}
          {#if page.style && !styles.includes(page.style)}<option value={page.style}>{page.style} (not in the theme)</option>{/if}
        </select>
      {:else}
        <span class="muted">Name a theme with styles in the project file to style the window.</span>
      {/if}
    </label>
    {/if}

    <fieldset class="checks">
      <legend>Data</legend>
      {#each page.data as decl (decl.span.start)}
        <div class="data-row">
          <code>{decl.name.name}: {dataType(decl)}</code>
          <button class="small" onclick={() => onedit({ op: "remove", item: decl.span.start })}>Remove</button>
        </div>
      {:else}
          <p class="hint">No data yet. Parts bind to the records, lists and values declared here.</p>
      {/each}
      <div class="add-data">
        <input bind:value={dataName} placeholder="name" />
        <select bind:value={dataOf} aria-label="What it holds">
          <option value="" disabled>holds</option>
          <optgroup label="Records">
            {#each entities as entity (entity.name.name)}
              <option value={entity.name.name}>{entity.name.name}</option>
            {/each}
          </optgroup>
          <optgroup label="Values">
            {#each PLAIN_TYPES as type (type)}
              <option value={type}>{type}</option>
            {/each}
            {#each enums as name (name)}
              <option value={name}>{name}</option>
            {/each}
          </optgroup>
        </select>
        <label class="inline"><input type="checkbox" bind:checked={dataList} disabled={!listable} /> list</label>
        <button class="small" onclick={addData} disabled={!dataName.trim() || !dataOf}>Add</button>
      </div>
    </fieldset>

    <fieldset class="checks">
      <legend>Events</legend>
      {#each ownerEvents as ev (ev)}
        {@const handler = ownerHandlers.find((h) => h.event === ev)?.handler ?? ""}
        <div class="event-row">
          <span class="event-name">{ev}</span>
          <input value={handler} placeholder="handler" onchange={(e) => setOwnerHandler(ev, textOf(e))} />
          {#if handler}
            <span class="event-buttons">
              <button class="small" title="Show the handler in the code behind" onclick={() => pageDesigner.showCode(handler)}>Code</button>
              <button class="small" title="Unwire" onclick={() => setOwnerHandler(ev, null)}>&times;</button>
            </span>
          {:else}
            <button class="small" title="Wire a handler for this event" onclick={() => setOwnerHandler(ev, OWNER_HANDLERS[ev] ?? null)}
              >Wire</button
            >
          {/if}
        </div>
      {/each}
      <p class="hint">
        {app ? "Start runs once, before the main window opens." : "Open runs when the window has its data, close before it closes, return when a window it opened has closed: the place to reload a list."}
      </p>
    </fieldset>
  {/if}
</section>

<style>
  .checks .width {
    width: 6em;
    margin-left: auto;
  }

  .checks .order {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  section {
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .kind {
    font-weight: 600;
  }

  .kind code {
    font-family: var(--mono);
    font-weight: 400;
    font-size: 12px;
  }

  .muted {
    color: var(--text-muted);
    font-weight: 400;
    font-size: 12px;
  }

  .prop {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 12px;
    color: var(--text-muted);
  }

  /* Width x height on one line under the label. */
  .prop.size {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
    align-items: center;
    column-gap: 6px;
  }

  .prop.size > span:first-child {
    grid-column: 1 / -1;
  }

  .prop.size input {
    min-width: 0;
  }

  .position {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 6px;
  }

  .position label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 11px;
    color: var(--text-muted);
  }

  .checks {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    margin: 0;
    padding: 6px 10px 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
  }

  .checks legend {
    font-size: 12px;
    color: var(--text-muted);
    padding: 0 4px;
  }

  .checks label {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .event-row {
    display: grid;
    grid-template-columns: 56px minmax(0, 1fr) auto;
    gap: 6px;
    align-items: center;
  }

  .event-buttons {
    display: flex;
    gap: 4px;
  }

  .event-name {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--text-muted);
  }

  .data-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 6px;
  }

  .data-row code {
    font-family: var(--mono);
    font-size: 12px;
  }

  .add-data {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 6px;
    align-items: center;
    margin-top: 4px;
  }

  .inline {
    font-size: 12px;
  }

  .small {
    padding: 2px 8px;
    font-size: 12px;
  }

  .actions {
    display: flex;
    gap: 6px;
  }

  .danger {
    color: var(--error);
  }
  .cell-look {
    margin-left: auto;
    font-size: 11px;
    padding: 0 2px;
  }
  .color-pick {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .color-name {
    color: var(--text-muted);
  }
</style>
