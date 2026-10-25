// How windows open and close in a running application on the web renderer,
// against the cases every stack shares (conformance/windows.json).

import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";
import { Navigator, type NavReply } from "./navigator.ts";

type Case = {
  name: string;
  dialogs: string[];
  start: string;
  steps: string[][];
  expect: { desktops: string[][]; events: string[]; ended: boolean };
};
const cases: { cases: Case[] } = JSON.parse(readFileSync(new URL("../../../conformance/windows.json", import.meta.url), "utf8"));

/** A desktop window of a case: its stack (a window is its name) and its navigator. */
type Desktop = { stack: string[]; nav: Navigator<string, string> };

/** Runs a case's application: the backend logs the events of every desktop window. */
function application() {
  const events: string[] = [];
  const desktops: Desktop[] = [];
  let ended = false;
  const reply = (state: string): NavReply<string, string> => ({ state, messages: [], open: [], close: false });

  function desktop(opener: Navigator<string, string> | null): Desktop {
    const d = { stack: [] as string[] } as Desktop;
    d.nav = new Navigator<string, string>(
      { get: () => d.stack, set: (next) => (d.stack = next) },
      {
        open: async (window) => {
          events.push(`${window}:open`);
          return reply(window);
        },
        close: async (state) => {
          events.push(`${state}:close`);
          return reply(state);
        },
        returnTo: async (state) => {
          events.push(`${state}:return`);
          return reply(state);
        },
        openApart: async (window, withData) => {
          await desktop(d.nav).nav.open(window, withData);
        },
        quit: async () => {
          // The first desktop window ends the application; another closes and tells its opener.
          desktops.splice(desktops.indexOf(d), 1);
          if (opener === null) ended = true;
          else await opener.returned();
        },
      },
      () => {},
      (err) => assert.fail(String(err)),
    );
    desktops.push(d);
    return d;
  }

  return { events, desktops, ended: () => ended, first: desktop(null) };
}

test("every case opens and closes its windows the same way", async () => {
  assert.ok(cases.cases.length > 0);
  for (const c of cases.cases) {
    const app = application();
    await app.first.nav.open(c.start);
    for (const actions of c.steps) {
      // A handler of the window on top of the newest desktop window asked for these.
      const d = app.desktops.at(-1)!;
      const opens = actions
        .filter((a) => a.startsWith("open "))
        .map((a) => {
          const apart = a.endsWith(" in new window");
          return { window: a.slice(5, apart ? -" in new window".length : undefined), with: {}, newWindow: apart };
        });
      await d.nav.apply(d.stack.length - 1, { state: d.stack.at(-1)!, messages: [], open: opens, close: actions.includes("close") });
    }
    assert.deepEqual(
      app.desktops.map((d) => d.stack),
      c.expect.desktops,
      `${c.name}: desktop windows`,
    );
    assert.deepEqual(app.events, c.expect.events, `${c.name}: events`);
    assert.equal(app.ended(), c.expect.ended, `${c.name}: ended`);
  }
});

test("a reply's questions are asked after its messages and the windows it opens, by the window that asked", async () => {
  type Reply = NavReply<string, string, string>;
  const said: string[] = [];
  let stack: string[] = [];
  const nav = new Navigator<string, string, string>(
    { get: () => stack, set: (next) => (stack = next) },
    {
      open: async (window): Promise<Reply> => ({ state: window, messages: [`${window} opened`], open: [], close: false, asks: window === "Main" ? ["Name it?"] : [] }),
      close: async (state): Promise<Reply> => ({ state, messages: [], open: [], close: false }),
      returnTo: async (state): Promise<Reply> => ({ state, messages: [], open: [], close: false }),
      openApart: async () => {},
      quit: async () => {},
    },
    (message) => said.push(message),
    (err) => assert.fail(String(err)),
    (question, window) => said.push(`ask ${question} of window ${window}`),
  );
  await nav.open("Main");
  assert.deepEqual(said, ["Main opened", "ask Name it? of window 0"]);
  await nav.apply(0, { state: "Main", messages: ["saved"], open: [{ window: "Card", with: {} }], close: false, asks: ["Sure?"] });
  assert.deepEqual(said.slice(2), ["saved", "Card opened", "ask Sure? of window 0"]);
});

test("a window that closes with a value gives it, after it has closed, with its state", async () => {
  type Reply = NavReply<string, string, string>;
  const said: string[] = [];
  let stack: string[] = [];
  const nav = new Navigator<string, string, string>(
    { get: () => stack, set: (next) => (stack = next) },
    {
      open: async (window): Promise<Reply> => ({ state: window, messages: [], open: [], close: false }),
      close: async (state): Promise<Reply> => {
        said.push(`${state} closes`);
        return { state, messages: [], open: [], close: false };
      },
      returnTo: async (state): Promise<Reply> => {
        said.push(`${state} returns`);
        return { state, messages: [], open: [], close: false };
      },
      openApart: async () => {},
      quit: async () => {},
    },
    () => {},
    (err) => assert.fail(String(err)),
    () => {},
    (state, result) => said.push(`${state} gave ${JSON.stringify(result)}`),
  );
  await nav.open("Albums");
  await nav.open("PickFriend");
  await nav.apply(1, { state: "PickFriend", messages: [], open: [], close: true, result: { id: 3 } });
  assert.deepEqual(said, ["PickFriend closes", "Albums returns", 'PickFriend gave {"id":3}']);
  assert.deepEqual(stack, ["Albums"]);
});

test("the title bar's close closes every window from the top, each told, then the desktop window", async () => {
  const app = application();
  await app.first.nav.open("Offline");
  await app.first.nav.open("Connection");
  app.events.length = 0;
  await app.first.nav.closeAll();
  // No window below is told it is on top again: they all close.
  assert.deepEqual(app.events, ["Connection:close", "Offline:close"]);
  assert.deepEqual(app.first.stack, []);
  assert.equal(app.ended(), true);
});
