// Which module draws a part: one per kind, and for a dropdown, a date, an
// image, a chart and a canvas the one its options call for (DESIGN 7.8, G6a).

import type { Component } from "svelte";
import type { PartContext } from "./context.ts";
import Accordion from "./Accordion.svelte";
import Badge from "./Badge.svelte";
import Busy from "./Busy.svelte";
import Button from "./Button.svelte";
import ButtonGroup from "./ButtonGroup.svelte";
import Canvas from "./Canvas.svelte";
import CanvasPlaceholder from "./CanvasPlaceholder.svelte";
import Chart from "./Chart.svelte";
import Check from "./Check.svelte";
import ColorPart from "./ColorPart.svelte";
import CustomPart from "./CustomPart.svelte";
import DatePart from "./DatePart.svelte";
import DateTimePart from "./DateTimePart.svelte";
import Display from "./Display.svelte";
import Dropdown from "./Dropdown.svelte";
import EngineCanvasPart from "./EngineCanvasPart.svelte";
import FilePart from "./FilePart.svelte";
import FolderPart from "./FolderPart.svelte";
import Heading from "./Heading.svelte";
import Help from "./Help.svelte";
import IconPart from "./IconPart.svelte";
import Label from "./Label.svelte";
import Link from "./Link.svelte";
import List from "./List.svelte";
import Markdown from "./Markdown.svelte";
import NumberBox from "./NumberBox.svelte";
import NumbersChart from "./NumbersChart.svelte";
import Panel from "./Panel.svelte";
import Picture from "./Picture.svelte";
import PictureField from "./PictureField.svelte";
import Progress from "./Progress.svelte";
import Radio from "./Radio.svelte";
import ScrollBox from "./ScrollBox.svelte";
import SearchDropdown from "./SearchDropdown.svelte";
import Separator from "./Separator.svelte";
import Slider from "./Slider.svelte";
import SpinBox from "./SpinBox.svelte";
import Splitter from "./Splitter.svelte";
import Steps from "./Steps.svelte";
import TablePart from "./TablePart.svelte";
import Tabs from "./Tabs.svelte";
import Tags from "./Tags.svelte";
import TextBox from "./TextBox.svelte";
import TextPart from "./TextPart.svelte";
import Toggle from "./Toggle.svelte";
import Tree from "./Tree.svelte";
import Trend from "./Trend.svelte";
import TypedDropdown from "./TypedDropdown.svelte";

export type KindModule = Component<{ ctx: PartContext }>;

/** One module for each kind that needs nothing more than its kind to choose it. */
const BY_KIND: Record<string, KindModule> = {
  label: Label,
  help: Help,
  separator: Separator,
  badge: Badge,
  link: Link,
  progress: Progress,
  busy: Busy,
  icon: IconPart,
  heading: Heading,
  text: TextPart,
  markdown: Markdown,
  display: Display,
  textbox: TextBox,
  password: TextBox,
  memo: TextBox,
  number: NumberBox,
  spinbox: SpinBox,
  file: FilePart,
  folder: FolderPart,
  tags: Tags,
  color: ColorPart,
  slider: Slider,
  checkbox: Check,
  switch: Check,
  radio: Radio,
  button: Button,
  toggle: Toggle,
  accordion: Accordion,
  scroll: ScrollBox,
  buttons: ButtonGroup,
  panel: Panel,
  form: Panel,
  tabs: Tabs,
  steps: Steps,
  splitter: Splitter,
  table: TablePart,
  tree: Tree,
  treeTable: Tree,
  trend: Trend,
  list: List,
  custom: CustomPart,
};

/** The module that draws the part; none for a kind the screen does not draw (a sound, a timer). */
export function kindModule(ctx: PartContext): KindModule | null {
  const { p, bound, design, preview, partState } = ctx;
  switch (p.kind) {
    case "date":
      return bound?.field.type.kind === "dateTime" ? DateTimePart : DatePart;
    case "dropdown":
      if (p.typed) return TypedDropdown;
      if (p.search && bound?.field.type.kind === "ref") return SearchDropdown;
      return Dropdown;
    case "chart":
      if (!p.chart) return null;
      return bound?.field.type.kind === "values" ? NumbersChart : Chart;
    case "image":
      return p.bind ? PictureField : Picture;
    case "canvas":
      if (p.runs && !design && preview.screenEngines.some((e) => e.name === p.runs)) return EngineCanvasPart;
      if (partState?.drawing || p.mode) return Canvas;
      return CanvasPlaceholder;
    default:
      return BY_KIND[p.kind] ?? null;
  }
}
