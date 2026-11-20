<script lang="ts">
  // A report of the reports folder (docs/ledger/reports-pdf.md), designed on
  // paper in the Reports tab: its bands one under the other at their
  // heights, its parts placed in them in millimetres. Drag a part to move
  // it, its corner to size it, a band's lower edge to change its height.
  // The palette adds parts to the picked band and the bands it lacks; the
  // properties change what is picked, the report or its data. Save writes
  // the .report file (one undo step); Preview lays it out with sample data
  // as Clarity.Pdf does in the program.

  import { invoke } from "@clarity/ui/host/backend.ts";
  import { dialogs } from "./dialogStore.svelte";
  import { withDeclaration, BANDS, REPEATED, PARTS, newPart, paperSize, partLabel, partLook, readReport, snapped, stylesheetRules, writeReport, mm, type BandKind, type PartKind, type ReportModel, type ReportPart } from "./reportDesign";
  import PartTile from "./PartTile.svelte";
  import Splitter from "@clarity/ui/parts/Splitter.svelte";
  import { layout } from "./layout.svelte";
  import ZoomBar from "./ZoomBar.svelte";
  import IdeIcon from "./IdeIcon.svelte";
  import ReportLook from "./ReportLook.svelte";
  import { reportDesigner } from "./reportDesignerStore.svelte";
  import SourceEditor from "./SourceEditor.svelte";
  import type { Diagnostic } from "@clarity/ui/generated/Diagnostic.ts";
  import type { Preview } from "@clarity/ui/generated/Preview.ts";
  import type { ReportPreview } from "@clarity/ui/generated/ReportPreview.ts";

  let {
    preview,
    name,
    onapplied,
    ondeleted = () => {},
  }: {
    preview: Preview;
    /** The report's name: its file is reports/<name>.report. */
    name: string;
    onapplied: (preview: Preview) => void;
    ondeleted?: (preview: Preview) => void;
  } = $props();

  const file = $derived(preview.project.reports.find((r) => r.name === name));
  const entities = $derived(preview.project.files.flatMap((f) => f.entities.filter((e) => !e.settings).map((e) => e.name.name)));

  let model = $state<ReportModel | null>(null);
  let problem = $state<string | null>(null);
  /** The text last read or saved, to know whether there are changes. */
  let saved = $state("");
  $effect.pre(() => {
    const text = file?.text ?? "";
    if (text === saved && model) return;
    try {
      model = readReport(text);
      problem = null;
    } catch (e) {
      model = null;
      problem = String(e instanceof Error ? e.message : e);
    }
    saved = text;
  });
  const text = $derived(model ? writeReport(model) : saved);
  const changed = $derived(model !== null && text !== saved);

  /** Pixels per millimetre on the canvas. */
  let zoom = $state(96 / 25.4);
  /** The grid parts snap to, in millimetres; 0 for none. */
  const grid = $derived(reportDesigner.grid);
  /** How far apart the grid's dots are, in millimetres: the snap step, not closer than 2.5 mm so the paper stays readable; 5 mm with no snap. */
  const dotStep = $derived(grid > 0 ? Math.max(grid, 2.5) : 5);
  /** A length on the grid. */
  function onGrid(n: number, least = 0): number {
    return grid > 0 ? Math.max(least, Math.round(n / grid) * grid) : snapped(n, least);
  }
  /** What is picked: a part of a band, a band, or the report. */
  let picked = $state<{ band: number; part: number | null } | null>(null);
  /** The properties slide out over the canvas when something is picked, and away with ×. */
  let showProperties = $state(false);
  let canvasEl = $state<HTMLDivElement | null>(null);
  $effect(() => {
    if (picked) showProperties = true;
  });

  const margin = $derived(model ? mm(model.attrs.margin, 20) : 20);
  const paper = $derived(model ? paperSize(model) : [210, 297]);
  const ordered = $derived(model ? model.bands.map((b, i) => ({ b, i })).sort((x, y) => BANDS.indexOf(x.b.kind) - BANDS.indexOf(y.b.kind)) : []);
  /** The bar above each band on the canvas, with its name: only on the canvas, never printed. */
  const BAR = 18;
  /** Each band's top on the canvas in pixels, under its bar: the page header
   *  at the top margin, the report's own bands under it, the page footer at
   *  the bottom of the page. Inside a band everything is as it prints. */
  const tops = $derived.by(() => {
    let y = margin * zoom;
    const out: Record<number, number> = {};
    for (const { b, i } of ordered) {
      if (b.kind === "PageFooter") continue;
      out[i] = y + BAR;
      y += BAR + b.height * zoom;
    }
    for (const { b, i } of ordered) {
      if (b.kind === "PageFooter") out[i] = Math.max(y + BAR, (paper[1] - margin - b.height) * zoom + ordered.length * BAR);
    }
    return out;
  });
  /** How tall the paper shows: the page with the bars, or more when the bands run past it. */
  const paperHeight = $derived(Math.max(paper[1] * zoom + ordered.length * BAR, ...ordered.map(({ b, i }) => tops[i] + (b.height + margin) * zoom)));
  /** The report's stylesheet's rules, for parts drawn as they print. */
  /** The report's stylesheet as the designer holds it; a look changed in the properties or the Styles tab is written into it. */
  let css = $state("");
  let cssRead = "";
  $effect.pre(() => {
    const onDisk = file?.stylesheet ?? "";
    if (onDisk !== cssRead) {
      css = onDisk;
      cssRead = onDisk;
    }
  });
  const rules = $derived(stylesheetRules(css));
  /** The stylesheet's file: the one the report names, else one named after the report, which it then names. */
  function stylesheetFile(): string {
    if (!model) return "";
    if (!model.attrs.stylesheet) model.attrs.stylesheet = `${name.charAt(0).toLowerCase()}${name.slice(1)}.css`;
    return model.attrs.stylesheet;
  }
  let cssSaving: ReturnType<typeof setTimeout> | null = null;
  /** Sets one declaration of a rule of the stylesheet, and writes the stylesheet a moment later. */
  function setLook(selector: string, property: string, value: string) {
    css = withDeclaration(css, selector, property, value);
    const target = stylesheetFile();
    if (cssSaving) clearTimeout(cssSaving);
    cssSaving = setTimeout(async () => {
      cssSaving = null;
      try {
        onapplied(await invoke<Preview>("save_report_stylesheet", { file: target, text: css }));
        cssRead = css;
      } catch (e) {
        await dialogs.message({ title: "Save stylesheet", text: String(e) });
      }
    }, 400);
  }
  /** Writes the text of a key in a language into the translations; one undo step. */
  async function setText(key: string, value: string, lang = language) {
    if (!key || !value.trim()) return;
    try {
      onapplied(await invoke<Preview>("set_translation", { language: lang, key, value }));
    } catch (e) {
      await dialogs.message({ title: "Text", text: String(e) });
    }
  }

  /** A part's class, made when it has none: named after its key or field (`invoiceTitle`, `number`), else its kind. */
  function classOf(p: ReportPart): string {
    if (!p.attrs.class) {
      const from = p.attrs.key || p.attrs.bind?.split(".").at(-1) || p.attrs.of?.split(".").at(-1) || p.kind.toLowerCase();
      p.attrs.class = from.replace(/[^A-Za-z0-9_-]/g, "");
    }
    return p.attrs.class;
  }
  /** The stylesheet's rules, for the Styles tab: Clarity's own selectors a report or its Markdown uses, and its classes. */
  const styleRules = $derived(["body", "h1", "h2", "h3", "p", "table", "th", "td", ...Object.keys(rules).filter((k) => k.startsWith("."))].filter((v, i, all) => all.indexOf(v) === i));
  let styleRule = $state("body");
  /** The slide-out shows the Look tab. */
  let looking = $state(false);
  const language = $derived(preview.project.locale.language ?? "en");
  /** The project's languages, the default first. */
  const languages = $derived([language, ...Object.keys(preview.project.translations).filter((l) => l !== language).sort()]);
  /** What a part shows as it prints: a text in the default language, or the field it binds. */
  function shown(p: ReportPart): string {
    if (p.kind === "Text") return preview.project.translations[language]?.[p.attrs.key] ?? (p.attrs.key ? `@${p.attrs.key}` : "Text");
    if (p.kind === "Field" || p.kind === "Markdown") return p.attrs.bind ? `[${p.attrs.bind}]` : `[${p.kind}]`;
    return partLabel(p);
  }
  const TILES: Record<string, string> = { Text: "reportText", Field: "reportField", Sum: "reportSum", Markdown: "reportMarkdown", PageNumber: "reportPageNumber", Image: "reportImage", Line: "reportLine", Box: "reportBox", PageHeader: "bandPageHeader", ReportHeader: "bandReportHeader", GroupHeader: "bandGroupHeader", GroupFooter: "bandGroupFooter", Detail: "bandDetail", ReportFooter: "bandReportFooter", PageFooter: "bandPageFooter" };
  /** Read and change the report's XML itself. */
  /** The slide-out shows the report's XML (the ribbon's XML), else the properties. */
  const source = $derived(reportDesigner.xml);
  function problemsOf(t: string): Diagnostic[] {
    try {
      readReport(t);
      return [];
    } catch (e) {
      return [{ severity: "error", message: String(e instanceof Error ? e.message : e), path: `reports/${name}.report`, span: { start: 0, end: 0, line: 1, col: 1 } } as Diagnostic];
    }
  }
  /** The bands the palette offers: those the report lacks, and groups, which it may have several of. */
  const missing = $derived(model ? BANDS.filter((k) => REPEATED.includes(k) || !model!.bands.some((b) => b.kind === k)) : []);

  type Drag = { kind: "move" | "size" | "height"; band: number; part: number | null; x: number; y: number; from: [number, number]; handle?: string; at?: [number, number] };
  /** A picked part's handles: its corners and the middles of its sides. */
  const HANDLES = ["nw", "n", "ne", "e", "se", "s", "sw", "w"] as const;
  let drag: Drag | null = null;

  /** The parts picked together (Shift+click, or a box drawn round them); the last picked leads, as the one the others align to. */
  let chosen = $state<{ band: number; part: number }[]>([]);
  const isChosen = (band: number, part: number) => chosen.some((c) => c.band === band && c.part === part);
  /** Where each chosen part was when a drag of them started. */
  let froms: [number, number][] = [];

  function startDrag(e: PointerEvent, kind: Drag["kind"], band: number, part: number | null, handle?: string) {
    if (!model) return;
    e.stopPropagation();
    if (kind === "move" && part !== null && e.shiftKey) {
      // Shift+click adds a part to the ones picked, or takes it out.
      chosen = isChosen(band, part) ? chosen.filter((c) => !(c.band === band && c.part === part)) : [...chosen, { band, part }];
      picked = { band, part };
      return;
    }
    if (part !== null && !isChosen(band, part)) chosen = [{ band, part }];
    if (part === null) chosen = [];
    picked = { band, part };
    const b = model.bands[band];
    const from: [number, number] = kind === "height" ? [0, b.height] : kind === "size" ? [...b.parts[part!].size] : [...b.parts[part!].at];
    froms = chosen.map((c) => [...model!.bands[c.band].parts[c.part].at] as [number, number]);
    drag = { kind, band, part, x: e.clientX, y: e.clientY, from, handle, at: part !== null ? [...b.parts[part].at] : undefined };
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
  }

  /** A box drawn on the paper round parts, to pick them together. */
  let marquee = $state<{ x0: number; y0: number; x1: number; y1: number } | null>(null);
  function startMarquee(e: PointerEvent) {
    if (e.button !== 0) return;
    marquee = { x0: e.clientX, y0: e.clientY, x1: e.clientX, y1: e.clientY };
  }
  function moveMarquee(e: PointerEvent) {
    if (marquee) {
      marquee.x1 = e.clientX;
      marquee.y1 = e.clientY;
    }
  }
  function endMarquee() {
    const m = marquee;
    marquee = null;
    if (!m || (Math.abs(m.x1 - m.x0) < 4 && Math.abs(m.y1 - m.y0) < 4)) return;
    const [left, right, top, bottom] = [Math.min(m.x0, m.x1), Math.max(m.x0, m.x1), Math.min(m.y0, m.y1), Math.max(m.y0, m.y1)];
    const inside: { band: number; part: number }[] = [];
    for (const el of document.querySelectorAll<HTMLElement>(".report .part[data-key]")) {
      const r = el.getBoundingClientRect();
      if (r.right >= left && r.left <= right && r.bottom >= top && r.top <= bottom) {
        const [band, part] = (el.dataset.key ?? "").split(":").map(Number);
        inside.push({ band, part });
      }
    }
    chosen = inside;
    picked = inside.at(-1) ?? null;
  }

  type Align = "left" | "center" | "right" | "top" | "middle" | "bottom" | "width" | "height" | "across" | "down";
  /** The Align tab's buttons. */
  const ALIGNS: { how: Align; icon: "align-left" | "align-center" | "align-right" | "align-top" | "align-middle" | "align-bottom" | "same-width" | "same-height" | "spread-across" | "spread-down"; label: string; title: string }[] = [
    { how: "left", icon: "align-left", label: "Left", title: "Left edges in line" },
    { how: "center", icon: "align-center", label: "Centre", title: "Centres in line, up and down" },
    { how: "right", icon: "align-right", label: "Right", title: "Right edges in line" },
    { how: "top", icon: "align-top", label: "Top", title: "Top edges in line" },
    { how: "middle", icon: "align-middle", label: "Middle", title: "Middles in line, across" },
    { how: "bottom", icon: "align-bottom", label: "Bottom", title: "Bottom edges in line" },
    { how: "width", icon: "same-width", label: "Width", title: "The same width" },
    { how: "height", icon: "same-height", label: "Height", title: "The same height" },
    { how: "across", icon: "spread-across", label: "Across", title: "Spread evenly across (three or more)" },
    { how: "down", icon: "spread-down", label: "Down", title: "Spread evenly down (three or more)" },
  ];
  /** Lines the picked parts up with the one picked last: their edges or middles, its width or height; spread evenly across or down. */
  function align(how: Align) {
    if (!model || chosen.length < 2 || !picked || picked.part === null) return;
    const lead = model.bands[picked.band].parts[picked.part];
    const parts = chosen.map((c) => model!.bands[c.band].parts[c.part]);
    for (const p of parts) {
      if (how === "left") p.at = [lead.at[0], p.at[1]];
      if (how === "center") p.at = [snapped(lead.at[0] + lead.size[0] / 2 - p.size[0] / 2), p.at[1]];
      if (how === "right") p.at = [snapped(lead.at[0] + lead.size[0] - p.size[0]), p.at[1]];
      if (how === "top") p.at = [p.at[0], lead.at[1]];
      if (how === "middle") p.at = [p.at[0], snapped(lead.at[1] + lead.size[1] / 2 - p.size[1] / 2)];
      if (how === "bottom") p.at = [p.at[0], snapped(lead.at[1] + lead.size[1] - p.size[1])];
      if (how === "width") p.size = [lead.size[0], p.size[1]];
      if (how === "height") p.size = [p.size[0], lead.size[1]];
    }
    if (how === "across" || how === "down") {
      const axis = how === "across" ? 0 : 1;
      const sorted = [...parts].sort((a, b) => a.at[axis] - b.at[axis]);
      const first = sorted[0];
      const last = sorted[sorted.length - 1];
      const room = last.at[axis] + last.size[axis] - first.at[axis] - sorted.reduce((sum, p) => sum + p.size[axis], 0);
      const gap = room / (sorted.length - 1);
      let at = first.at[axis];
      for (const p of sorted) {
        const next: [number, number] = [...p.at];
        next[axis] = snapped(at);
        p.at = next;
        at += p.size[axis] + gap;
      }
    }
  }

  function moveDrag(e: PointerEvent) {
    if (!drag || !model) return;
    const dx = (e.clientX - drag.x) / zoom;
    const dy = (e.clientY - drag.y) / zoom;
    const band = model.bands[drag.band];
    if (drag.kind === "height") {
      band.height = onGrid(drag.from[1] + dy, 1);
    } else if (drag.part !== null) {
      const part = band.parts[drag.part];
      if (drag.kind === "move") {
        // Every picked part moves with the one dragged.
        const step: [number, number] = [onGrid(drag.from[0] + dx) - drag.from[0], onGrid(drag.from[1] + dy) - drag.from[1]];
        chosen.forEach((c, n) => {
          const from = froms[n];
          if (from) model!.bands[c.band].parts[c.part].at = [Math.max(0, snapped(from[0] + step[0])), Math.max(0, snapped(from[1] + step[1]))];
        });
        if (chosen.length === 0) part.at = [onGrid(drag.from[0] + dx), onGrid(drag.from[1] + dy)];
      } else {
        // A handle moves the sides it is on; the opposite sides stay where they are.
        const h = drag.handle ?? "se";
        const [x0, y0] = drag.at ?? part.at;
        const [w0, h0] = drag.from;
        let [x, y, w, hh] = [x0, y0, w0, h0];
        if (h.includes("e")) w = onGrid(w0 + dx, 1);
        if (h.includes("s")) hh = onGrid(h0 + dy, part.kind === "Line" ? 0 : 1);
        if (h.includes("w")) {
          x = Math.min(onGrid(x0 + dx), x0 + w0 - 1);
          w = x0 + w0 - x;
        }
        if (h.includes("n")) {
          y = Math.min(onGrid(y0 + dy), y0 + h0 - (part.kind === "Line" ? 0 : 1));
          hh = y0 + h0 - y;
        }
        part.at = [Math.max(0, snapped(x)), Math.max(0, snapped(y))];
        part.size = [snapped(w), snapped(hh)];
      }
    }
  }

  function endDrag() {
    if (drag) {
      drag = null;
      record();
    }
  }

  // Undo and redo of the designer's own steps (Ctrl+Z, Ctrl+Y): each a text of the report.
  let undone = $state<string[]>([]);
  let redone = $state<string[]>([]);
  /** The text the last step left. */
  let recorded = "";
  /** Keeps the step just made, unless a drag goes on (one drag is one step); then writes it to the file. */
  function record() {
    if (!model || drag) return;
    if (recorded && text !== recorded) {
      undone = [...undone.slice(-99), recorded];
      redone = [];
    }
    recorded = text;
    queueSave();
  }
  $effect(() => {
    if (text) record();
  });
  function stepBack(from: string[], to: string[]): [string[], string[]] {
    const last = from.at(-1);
    if (last === undefined || !model) return [from, to];
    to = [...to, text];
    model = readReport(last);
    recorded = last;
    queueSave();
    return [from.slice(0, -1), to];
  }
  function undo() {
    [undone, redone] = stepBack(undone, redone);
  }
  function redo() {
    [redone, undone] = stepBack(redone, undone);
  }

  /** Every change is written to the file a moment after it is made, as the window designer does. */
  let saving: ReturnType<typeof setTimeout> | null = null;
  function queueSave() {
    if (saving) clearTimeout(saving);
    saving = setTimeout(() => {
      saving = null;
      if (changed) void save();
    }, 400);
  }

  function addPart(kind: PartKind, band = picked?.band, at: [number, number] = [0, 0]) {
    if (!model || band === undefined) return;
    const into = model.bands[band];
    into.parts.push(newPart(kind, at));
    picked = { band, part: into.parts.length - 1 };
  }

  /** A part or band dragged from the palette: what, and where the pointer is. */
  let placing = $state<{ kind: PartKind | BandKind; band: boolean; x: number; y: number; moved: boolean; bind?: string; list?: boolean } | null>(null);

  function startPlacing(e: PointerEvent, kind: PartKind | BandKind, band: boolean, bind?: string, list?: boolean) {
    if (!model) return;
    // No text selection or the browser's own dragging while a tile is carried.
    e.preventDefault();
    placing = { kind, band, x: e.clientX, y: e.clientY, moved: false, bind, list };
  }

  function movePlacing(e: PointerEvent) {
    if (!placing) return;
    placing.moved ||= Math.abs(e.clientX - placing.x) + Math.abs(e.clientY - placing.y) > 4;
    placing.x = e.clientX;
    placing.y = e.clientY;
  }

  /** Let go: on a band a part lands where it was let go; a band lands on the paper; let go where it was picked up, it is a click. */
  function endPlacing(e: PointerEvent) {
    const was = placing;
    placing = null;
    if (!was || !model) return;
    // A list of the field list: a Detail band that repeats for its records.
    if (was.list && was.bind) {
      const detail = model.bands.find((b) => b.kind === "Detail");
      if (detail) detail.attrs.list = was.bind;
      else {
        addBand("Detail");
        model.bands[model.bands.length - 1].attrs.list = was.bind;
      }
      return;
    }
    if (!was.moved) {
      if (was.band) addBand(was.kind as BandKind);
      else if (!was.bind) addPart(was.kind as PartKind);
      return;
    }
    const under = document.elementFromPoint(e.clientX, e.clientY);
    if (was.band) {
      if (under?.closest(".paper")) addBand(was.kind as BandKind);
      return;
    }
    const bandEl = under?.closest("[data-band]") as HTMLElement | null;
    if (!bandEl) return;
    const box = bandEl.getBoundingClientRect();
    const at: [number, number] = [onGrid((e.clientX - box.left) / zoom), onGrid((e.clientY - box.top) / zoom)];
    addPart(was.kind as PartKind, Number(bandEl.dataset.band), at);
    // A field of the field list: a Field bound to it.
    if (was.bind && picked && picked.part !== null) model.bands[picked.band].parts[picked.part].attrs.bind = was.bind;
  }

  function addBand(kind: BandKind) {
    if (!model) return;
    // A group footer goes with the group header that has none yet.
    const by = kind === "GroupFooter" ? (model.bands.find((h) => h.kind === "GroupHeader" && !model!.bands.some((f) => f.kind === "GroupFooter" && f.attrs.by === h.attrs.by))?.attrs.by ?? "") : "";
    const attrs: Record<string, string> = kind === "Detail" ? { list: "" } : REPEATED.includes(kind) ? { by } : {};
    model.bands.push({ kind, height: kind === "Detail" ? 7 : kind === "PageFooter" || kind === "PageHeader" || REPEATED.includes(kind) ? 8 : 20, attrs, parts: [] });
    picked = { band: model.bands.length - 1, part: null };
  }

  /** The name a detail's binds start with for the records of a list: its entity's name in camelCase (`Line` as `line`). */
  function itemName(entity: string): string {
    return entity.charAt(0).toLowerCase() + entity.slice(1);
  }
  type FieldItem = { label: string; bind: string; list: boolean; note: string };
  /** The report's data as the field list shows it: each Data's fields, a record's fields under it, a list's records' fields under the list. */
  const fieldList = $derived.by(() => {
    if (!model) return [] as { source: string; type: string; groups: { title: string; items: FieldItem[] }[] }[];
    const entityOf = (n: string) => preview.project.files.flatMap((f) => f.entities).find((e) => e.name.name === n);
    const groups: { title: string; items: FieldItem[] }[] = [];
    const walk = (entity: string, path: string, depth: number, items: FieldItem[]) => {
      for (const f of entityOf(entity)?.fields ?? []) {
        const t = f.type;
        if (t.kind === "record" && depth < 2) walk(t.entity.name, `${path}.${f.name.name}`, depth + 1, items);
        else if (t.kind === "list") {
          items.push({ label: f.name.name, bind: `${path}.${f.name.name}`, list: true, note: `list of ${t.item.name}` });
          const item = itemName(t.item.name);
          const inner: FieldItem[] = [];
          walk(t.item.name, item, depth + 1, inner);
          groups.push({ title: `${path}.${f.name.name}: each ${item}`, items: inner.filter((i) => !i.list) });
        } else if (t.kind !== "ref" && t.kind !== "values" && t.kind !== "image" && t.kind !== "blob" && t.kind !== "id") items.push({ label: f.name.name, bind: `${path}.${f.name.name}`, list: false, note: t.kind });
      }
    };
    // One source per Data, each its own tab of the palette: its fields, then each list's records' fields.
    return model.data.map((d) => {
      groups.length = 0;
      const items: FieldItem[] = [];
      walk(d.type, d.name, 0, items);
      return { source: d.name, type: d.type, groups: [{ title: `${d.name}: ${d.type}`, items }, ...groups] };
    });
  });
  /** The palette's tab: parts, bands, or the fields of a source (a Data's name). */
  let paletteTab = $state<string>("parts");

  /** A part cut or copied, to paste into another band. */
  let clipboard = $state<ReportPart[] | null>(null);

  function copyPicked(cut: boolean) {
    if (!model || !picked || picked.part === null) return;
    const taken = chosen.length > 0 ? chosen : [{ band: picked.band, part: picked.part }];
    clipboard = JSON.parse(JSON.stringify(taken.map((c) => model!.bands[c.band].parts[c.part])));
    if (cut) removePicked();
  }

  /** Pastes the part cut or copied into the picked band, at its place there (kept inside the band). */
  function paste() {
    if (!model || !clipboard || !picked) return;
    const band = model.bands[picked.band];
    const into = picked.band;
    const pasted: { band: number; part: number }[] = [];
    for (const kept of clipboard) {
      const part: ReportPart = JSON.parse(JSON.stringify(kept));
      part.at = [part.at[0], Math.min(part.at[1], Math.max(0, band.height - part.size[1]))];
      band.parts.push(part);
      pasted.push({ band: into, part: band.parts.length - 1 });
    }
    chosen = pasted;
    picked = pasted.at(-1) ?? picked;
  }

  $effect(() => {
    reportDesigner.actions = {
      undo,
      redo,
      cut: () => copyPicked(true),
      copy: () => copyPicked(false),
      paste,
      remove: removePicked,
      align,
      showReport: () => {
        picked = null;
        chosen = [];
        reportDesigner.xml = false;
        showProperties = true;
      },
      deleteReport: () => void remove(),
    };
    return () => {
      reportDesigner.actions = null;
      reportDesigner.previewing = false;
      reportDesigner.xml = false;
    };
  });
  $effect(() => {
    reportDesigner.canUndo = undone.length > 0;
    reportDesigner.canRedo = redone.length > 0;
    reportDesigner.canPaste = clipboard !== null && picked !== null;
    reportDesigner.picked = chosen.length || (picked ? 1 : 0);
  });
  // The ribbon's Preview shows the laid-out pages; its XML opens the slide-out on the report's file.
  $effect(() => {
    if (reportDesigner.previewing) void showPreview();
  });
  $effect(() => {
    if (reportDesigner.xml) showProperties = true;
  });

  /** Pixels per millimetre at 100%: a millimetre on the paper as large as on a screen of 96 pixels an inch. */
  const REAL = 96 / 25.4;

  /** The wheel zooms the paper. */
  function wheel(e: WheelEvent) {
    e.preventDefault();
    zoom = Math.min(REAL * 3, Math.max(REAL / 4, zoom * (e.deltaY < 0 ? 1.1 : 1 / 1.1)));
  }

  /** The designer's keys: Ctrl+Z and Ctrl+Y (or Ctrl+Shift+Z) undo and redo, Ctrl+X, Ctrl+C and Ctrl+V cut, copy and paste a part,
   *  Delete removes what is picked, the arrows move the picked part by the snap step (Shift: ten). Not while typing in a field. */
  function key(e: KeyboardEvent) {
    if (source || previewing || !model) return;
    if (document.querySelector('[role="dialog"][aria-modal="true"]')) return;
    const target = e.target as HTMLElement | null;
    if (target && target.closest("input, select, textarea, [contenteditable], .cm-editor") !== null) return;
    const ctrl = e.ctrlKey || e.metaKey;
    const k = e.key.toLowerCase();
    const part = picked && picked.part !== null ? model.bands[picked.band].parts[picked.part] : null;
    const step = (grid || 1) * (e.shiftKey ? 10 : 1);
    if (ctrl && k === "z" && !e.shiftKey) undo();
    else if (ctrl && (k === "y" || (k === "z" && e.shiftKey))) redo();
    else if (ctrl && k === "x") copyPicked(true);
    else if (ctrl && k === "c") copyPicked(false);
    else if (ctrl && k === "v") paste();
    else if (ctrl && k === "s") void save();
    else if ((e.key === "Delete" || e.key === "Backspace") && picked) removePicked();
    else if (part && e.key.startsWith("Arrow")) {
      const [dx, dy] = e.key === "ArrowLeft" ? [-step, 0] : e.key === "ArrowRight" ? [step, 0] : e.key === "ArrowUp" ? [0, -step] : [0, step];
      const moved = chosen.length > 0 ? chosen.map((c) => model!.bands[c.band].parts[c.part]) : [part];
      for (const p of moved) p.at = [Math.max(0, snapped(p.at[0] + dx)), Math.max(0, snapped(p.at[1] + dy))];
    } else if (ctrl && k === "a" && picked) {
      // Ctrl+A picks every part of the picked band.
      chosen = model.bands[picked.band].parts.map((_, part) => ({ band: picked!.band, part }));
    }
    else return;
    // The IDE's own keys (its Undo of files, its Delete) do not run as well.
    e.preventDefault();
    e.stopImmediatePropagation();
  }
  $effect(() => {
    window.addEventListener("keydown", key, true);
    return () => window.removeEventListener("keydown", key, true);
  });

  function removePicked() {
    if (!model || !picked) return;
    if (chosen.length > 1) {
      // From the last to the first, so the places of those still to go stay right.
      for (const c of [...chosen].sort((a, b) => b.band - a.band || b.part - a.part)) model.bands[c.band].parts.splice(c.part, 1);
    } else if (picked.part !== null) model.bands[picked.band].parts.splice(picked.part, 1);
    else model.bands.splice(picked.band, 1);
    chosen = [];
    picked = null;
  }

  function addData() {
    if (!model) return;
    model.data.push({ name: "", type: entities[0] ?? "" });
  }

  async function save() {
    if (!model) return;
    // What is written is the text of now; a change made while it is written is written next.
    const written = text;
    try {
      const next = await invoke<Preview>("save_report", { name, text: written });
      saved = written;
      onapplied(next);
    } catch (e) {
      await dialogs.message({ title: "Save report", text: String(e) });
    }
  }

  async function remove() {
    if (!(await dialogs.confirm({ title: "Delete report", text: `Delete the report ${name}? Undo brings it back.`, action: "Delete", danger: true }))) return;
    try {
      ondeleted(await invoke<Preview>("delete_report", { name }));
    } catch (e) {
      await dialogs.message({ title: "Delete report", text: String(e) });
    }
  }

  const previewing = $derived(reportDesigner.previewing);
  let pages = $state<string[]>([]);
  let previewProblem = $state<string | null>(null);
  async function showPreview() {
    reportDesigner.previewing = true;
    try {
      const shown = await invoke<ReportPreview>("report_preview", { name, text });
      pages = shown.pages;
      previewProblem = shown.problem;
    } catch (e) {
      pages = [];
      previewProblem = String(e);
    }
  }
  // While the preview shows, it follows every change.
  $effect(() => {
    if (previewing && text) void showPreview();
  });

  const pickedBand = $derived(model && picked ? model.bands[picked.band] : null);
  const pickedPart = $derived(pickedBand && picked && picked.part !== null ? pickedBand.parts[picked.part] : null);
  /** The words a part of a kind has besides its place and size. */
  function wordsOf(kind: PartKind): string[] {
    const own = kind === "Text" ? ["key"] : kind === "Field" ? ["bind", "format"] : kind === "Markdown" ? ["bind"] : kind === "Sum" ? ["of", "format"] : kind === "Image" ? ["source", "bind", "barcode"] : [];
    if (kind === "Line" || kind === "Box") return ["class"];
    return [...own, "align", "class"];
  }
</script>

<svelte:window
  onpointermove={(e) => {
    movePlacing(e);
    moveMarquee(e);
  }}
  onpointerup={(e) => {
    endPlacing(e);
    endMarquee();
  }}
/>

<div class="report">
  {#if marquee}
    <div class="marquee" style:left="{Math.min(marquee.x0, marquee.x1)}px" style:top="{Math.min(marquee.y0, marquee.y1)}px" style:width="{Math.abs(marquee.x1 - marquee.x0)}px" style:height="{Math.abs(marquee.y1 - marquee.y0)}px"></div>
  {/if}
  {#if placing?.moved}
    <div class="ghost" style:left="{placing.x + 8}px" style:top="{placing.y + 8}px">{placing.kind}</div>
  {/if}
  <div class="bar">
    <strong>{name}</strong>
    <span class="file">reports/{name}.report</span>
    <span class="spacer"></span>
    {#if chosen.length > 1}<span class="file">{chosen.length} parts picked: line them up in the Align tab</span>{/if}
    <span class="file" title="Every change is written to the file as you make it">{changed ? "Saving..." : "Saved"}</span>
  </div>

  {#if problem && !source}
    <p class="problem">This report cannot be designed: {problem}. Open XML in the ribbon to put it right.</p>
  {:else if model}
    <div class="body">
      <aside class="palette" style:width="{layout.sizes.reportPalette}px">
        <div class="palette-tabs" role="tablist">
          <button role="tab" aria-selected={paletteTab === "parts"} class:active={paletteTab === "parts"} onclick={() => (paletteTab = "parts")}>Parts</button>
          <button role="tab" aria-selected={paletteTab === "bands"} class:active={paletteTab === "bands"} onclick={() => (paletteTab = "bands")}>Bands</button>
          <button role="tab" aria-selected={paletteTab === "styles"} class:active={paletteTab === "styles"} onclick={() => (paletteTab = "styles")}>Styles</button>
          <button role="tab" aria-selected={paletteTab === "align"} class:active={paletteTab === "align"} onclick={() => (paletteTab = "align")}>Align</button>
          {#each fieldList as src (src.source)}
            <button role="tab" aria-selected={paletteTab === src.source} class:active={paletteTab === src.source} title="The fields of {src.source}, a {src.type}" onclick={() => (paletteTab = src.source)}>{src.source}</button>
          {/each}
        </div>
        {#if paletteTab === "parts"}
          <div class="tiles">
            {#each PARTS as kind (kind)}
              <PartTile kind={TILES[kind]} name={kind} title="Drag it onto a band, or pick a band and click" onpointerdown={(e) => startPlacing(e, kind, false)} />
            {/each}
          </div>
          {#if fieldList.length === 0}<p class="note">Fields get a tab per source once the report has data: pick the paper, then Add data.</p>{/if}
        {:else if paletteTab === "align"}
          <div class="tiles">
            {#each ALIGNS as a (a.how)}
              <button class="tile align-tile" title={a.title} disabled={chosen.length < (a.how === "across" || a.how === "down" ? 3 : 2)} onclick={() => align(a.how)}>
                <IdeIcon name={a.icon} size={22} />
                <span>{a.label}</span>
              </button>
            {/each}
          </div>
          <p class="note">{chosen.length < 2 ? "Pick two or more parts (Shift+click, or draw a box round them) to line them up with the one picked last." : `${chosen.length} parts picked; they line up with the one picked last.`}</p>
        {:else if paletteTab === "styles"}
          <label class="style-pick">Rule
            <select bind:value={styleRule}>
              {#each styleRules as r (r)}<option value={r}>{r}</option>{/each}
            </select>
          </label>
          <ReportLook {css} selector={styleRule} onset={(prop, value) => setLook(styleRule, prop, value)} />
          <p class="note">The report's stylesheet{model?.attrs.stylesheet ? `, ${model.attrs.stylesheet}` : ""}: body is every text's look, h1 to h3, p and the table's its Markdown's, a class (.title) the parts that name it.</p>
        {:else if paletteTab === "bands"}
          <div class="tiles">
            {#each missing as kind (kind)}
              <PartTile kind={TILES[kind]} name={kind} title="Drag it onto the paper, or click" onpointerdown={(e) => startPlacing(e, kind, true)} />
            {/each}
          </div>
          {#if missing.length === 0}<p class="note">Every band is there.</p>{/if}
        {:else}
          {#each fieldList.find((f) => f.source === paletteTab)?.groups ?? [] as group (group.title)}
            <div class="field-group">{group.title}</div>
            {#each group.items as f (f.bind)}
              <button class="field" title={f.list ? "Drag it onto the paper: a Detail band repeats for its records" : `Drag it onto a band: a Field bound to ${f.bind}`} onpointerdown={(e) => startPlacing(e, "Field", false, f.bind, f.list)}>
                {f.label}<span class="note">{f.note}</span>
              </button>
            {/each}
          {/each}
        {/if}
      </aside>
      <Splitter axis="x" label="Width of the palette" onmove={(delta) => layout.resize("reportPalette", delta)} />

      <div class="canvas-room">
      <ZoomBar zoom={zoom / REAL} onzoom={(f) => (zoom = Math.min(REAL * 3, Math.max(REAL / 4, zoom * f)))} onreset={() => (zoom = REAL)} oncenter={() => canvasEl?.scrollTo({ left: (canvasEl.scrollWidth - canvasEl.clientWidth) / 2, top: 0, behavior: "smooth" })} />
      <div class="canvas" bind:this={canvasEl} onpointermove={moveDrag} onpointerup={endDrag} onwheel={wheel} role="presentation">
        {#if previewing}
          {#if previewProblem}<p class="problem">{previewProblem}</p>{/if}
          {#each pages as page, i (i)}
            <div class="page-shown" style:width="{paper[0] * zoom}px">{@html page}</div>
          {/each}
        {:else}
          <div class="ruler" style:width="{paper[0] * zoom}px">
            {#each Array.from({ length: Math.floor(paper[0] / 10) + 1 }, (_, n) => n * 10) as at (at)}
              <span class="tick" style:left="{at * zoom}px">{at}</span>
            {/each}
          </div>
          <div
            class="paper"
            class:gridded={reportDesigner.showGrid}
            style:background-size={reportDesigner.showGrid ? `${dotStep * zoom}px ${dotStep * zoom}px` : undefined}
            style:background-position="{margin * zoom}px {margin * zoom}px"
            style:width="{paper[0] * zoom}px"
            style:height="{paperHeight}px" onpointerdown={() => (picked = null)} role="presentation">
            {#each ordered as { b, i } (i)}
              <div class="band-bar" class:picked={picked?.band === i} style:left="0" style:top="{tops[i] - BAR}px" style:width="{paper[0] * zoom}px" style:height="{BAR}px">
                {b.kind}{b.kind === "Detail" && b.attrs.list ? `: once per ${b.attrs.list}` : ""}{REPEATED.includes(b.kind) ? ` by ${b.attrs.by || "?"}` : ""}
              </div>
              <div
                class="band"
                data-band={i}
                class:picked={picked?.band === i && picked.part === null}
                style:left="{margin * zoom}px"
                style:top="{tops[i]}px"
                style:width="{(paper[0] - 2 * margin) * zoom}px"
                style:height="{b.height * zoom}px"
                onpointerdown={(e) => {
                  e.stopPropagation();
                  picked = { band: i, part: null };
                  chosen = [];
                  startMarquee(e);
                }}
                role="presentation"
              >
                {#each b.parts as p, j (j)}
                  {@const look = partLook(rules, p.attrs.class)}
                  <div
                    class="part"
                    class:picked={(picked?.band === i && picked.part === j) || isChosen(i, j)}
                    data-key="{i}:{j}"
                    style:left="{p.at[0] * zoom}px"
                    style:top="{p.at[1] * zoom}px"
                    style:width="{p.size[0] * zoom}px"
                    style:height="{p.size[1] * zoom}px"
                    style:text-align={p.attrs.align || look.align || "left"}
                    style:font-size="{(look.sizePt * 25.4 * zoom) / 72}px"
                    style:font-weight={look.bold ? "700" : "400"}
                    style:font-style={look.italic ? "italic" : "normal"}
                    style:color={look.color}
                    style:background={look.background ?? "transparent"}
                    style:font-family={look.font}
                    style:line-height="{((look.sizePt * 25.4 * zoom) / 72) * 1.2}px"
                    class:bound={p.kind === "Field" || p.kind === "Markdown" || p.kind === "Sum"}
                    class:picture={p.kind === "Image"}
                    class:drawn-line={p.kind === "Line"}
                    class:drawn-box={p.kind === "Box"}
                    onpointerdown={(e) => startDrag(e, "move", i, j)}
                    role="presentation"
                  >
                    <span class="shown">{shown(p)}</span>
                    {#if picked?.band === i && picked.part === j && chosen.length <= 1}
                      {#each HANDLES as h (h)}
                        <span class="handle {h}" onpointerdown={(e) => startDrag(e, "size", i, j, h)} role="presentation"></span>
                      {/each}
                    {/if}
                  </div>
                {/each}
                <span class="edge" onpointerdown={(e) => startDrag(e, "height", i, null)} role="presentation"></span>
              </div>
            {/each}
          </div>
        {/if}
      </div>
      </div>

      {#if showProperties}
      <aside class="properties" aria-label="Properties" style:width="{layout.sizes.reportProperties}px">
        <div class="sizer"><Splitter axis="x" label="Width of the properties" onmove={(delta) => layout.resize("reportProperties", -delta)} /></div>
        <div class="inspector">
        <div class="inspector-bar">
          <div class="inspector-tabs" role="tablist">
            <button role="tab" aria-selected={!source && !looking} class:active={!source && !looking} onclick={() => ((reportDesigner.xml = false), (looking = false))}>Properties</button>
            <button role="tab" aria-selected={!source && looking} class:active={!source && looking} onclick={() => ((reportDesigner.xml = false), (looking = true))}>Look</button>
            <button role="tab" aria-selected={source} class:active={source} onclick={() => (reportDesigner.xml = true)}>XML</button>
          </div>
          <button class="undock" title="Slide the properties away" onclick={() => { showProperties = false; reportDesigner.xml = false; }}>Hide &rsaquo;</button>
        </div>
        {#if source}
          <div class="source">
            <SourceEditor
              load={async () => ({ text, problems: problemsOf(text) })}
              check={async (t) => problemsOf(t)}
              save={async (t) => {
                readReport(t);
                const next = await invoke<Preview>("save_report", { name, text: t });
                saved = "";
                onapplied(next);
              }}
              reloadKey={`${name}:${saved}`}
              language="cxaml"
              errorsBlockSave={true}
            />
          </div>
        {:else if looking}
          {#if pickedPart && pickedPart.kind !== "Image"}
            <h4>{pickedPart.kind} <span class="note">class .{pickedPart.attrs.class || "(new)"}</span></h4>
            <p class="note">{pickedPart.attrs.class ? "Its class's look, shared by every part of it." : "A change gives it a class of its own, named after it."}</p>
            <ReportLook {css} selector=".{pickedPart.attrs.class || ""}" onset={(prop, value) => setLook(`.${classOf(pickedPart!)}`, prop, value)} />
          {:else}
            <label class="style-pick">Rule
              <select bind:value={styleRule}>
                {#each styleRules as r (r)}<option value={r}>{r}</option>{/each}
              </select>
            </label>
            <ReportLook {css} selector={styleRule} onset={(prop, value) => setLook(styleRule, prop, value)} />
            <p class="note">Pick a part for its own look; here every rule of the stylesheet: body for every text, h1 to h3, p and the table for Markdown.</p>
          {/if}
        {:else}
        {#if pickedPart}
          <h4>{pickedPart.kind}</h4>
          <label>At (mm) <span class="pair"><input type="number" step="0.5" bind:value={pickedPart.at[0]} /><input type="number" step="0.5" bind:value={pickedPart.at[1]} /></span></label>
          <label>Size (mm) <span class="pair"><input type="number" step="0.5" min="1" bind:value={pickedPart.size[0]} /><input type="number" step="0.5" min="1" bind:value={pickedPart.size[1]} /></span></label>
          {#each wordsOf(pickedPart.kind) as word (word)}
            <label>
              {word}
              {#if word === "align"}
                <select value={pickedPart.attrs.align ?? ""} onchange={(e) => (pickedPart.attrs.align = e.currentTarget.value)}>
                  <option value="">left</option>
                  <option value="center">center</option>
                  <option value="right">right</option>
                </select>
              {:else}
                <input bind:value={pickedPart.attrs[word]} placeholder={word === "bind" ? "order.number" : word === "of" ? "line.price" : word === "key" ? "invoiceTitle" : word === "format" ? "as its type: 0.00, dd-MM-yyyy" : word === "barcode" ? "qr, code128 or ean13" : "title"} spellcheck="false" />
                {#if word === "key" && pickedPart.attrs.key}
                  <!-- The key's text in each language of the project, the default first, written into its translations on the spot. -->
                  {@const key = pickedPart.attrs.key}
                  {@const others = languages.filter((l) => l !== language).sort((a, b) => Number(!!preview.project.translations[a]?.[key]) - Number(!!preview.project.translations[b]?.[key]) || a.localeCompare(b))}
                  {@const missing = others.filter((l) => !preview.project.translations[l]?.[key]).length}
                  <span class="text-of">
                    <span class="lang">{language}</span>
                    <input value={preview.project.translations[language]?.[key] ?? ""} placeholder="its text: Invoice {'{'}order.number{'}'}" title="The text of {key} in {language}, the default language" onchange={(e) => void setText(key, e.currentTarget.value, language)} />
                  </span>
                  {#if others.length > 0}
                    <!-- The other languages folded away, the missing ones first, scrolling past a few: thirty languages are one line until opened. -->
                    <details class="others" open={missing > 0 && others.length <= 3}>
                      <summary>Other languages {missing > 0 ? `(${missing} of ${others.length} missing)` : `(${others.length})`}</summary>
                      <div class="others-list">
                        {#each others as lang (lang)}
                          <span class="text-of" class:missing={!preview.project.translations[lang]?.[key]}>
                            <span class="lang">{lang}</span>
                            <input value={preview.project.translations[lang]?.[key] ?? ""} placeholder="in {lang}" title="The text of {key} in {lang}" onchange={(e) => void setText(key, e.currentTarget.value, lang)} />
                          </span>
                        {/each}
                      </div>
                    </details>
                  {/if}
                {/if}
              {/if}
            </label>
          {/each}
          <div class="row">
            <button onclick={() => copyPicked(true)} title="Ctrl+X: cut it, to paste into another band">Cut</button>
            <button onclick={() => copyPicked(false)} title="Ctrl+C">Copy</button>
          </div>
          <button class="danger" onclick={removePicked} title="Delete">Remove the part</button>
        {:else if pickedBand}
          <h4>{pickedBand.kind}</h4>
          <label>Height (mm) <input type="number" step="0.5" min="1" bind:value={pickedBand.height} /></label>
          {#if pickedBand.kind === "Detail"}
            <label>List <input bind:value={pickedBand.attrs.list} placeholder="order.lines" spellcheck="false" /></label>
          {/if}
          {#if REPEATED.includes(pickedBand.kind)}
            <label>By <input bind:value={pickedBand.attrs.by} placeholder="line.category" spellcheck="false" /></label>
            <p class="note">A new group starts where this value changes in the detail's list, as code gathered it. A Sum here adds the group.</p>
          {/if}
          <p class="note">Drag a part from the palette onto it{clipboard ? ", or paste the part you cut" : ""}.</p>
          {#if clipboard}<button onclick={paste} title="Ctrl+V">Paste {clipboard.length === 1 ? clipboard[0].kind : `${clipboard.length} parts`}</button>{/if}
          <button class="danger" onclick={removePicked}>Remove the band</button>
        {:else}
          <h4>Report</h4>
          <label>Paper
            <select bind:value={model.attrs.paper}>
              {#each ["A3", "A4", "A5", "Letter", "Legal", "A4 landscape", "A5 landscape", "Letter landscape"] as p (p)}<option value={p}>{p}</option>{/each}
            </select>
          </label>
          <label>Margin <input bind:value={model.attrs.margin} placeholder="20mm" /></label>
          <label>Stylesheet <input bind:value={model.attrs.stylesheet} placeholder="invoice.css" spellcheck="false" /></label>
          <label title="Which header tops the first page; later pages start with the page header">Header first
            <select
              value={model.attrs.headerFirst === "false" ? "false" : "true"}
              onchange={(e) => {
                if (e.currentTarget.value === "false") model!.attrs.headerFirst = "false";
                else delete model!.attrs.headerFirst;
              }}
            >
              <option value="true">Report header, then page header</option>
              <option value="false">Page header, then report header</option>
            </select>
          </label>
          <h4>Data</h4>
          {#each model.data as d, i (i)}
            <div class="data">
              <input bind:value={d.name} placeholder="order" spellcheck="false" />
              <select bind:value={d.type}>{#each entities as e (e)}<option value={e}>{e}</option>{/each}</select>
              <button title="Remove" onclick={() => model!.data.splice(i, 1)}>×</button>
            </div>
          {/each}
          <button onclick={addData} disabled={entities.length === 0}>Add data</button>
        {/if}
        {/if}
        </div>
      </aside>
      {/if}
    </div>
  {/if}
</div>

<style>
  .marquee {
    position: fixed;
    z-index: 900;
    pointer-events: none;
    border: 1px dashed var(--accent);
    background: rgba(31, 95, 191, 0.08);
  }
  .ghost {
    position: fixed;
    z-index: 1000;
    pointer-events: none;
    padding: 2px 8px;
    border: 1px solid var(--accent);
    background: var(--panel);
    border-radius: var(--radius);
    font-size: 12px;
  }
  .report {
    min-width: 0;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  .file {
    color: var(--text-muted);
    font-size: 0.85em;
  }
  .spacer {
    flex: 1;
  }
  .body {
    position: relative;
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .palette,
  .properties {
    flex: none;
    width: 200px;
    /* Never narrower than two tiles side by side: 12 + 76 + 8 + 76 + 12, and room for a scroll bar. */
    min-width: 200px;
    /* The same room on every side; the splitter beside it is its edge. */
    padding: 12px;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow: auto;
  }
  .properties {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    z-index: 5;
    flex-direction: row;
    padding: 0;
    gap: 0;
    min-width: 240px;
    background: var(--panel);
    box-shadow: -4px 0 12px rgba(0, 0, 0, 0.12);
  }
  /* Its left edge sizes it, as the window designer's inspector is sized. */
  .sizer {
    display: flex;
    flex: none;
  }
  .inspector {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 12px 12px;
    overflow: auto;
  }
  .undock {
    padding: 2px 8px;
    font-size: 11px;
    border: none;
    background: none;
    color: var(--text-muted);
  }
  .undock:hover {
    color: var(--accent);
  }
  h4 {
    margin: 6px 0 2px;
  }
  /* The zoom bar at the bottom right, out of the way of the ruler and the paper's top. */
  .canvas-room :global(.zoom) {
    top: auto;
    left: auto;
    right: 18px;
    bottom: 18px;
    z-index: 4;
  }
  .canvas-room {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
  }
  .inspector-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--border);
  }
  .inspector-tabs button {
    border: none;
    border-bottom: 2px solid transparent;
    background: none;
    border-radius: 0;
    padding: 4px 8px;
  }
  .inspector-tabs button.active {
    border-bottom-color: var(--accent);
    color: var(--accent);
  }
  .canvas {
    flex: 1;
    overflow: auto;
    background: var(--panel);
    padding: 16px;
  }
  .paper,
  .page-shown {
    position: relative;
    margin: 0 auto 16px;
    background: #fff;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.25);
  }
  .page-shown :global(svg) {
    width: 100%;
    height: auto;
    display: block;
  }
  .band {
    position: absolute;
    outline: 1px dashed #c8ccd4;
  }
  .band-bar {
    position: absolute;
    box-sizing: border-box;
    padding: 2px 8px;
    font-size: 11px;
    color: var(--text-muted);
    background: var(--panel);
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    white-space: nowrap;
    overflow: hidden;
  }
  .band-bar.picked {
    color: var(--accent);
  }
  .ruler {
    position: relative;
    height: 16px;
    margin: 0 auto;
    border-bottom: 1px solid var(--border);
    font-size: 9px;
    color: var(--text-muted);
  }
  .tick {
    position: absolute;
    bottom: 0;
    /* The digit on the left of its line, the line at its millimetre. */
    transform: translateX(-100%);
    padding-right: 2px;
    border-right: 1px solid var(--border);
    height: 10px;
    line-height: 10px;
  }
  .paper.gridded {
    background-image: radial-gradient(circle, #c8ccd4 0.8px, transparent 0.9px);
  }
  .palette-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 6px;
  }
  .palette-tabs button {
    border: none;
    border-bottom: 2px solid transparent;
    background: none;
    padding: 4px 6px;
    border-radius: 0;
  }
  .palette-tabs button.active {
    border-bottom-color: var(--accent);
    color: var(--accent);
  }
  .text-of {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 4px;
  }
  .others summary {
    margin-top: 4px;
    font-size: 0.85em;
    color: var(--text-muted);
    cursor: pointer;
  }
  .others-list {
    max-height: 180px;
    overflow: auto;
    padding-right: 2px;
  }
  .text-of.missing .lang {
    color: var(--error);
  }
  .text-of .lang {
    width: 22px;
    font-size: 0.85em;
    color: var(--text-muted);
  }
  .text-of input {
    flex: 1;
    min-width: 0;
  }
  .field-group {
    margin-top: 4px;
    font-size: 0.85em;
    color: var(--text-muted);
  }
  .field {
    display: flex;
    justify-content: space-between;
    gap: 6px;
    text-align: left;
    cursor: grab;
    touch-action: none;
  }
  .row {
    display: flex;
    gap: 4px;
  }
  .row button {
    flex: 1;
  }
  /* Square tiles of one size, as many on a row as the palette is wide. */
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fill, 76px);
    gap: 8px;
  }
  .align-tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    background: var(--panel-alt);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
  }
  .align-tile:disabled {
    opacity: 0.45;
  }
  .tiles :global(.tile) {
    width: 76px;
    height: 76px;
    box-sizing: border-box;
    justify-content: center;
  }
  .source {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .part.drawn-line {
    outline: none;
    border-top: 1px solid currentColor;
    min-height: 4px;
  }
  .part.drawn-box {
    outline: none;
    border: 1px solid currentColor;
  }
  .part.picture {
    display: flex;
    align-items: center;
    justify-content: center;
    background: repeating-linear-gradient(45deg, #f1f3f6, #f1f3f6 6px, #e6e9ee 6px, #e6e9ee 12px) !important;
    color: var(--text-muted);
  }
  .part.bound {
    font-style: italic;
  }
  .band.picked {
    background: rgba(31, 95, 191, 0.1);
  }
  .part {
    position: absolute;
    box-sizing: border-box;
    outline: 1px dotted #9aa3b2;
    overflow: hidden;
    white-space: nowrap;
    padding: 0 2px;
    cursor: move;
  }
  .part.picked {
    outline: 1px solid var(--accent);
  }
  .part {
    overflow: visible;
  }
  /* What a part shows is cut off at its box, as it prints; its handles stand out of it. */
  .shown {
    display: block;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }
  .handle {
    position: absolute;
    width: 7px;
    height: 7px;
    box-sizing: border-box;
    background: var(--panel);
    border: 1px solid var(--accent);
    z-index: 2;
  }
  .handle.nw { left: -4px; top: -4px; cursor: nwse-resize; }
  .handle.n { left: calc(50% - 4px); top: -4px; cursor: ns-resize; }
  .handle.ne { right: -4px; top: -4px; cursor: nesw-resize; }
  .handle.e { right: -4px; top: calc(50% - 4px); cursor: ew-resize; }
  .handle.se { right: -4px; bottom: -4px; cursor: nwse-resize; }
  .handle.s { left: calc(50% - 4px); bottom: -4px; cursor: ns-resize; }
  .handle.sw { left: -4px; bottom: -4px; cursor: nesw-resize; }
  .handle.w { left: -4px; top: calc(50% - 4px); cursor: ew-resize; }
  .edge {
    position: absolute;
    left: 0;
    right: 0;
    bottom: -3px;
    height: 6px;
    cursor: ns-resize;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 0.9em;
  }
  .pair {
    display: flex;
    gap: 4px;
  }
  .pair input {
    width: 50%;
  }
  .data {
    display: flex;
    gap: 4px;
  }
  .data input {
    width: 40%;
  }
  .note {
    color: var(--text-muted);
    font-size: 0.85em;
  }
  .problem {
    color: var(--error);
    padding: 10px;
  }
</style>
