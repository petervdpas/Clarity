// The code behind a window, per element: the handlers of the part picked in
// the designer, with the handlers they call, edited on their own and merged
// back into the window's code.

import { test } from "node:test";
import assert from "node:assert/strict";
import { commandAction, commandHandler, fixOf, freePartName, focusOf, handlerName, handlersOf, mergeFocus, wireCommand, wireEvent } from "./codeParts.ts";
import type { CommandProps } from "./generated/CommandProps";
import type { WidgetProps } from "./generated/WidgetProps";

const CODE = `// A click on a square plays it.
handler board_clicked
    status = Board.play(1)

handler restart_clicked
    status = Board.restart()

// The grid, and on each square its mark.
handler draw_board
    board.clear("panel")
    draw_mark(1, 0, 0)

// One mark.
handler draw_mark(square: integer, left: integer, top: integer)
    let mark = Board.mark(square)
`;

test("the code splits into handlers, each with the comment above it", () => {
  const handlers = handlersOf(CODE);
  assert.deepEqual(
    handlers.map((h) => h.name),
    ["board_clicked", "restart_clicked", "draw_board", "draw_mark"],
  );
  assert.equal(handlers[0].text, "// A click on a square plays it.\nhandler board_clicked\n    status = Board.play(1)\n");
  assert.equal(handlers[3].text, "// One mark.\nhandler draw_mark(square: integer, left: integer, top: integer)\n    let mark = Board.mark(square)\n");
});

test("a part's code is its handlers and the handlers they call", () => {
  const focus = focusOf(CODE, ["draw_board", "board_clicked"]);
  assert.deepEqual(focus.names, ["board_clicked", "draw_board", "draw_mark"]);
  assert.equal(
    focus.text,
    "// A click on a square plays it.\nhandler board_clicked\n    status = Board.play(1)\n\n// The grid, and on each square its mark.\nhandler draw_board\n    board.clear(\"panel\")\n    draw_mark(1, 0, 0)\n\n// One mark.\nhandler draw_mark(square: integer, left: integer, top: integer)\n    let mark = Board.mark(square)\n",
  );
  // The line in the window's code each focused line is.
  assert.equal(focus.firstLine, 1);
  assert.deepEqual(focusOf(CODE, ["restart_clicked"]).text, "handler restart_clicked\n    status = Board.restart()\n");
  // A handler that is not written yet has no code.
  assert.deepEqual(focusOf(CODE, ["nothing"]).names, []);
});

test("what is typed for a part goes back in place of its handlers; the rest of the code stays", () => {
  const names = ["restart_clicked"];
  const merged = mergeFocus(CODE, names, "handler restart_clicked\n    status = Board.restart()\n    message \"New game\"\n");
  assert.ok(merged.text.includes('handler restart_clicked\n    status = Board.restart()\n    message "New game"\n\n// The grid'), merged.text);
  assert.ok(merged.text.startsWith("// A click on a square plays it.\nhandler board_clicked"));
  assert.ok(merged.text.endsWith("    let mark = Board.mark(square)\n"));
  // Where the typed text starts in the window's code, to place its problems.
  assert.equal(merged.firstLine, 5);
  // A new handler typed among them is kept with them.
  const added = mergeFocus(CODE, names, "handler restart_clicked\n    status = Board.restart()\n\nhandler helper\n    status = \"x\"\n");
  assert.deepEqual(handlersOf(added.text).map((h) => h.name), ["board_clicked", "restart_clicked", "helper", "draw_board", "draw_mark"]);
  // A part with no handler written yet adds its text at the end.
  const fresh = mergeFocus(CODE, ["again_clicked"], "handler again_clicked\n    status = \"again\"\n");
  assert.ok(fresh.text.endsWith('    let mark = Board.mark(square)\n\nhandler again_clicked\n    status = "again"\n'), fresh.text);
});

test("wiring an event writes the handler named after the part and the event, and shows it", () => {
  const props = { name: "board", handlers: [{ event: "click", handler: "board_clicked" }], onClick: null } as unknown as WidgetProps;
  const wired = wireEvent({ item: 40, props, events: ["draw", "click", "pointer"], owner: "Game", event: "draw", handler: "board_draw" });
  assert.deepEqual(wired, {
    op: "batch",
    edits: [
      { op: "updateWidget", item: 40, container: null, props: { ...props, handlers: [{ event: "draw", handler: "board_draw" }, { event: "click", handler: "board_clicked" }] } },
      { op: "addHandler", owner: "Game", name: "board_draw" },
    ],
  });
  // A click runs a handler or opens a window, not both.
  const opening = { ...props, handlers: [], onClick: "Settings" } as unknown as WidgetProps;
  const clicked = wireEvent({ item: 40, props: opening, events: ["click"], owner: "Game", event: "click", handler: "board_click" });
  assert.equal(clicked.op === "batch" && clicked.edits[0].op === "updateWidget" && clicked.edits[0].props.onClick, null);
  // Unwiring only changes the part; the handler stays in the code.
  const unwired = wireEvent({ item: 40, props, events: ["click"], owner: "Game", event: "click", handler: null });
  assert.equal(unwired.op, "updateWidget");
});

test("the handler a wired event runs, without the values a click gives it", () => {
  assert.equal(handlerName("played(1)"), "played");
  assert.equal(handlerName("board_clicked"), "board_clicked");
});

const item = (changes: Partial<CommandProps>): CommandProps =>
  ({ kind: "item", caption: "New game", name: null, icon: null, shortcut: null, large: false, onClick: null, newWindow: false, handler: null, ...changes }) as CommandProps;

test("a menu item's handler is named after the item, or its caption when it has no name", () => {
  assert.equal(commandHandler(item({})), "new_game_click");
  assert.equal(commandHandler(item({ name: "restart" })), "restart_click");
  assert.equal(commandHandler(item({ caption: "Save & close..." })), "save_close_click");
});

test("a menu item that opens a window or quits has no code, and says what it does", () => {
  assert.equal(commandAction(item({ onClick: "Game", newWindow: true })), "opens Game in a new window");
  assert.equal(commandAction(item({ onClick: "Settings" })), "opens Settings");
  assert.equal(commandAction(item({ onClick: "quit" })), "quits the application");
  assert.equal(commandAction(item({ onClick: "close" })), "closes the window");
  assert.equal(commandAction(item({ onClick: "theme" })), "lets the person pick a theme");
  assert.equal(commandAction(item({ onClick: "language" })), "lets the person pick a language");
  assert.equal(commandAction(item({ handler: "x" })), null);
});

test("wiring a menu item writes its handler and stops it opening a window", () => {
  const wired = wireCommand({ item: 7, props: item({ onClick: "Game" }), owner: "Noughts", handler: "new_game_click" });
  assert.deepEqual(wired, {
    op: "batch",
    edits: [
      { op: "updateCommand", item: 7, props: item({ handler: "new_game_click", onClick: null }) },
      { op: "addHandler", owner: "Noughts", name: "new_game_click" },
    ],
  });
  assert.equal(wireCommand({ item: 7, props: item({ handler: "x" }), owner: "Noughts", handler: null }).op, "updateCommand");
});

test("a part without a name gets the first free one of its kind, as a new part does", () => {
  assert.equal(freePartName(["button1", "canvas1"], "canvas"), "canvas2");
  assert.equal(freePartName([], "label"), "label1");
});

test("a naming problem offers the fix that solves it for the whole file", () => {
  assert.deepEqual(fixOf("text at 1,9 has no name; every part has one (Name them, in Problems, names them all)"), { label: "Name them", edit: { op: "nameParts" } });
  assert.deepEqual(fixOf("handler 'new_game' is not camelCase: letters and digits, starting with a small letter, such as 'newGame'"), {
    label: "Make them camelCase",
    edit: { op: "camelNames" },
  });
  assert.equal(fixOf('item "New game" has no name; every menu item and button has one (Name them, in Problems, names them all)')?.label, "Name them");
  assert.equal(fixOf("unknown name 'x'"), null);
});
