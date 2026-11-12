// The guided sign-in in the designer (docs/APP-CAPABILITIES-PLAN.md, "The
// sign-in window is designed, and guided"): which views are sign-in views
// (the views the sign-in window's content shows), and the palette of a
// sign-in view, made from the project's sign-in setup: a field per input of
// its strategies, a button per strategy, keep me signed in and the language
// dropdown when the setup turns them on, an "other way" to each other
// sign-in view, and the refusal. The checker holds the same rules.

import type { Page } from "@clarity/ui/generated/Page.ts";
import type { Signin } from "@clarity/ui/generated/Signin.ts";
import type { Widget } from "@clarity/ui/generated/Widget.ts";
import type { WidgetKind } from "@clarity/ui/generated/WidgetKind.ts";

/** The words the palette shows for an input of a strategy. */
const INPUT_NAMES: Record<string, string> = {
  name: "Name",
  password: "Password",
  code: "Recovery code",
  token: "Token",
};

/** The words the palette shows for a strategy's button. */
const BUTTON_NAMES: Record<string, string> = {
  password: "Sign in with a password",
  recovery: "Sign in with a recovery code",
  token: "Sign in with a token",
  account: "Use this computer's account",
  github: "Sign in with GitHub",
  microsoft: "Sign in with Microsoft",
  openid: "Sign in with your account",
};

/** A tile of the sign-in palette: the part it drops, with what it signs in with. */
export type SigninTile = {
  kind: WidgetKind;
  label: string;
  width: number;
  height: number;
  input?: string;
  strategy?: string;
  shows?: string;
};

/** The views the content of the sign-in window shows, in order. */
export function signinViews(pages: Page[]): string[] {
  const window = pages.find((p) => p.kind === "signin");
  if (!window) return [];
  const views: string[] = [];
  const walk = (widgets: Widget[]) => {
    for (const w of widgets) {
      if (w.props.kind === "content") {
        for (const page of w.pages) if (page.view) views.push(page.view);
      }
      walk(w.children);
      for (const page of w.pages) walk(page.children);
    }
  };
  walk(window.widgets);
  return views;
}

/** What the designer guides on `page`: a sign-in view, the sign-in window, or nothing. */
export function guided(page: Page, pages: Page[]): "view" | "window" | null {
  if (page.kind === "signin") return "window";
  if (page.kind === "view" && signinViews(pages).includes(page.name.name)) return "view";
  return null;
}

/** The sign-in tiles of the palette of sign-in view `view`, from the setup. */
export function signinTiles(signin: Signin | null, view: string, views: string[]): SigninTile[] {
  if (!signin) return [];
  const tiles: SigninTile[] = [];
  if (signin.languagePicker) tiles.push({ kind: "signinLanguage", label: "Language", width: 12, height: 2 });
  const inputs: string[] = [];
  for (const s of signin.strategies) for (const input of s.inputs ?? []) if (!inputs.includes(input)) inputs.push(input);
  for (const input of inputs) tiles.push({ kind: "signinField", label: INPUT_NAMES[input] ?? input, width: 12, height: 2, input });
  if (signin.rememberAsks) tiles.push({ kind: "signinRemember", label: "Keep me signed in", width: 12, height: 1 });
  for (const s of signin.strategies) tiles.push({ kind: "signinButton", label: BUTTON_NAMES[s.name] ?? `Sign in: ${s.name}`, width: 12, height: 1, strategy: s.name });
  for (const other of views.filter((v) => v !== view)) tiles.push({ kind: "signinOther", label: `Other way: ${other}`, width: 12, height: 1, shows: other });
  tiles.push({ kind: "signinRefusal", label: "Refusal", width: 12, height: 1 });
  return tiles;
}

/** The choices of a sign-in part's own setting: the inputs, strategies or other sign-in views it may take. */
export function signinChoices(kind: WidgetKind, signin: Signin | null, view: string, views: string[]): string[] {
  if (!signin) return [];
  switch (kind) {
    case "signinField": {
      const inputs: string[] = [];
      for (const s of signin.strategies) for (const input of s.inputs ?? []) if (!inputs.includes(input)) inputs.push(input);
      return inputs;
    }
    case "signinButton":
      return signin.strategies.map((s) => s.name);
    case "signinOther":
      return views.filter((v) => v !== view);
    default:
      return [];
  }
}
