// The guided sign-in in the designer: which pages are sign-in views, and the palette made from the setup.

import { test } from "node:test";
import assert from "node:assert/strict";
import { guided, signinChoices, signinTiles, signinViews } from "./signinView.ts";
import type { Page } from "@clarity/ui/generated/Page.ts";
import type { Signin } from "@clarity/ui/generated/Signin.ts";

function page(name: string, kind: Page["kind"], widgets: unknown[] = []): Page {
  return { name: { name }, kind, widgets } as unknown as Page;
}

const content = { props: { kind: "content" }, children: [], pages: [{ view: "PasswordView", children: [] }, { view: "CodeView", children: [] }] };
const PAGES = [page("Main", "window"), page("SignIn", "signin", [content]), page("PasswordView", "view"), page("CodeView", "view"), page("Other", "view")];

const SIGNIN = {
  entity: "Member",
  targets: ["desktop"],
  strategies: [
    { name: "password", fields: ["email", "passwordHash"], inputs: ["name", "password"] },
    { name: "recovery", fields: ["email", "codes"], inputs: ["name", "code"] },
    { name: "account", fields: ["login"], inputs: [] },
  ],
  sessionMinutes: 480,
  tries: 5,
  triesMinutes: 15,
  rememberAsks: true,
  shows: [],
  languagePicker: true,
} as unknown as Signin;

test("the sign-in views are the views the sign-in window's content shows", () => {
  assert.deepEqual(signinViews(PAGES), ["PasswordView", "CodeView"]);
  assert.equal(guided(PAGES[1], PAGES), "window");
  assert.equal(guided(PAGES[2], PAGES), "view");
  assert.equal(guided(PAGES[4], PAGES), null);
  assert.equal(guided(PAGES[0], PAGES), null);
});

test("a sign-in view's palette is made from the setup", () => {
  const tiles = signinTiles(SIGNIN, "PasswordView", ["PasswordView", "CodeView"]);
  assert.deepEqual(
    tiles.map((t) => [t.kind, t.label]),
    [
      ["signinLanguage", "Language"],
      ["signinField", "Name"],
      ["signinField", "Password"],
      ["signinField", "Recovery code"],
      ["signinRemember", "Keep me signed in"],
      ["signinButton", "Sign in with a password"],
      ["signinButton", "Sign in with a recovery code"],
      ["signinButton", "Use this computer's account"],
      ["signinOther", "Other way: CodeView"],
      ["signinRefusal", "Refusal"],
    ],
  );
  assert.equal(tiles[2].input, "password");
  assert.equal(tiles[7].strategy, "account");
  assert.equal(tiles[8].shows, "CodeView");
  // Without the options turned on, no language and no keep me signed in.
  const plain = signinTiles({ ...SIGNIN, rememberAsks: false, languagePicker: false }, "PasswordView", ["PasswordView"]);
  assert.ok(!plain.some((t) => t.kind === "signinLanguage" || t.kind === "signinRemember" || t.kind === "signinOther"));
  assert.deepEqual(signinTiles(null, "PasswordView", []), []);
});

test("a sign-in part's setting offers what the setup and the views have", () => {
  assert.deepEqual(signinChoices("signinField", SIGNIN, "PasswordView", ["PasswordView", "CodeView"]), ["name", "password", "code"]);
  assert.deepEqual(signinChoices("signinButton", SIGNIN, "PasswordView", []), ["password", "recovery", "account"]);
  assert.deepEqual(signinChoices("signinOther", SIGNIN, "CodeView", ["PasswordView", "CodeView"]), ["PasswordView"]);
});
