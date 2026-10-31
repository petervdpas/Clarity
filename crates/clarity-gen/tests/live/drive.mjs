// The steps a person takes in the every-part project (tests/everything_live.rs),
// the same on every stack. Each step does what a person does (keys, typing,
// pointer presses and drags) and then reads the screen and the database.
// A step that fails is reported with a screenshot, and the next one runs,
// so one run shows everything that is wrong.
// Arguments: the address it is served at, the folder it runs in, the stack,
// the database file.

import { execFileSync } from "node:child_process";
import { openBrowser } from "./browser.mjs";

import { resolve } from "node:path";

// Clean absolute paths: the browser reads no file through "..".
const [url, given, stack, database] = process.argv.slice(2);
const folder = resolve(given);

// As any client of the database does, it waits a moment for the app's write to end.
const sql = (query) => execFileSync("sqlite3", ["-cmd", ".timeout 5000", "-separator", "|", database, query]).toString().trim();
const rows = (query) => (sql(query) ? sql(query).split("\n") : []);

const failed = [];
let current = "";
function check(ok, what) {
  if (!ok) throw new Error(what);
}
/** Waits up to 3 s for `read()` to give `expected`: what follows a round trip to the runtime. */
async function sameSoon(read, expected, what) {
  const end = Date.now() + 3000;
  let actual = await read();
  while (JSON.stringify(actual) !== JSON.stringify(expected) && Date.now() < end) {
    await new Promise((r) => setTimeout(r, 100));
    actual = await read();
  }
  same(actual, expected, what);
}

function same(actual, expected, what) {
  const a = JSON.stringify(actual);
  const e = JSON.stringify(expected);
  if (a !== e) throw new Error(`${what}: expected ${e}, got ${a}`);
}

const b = await openBrowser(url, { folder });

/** An error the app said on screen, closed so the next step starts clean; none when it said none. */
async function errorSaid() {
  const text = await b.eval(`[...document.querySelectorAll("p.said.error, .message.error")].map(e => e.textContent.trim()).join(" / ")`);
  if (text) {
    await b.eval(`document.querySelector(".said")?.closest("[role=dialog], .panel, div")?.querySelector("button")?.click()`);
    await b.settle(200);
  }
  return text;
}

async function step(name, run) {
  current = name;
  const logs = b.logs.length;
  try {
    await run();
    const said = b.logs.slice(logs).filter((l) => !l.includes("favicon"));
    if (said.length) throw new Error(`the page said:\n    ${said.join("\n    ")}`);
    const error = await errorSaid();
    if (error) throw new Error(`the app said: ${error}`);
    console.log(`ok   ${name}`);
  } catch (e) {
    const shot = `${folder}/failed-${failed.length + 1}.png`;
    await b.screenshot(shot).catch(() => {});
    await errorSaid().catch(() => {});
    // Whatever the step left open (a question, a menu) is closed, as a person would.
    for (let i = 0; i < 3 && (await b.eval(`!!document.querySelector("[role=dialog]")`)); i++) await b.key("Escape");
    failed.push(name);
    console.log(`FAIL ${name}\n     ${String(e.message ?? e).split("\n").join("\n     ")}\n     (screenshot ${shot})`);
  }
}

/** The texts of a part's rows, as shown. */
const shownRows = (part) => b.eval(`[...document.querySelectorAll('[data-part="${part}"] [data-row]')].map(r => r.textContent.trim().replace(/\\s+/g, " "))`);
const value = (part, inner = "input, textarea, select") => b.eval(`document.querySelector('[data-part="${part}"] :is(${inner})')?.value ?? null`);
const part = (name) => `[data-part="${name}"]`;
async function focusRow(name, index) {
  await b.click(`${part(name)} [data-row="${index}"]`);
}
async function undo() {
  await b.key("z", ["Control"]);
  await b.settle(400);
}
async function redo() {
  await b.key("y", ["Control"]);
  await b.settle(400);
}
/** Presses the button whose text is `text`, inside `scope`. */
async function press(text, scope = "body") {
  const at = await b.eval(`(() => { const el = [...document.querySelectorAll(${JSON.stringify(scope + " button")})].find(e => e.offsetParent !== null && e.textContent.replace(/^[^\\p{L}\\p{N}]+/u, "").trim() === ${JSON.stringify(text)}); if (!el) return null; el.setAttribute("data-live", "pressed"); return true; })()`);
  if (!at) throw new Error(`no button ${text} in ${scope}`);
  await b.click('[data-live="pressed"]');
  await b.eval(`document.querySelector('[data-live="pressed"]')?.removeAttribute("data-live")`);
}
const checked = (name) => b.eval(`document.querySelector('${part(name)} input')?.checked ?? null`);
async function edit(name, text, inner = "input") {
  await b.click(`${part(name)} ${inner}`);
  await b.key("a", ["Control"]);
  await b.type(text);
  await b.key("Tab");
}
async function undoIn(name, inner = "input") {
  await b.eval(`document.querySelector('${part(name)} ${inner}')?.focus()`);
  await undo();
}

async function openWindow(button) {
  await b.click(part(button));
  await b.settle(600);
}

try {
  await b.until(`document.querySelector("[data-part]")`, "the first window", 30000);

  // Records to work with, as a person would have them.
  sql(`INSERT INTO album (title, position, copies) VALUES ('Alpha', 1, 1), ('Beta', 2, 2), ('Gamma', 3, 3);
       INSERT INTO folder (name, position, size, parent) VALUES ('Music', 1, 10, NULL), ('Docs', 2, 20, NULL);
       INSERT INTO folder (name, position, size, parent) VALUES ('Rock', 1, 5, (SELECT id FROM folder WHERE name = 'Music'));
       INSERT INTO friend (name, city, position) VALUES ('Ann', 'Oslo', 1), ('Bob', 'Rome', 2), ('Cid', 'Lima', 3);`);

  // ---- Inputs in a form: each edit is one step back, the last first.
  await step("a form's fields are taken back one by one, the last first", async () => {
    await b.click(`${part("albumTitleBox")} input`);
    await b.type("Zed");
    await b.key("Tab");
    await b.type("4");
    await b.key("Tab");
    same(await value("albumTitleBox"), "Zed", "title typed");
    same(await value("albumCopiesBox"), "4", "copies typed");
    await b.click(`${part("albumTitleBox")} input`);
    await b.key("Tab");
    await undo();
    same(await value("albumCopiesBox"), "0", "copies taken back first");
    same(await value("albumTitleBox"), "Zed", "title still there");
    await undo();
    same(await value("albumTitleBox"), "", "title taken back next");
    await redo();
    same(await value("albumTitleBox"), "Zed", "title done again");
  });

  await step("a toggle's press and what its handler saved are taken back together", async () => {
    const before = Number(sql("SELECT count(*) FROM album"));
    await b.click(part("favouriteToggle"));
    await b.settle(500);
    same(Number(sql("SELECT count(*) FROM album")), before + 1, "favouriteChanged saved the album");
    await undo();
    same(Number(sql("SELECT count(*) FROM album")), before, "the saved album is taken back");
  });

  await step("a slider and a spin box are edits", async () => {
    await b.click(`${part("copiesSpin")} input`);
    await b.key("ArrowUp");
    await b.key("Tab");
    const up = await value("copiesSpin");
    await b.click(`${part("copiesSpin")} input`);
    await undo();
    check((await value("copiesSpin")) !== up || up === "0", `spin box taken back (was ${up}, now ${await value("copiesSpin")})`);
  });

  // ---- Every input of the albums window: each change is an edit Ctrl+Z takes back.
  await step("a switch is an edit", async () => {
    const was = await checked("toolbarSwitch");
    await b.click(`${part("toolbarSwitch")} input`);
    same(await checked("toolbarSwitch"), !was, "switched");
    await undoIn("toolbarSwitch");
    same(await checked("toolbarSwitch"), was, "taken back");
  });

  await step("a dropdown of records is an edit", async () => {
    await b.eval(`document.querySelector('${part("favouriteFriendBox")} select').focus()`);
    await b.key("ArrowDown");
    await b.settle(300);
    const picked = await b.eval(`(() => { const s = document.querySelector('${part("favouriteFriendBox")} select'); return s.options[s.selectedIndex]?.text ?? ""; })()`);
    check(picked !== "", "a friend picked");
    await undoIn("favouriteFriendBox", "select");
    same(await b.eval(`(() => { const s = document.querySelector('${part("favouriteFriendBox")} select'); return s.options[s.selectedIndex]?.text ?? ""; })()`), "", "taken back");
  });

  await step("a color typed is an edit", async () => {
    await edit("tintBox", "#ff0000");
    same(await value("tintBox", "input"), "#ff0000", "typed");
    await undoIn("tintBox");
    same(await value("tintBox", "input"), "#3b82f6", "taken back");
  });

  await step("a slider moved with the keys is an edit", async () => {
    await b.eval(`document.querySelector('${part("ratingSlider")} input').focus()`);
    await b.key("ArrowRight");
    await b.settle(300);
    same(await value("ratingSlider"), "0.5", "moved");
    await undoIn("ratingSlider");
    same(await value("ratingSlider"), "0", "taken back");
  });

  await step("a tag added is an edit", async () => {
    await b.click(`${part("tagsBox")} input`);
    await b.type("live");
    await b.key("Enter");
    await b.settle(300);
    const chips = () => b.eval(`document.querySelector('${part("tagsBox")}').textContent`);
    check((await chips()).includes("live"), "the tag shows");
    await undoIn("tagsBox");
    check(!(await chips()).includes("live"), "taken back");
  });

  await step("a typed dropdown is an edit", async () => {
    await edit("cityBox", "Oslo");
    same(await value("cityBox"), "Oslo", "typed");
    await undoIn("cityBox");
    same(await value("cityBox"), "", "taken back");
  });

  await step("a file chosen is an edit", async () => {
    const { writeFileSync } = await import("node:fs");
    writeFileSync(`${folder}/notes.pdf`, "%PDF-1.4\n%%EOF\n");
    await b.files(`${part("bookletBox")} input[type=file]`, [`${folder}/notes.pdf`]);
    await b.until(`document.querySelector('${part("bookletBox")}').textContent.includes("notes.pdf")`, "the file shows");
    await b.eval(`document.querySelector('${part("bookletBox")} button')?.focus()`);
    await undo();
    check(!(await b.eval(`document.querySelector('${part("bookletBox")}').textContent`)).includes("notes.pdf"), "taken back");
  });

  await step("an accordion opens another section", async () => {
    await press("Site", part("notesAccordion"));
    check(await b.eval(`!!document.querySelector('${part("siteText")}')`), "the site section shows");
  });

  await step("a question for a number sets what it answers", async () => {
    await b.click(part("copiesButton"));
    await b.until(`document.querySelector('[role=dialog] form input')`, "the question");
    await b.key("a", ["Control"]);
    await b.type("7");
    await b.key("Enter");
    await b.settle(500);
    same(await b.eval(`document.querySelector('${part("copiesBadge")}').textContent.trim()`), "7", "the copies");
  });

  await step("a wizard refuses an empty title, then saves the album", async () => {
    const before = Number(sql("SELECT count(*) FROM album"));
    await b.click(part("newButton"));
    await b.until(`document.querySelector('${part("wizard")}')`, "the wizard");
    await press("Next", part("wizard"));
    await b.settle(300);
    same(await errorSaid(), "A title first", "the refusal said");
    check(await b.eval(`!!document.querySelector('${part("wizard")} .step-title.current')`), "still on the first step");
    await b.click(`${part("wizard")} ${part("titleBox")} input`);
    await b.type("Delta");
    await b.key("Tab");
    await press("Next", part("wizard"));
    await press("Next", part("wizard"));
    await press("Finish", part("wizard"));
    await b.settle(600);
    same(Number(sql("SELECT count(*) FROM album")), before + 1, "saved");
    same(sql("SELECT count(*) FROM album WHERE title = 'Delta'"), "1", "the title");
  });

  await step("a yes-or-no question deletes on yes", async () => {
    await b.click(part("deleteButton"));
    await b.until(`document.querySelector('[role=dialog]')`, "the question");
    await press("Delete", "[role=dialog]");
    await b.settle(500);
    same(sql("SELECT count(*) FROM album WHERE title = 'Delta'"), "0", "deleted");
  });
  // The steps after work on the albums they started with, whatever happened above.
  sql("DELETE FROM album WHERE title NOT IN ('Alpha', 'Beta', 'Gamma')");

  await step("a split button opens its menu", async () => {
    await b.click(`${part("saveButton")} button:last-of-type`);
    await b.settle(200);
    check(await b.eval(`[...document.querySelectorAll("button")].some(e => e.offsetParent && e.textContent.includes("Save a copy"))`), "the menu shows");
    await b.key("Escape");
  });

  await step("a menu item's key opens its window", async () => {
    await b.key("f", ["Control"]);
    await b.settle(600);
    check(await b.eval(`!!document.querySelector('${part("folderTree")}')`), "Ctrl+F opened the folders");
    await openWindow("albumsButton");
  });

  // ---- The gallery: a list that reorders, a panel, a no-undo input, a picture.
  await step("the gallery opens with the albums in their order", async () => {
    await openWindow("galleryButton");
    await b.until(`document.querySelectorAll('${part("albumList")} [data-row]').length >= 3`, "the albums listed");
    same(await shownRows("albumList"), ["Alpha | 1", "Beta | 2", "Gamma | 3"], "albums");
  });

  await step("Alt+Down moves a list row, saved; Ctrl+Z takes it back, Ctrl+Y does it again", async () => {
    await focusRow("albumList", 0);
    await b.key("ArrowDown", ["Alt"]);
    await b.settle(500);
    await sameSoon(() => shownRows("albumList"), ["Beta | 2", "Alpha | 1", "Gamma | 3"], "moved down");
    same(rows("SELECT title FROM album WHERE title IN ('Alpha','Beta','Gamma') ORDER BY position"), ["Beta", "Alpha", "Gamma"], "saved order");
    await undo();
    same(await shownRows("albumList"), ["Alpha | 1", "Beta | 2", "Gamma | 3"], "taken back");
    same(rows("SELECT title FROM album WHERE title IN ('Alpha','Beta','Gamma') ORDER BY position"), ["Alpha", "Beta", "Gamma"], "saved order taken back");
    await redo();
    same(rows("SELECT title FROM album WHERE title IN ('Alpha','Beta','Gamma') ORDER BY position"), ["Beta", "Alpha", "Gamma"], "done again");
    await undo();
  });

  await step("a dragged list row lands where it is dropped, and stays after reopening", async () => {
    const from = await b.at(`${part("albumList")} [data-row="2"]`);
    const to = await b.at(`${part("albumList")} [data-row="0"]`, 0.5, 0.2);
    await b.drag(from, to);
    await sameSoon(() => shownRows("albumList"), ["Gamma | 3", "Alpha | 1", "Beta | 2"], "dragged to the top");
    same(rows("SELECT title FROM album WHERE title IN ('Alpha','Beta','Gamma') ORDER BY position"), ["Gamma", "Alpha", "Beta"], "saved");
    await openWindow("foldersButton");
    await openWindow("galleryButton");
    await b.until(`document.querySelectorAll('${part("albumList")} [data-row]').length >= 3`, "the albums listed again");
    same((await shownRows("albumList")).slice(0, 3), ["Gamma | 3", "Alpha | 1", "Beta | 2"], "order after reopening");
  });

  await step("a panel's fields are taken back one by one", async () => {
    await b.doubleClick(`${part("albumList")} [data-row="1"]`);
    await b.click(`${part("priceBox")} input`);
    await b.key("a", ["Control"]);
    await b.type("12");
    await b.key("Tab");
    await b.click(`${part("notesMemo")} textarea`);
    await b.type("note");
    await b.click(`${part("releasedOn")}`);
    same(await value("notesMemo"), "note", "memo typed");
    await undo();
    same(await value("notesMemo"), "", "memo taken back first");
    check((await value("priceBox")).startsWith("12"), `price still there (now ${await value("priceBox")})`);
    await undo();
    check(!(await value("priceBox")).startsWith("12"), `price taken back (now ${await value("priceBox")})`);
  });

  await step("a timer keeps ticking in a window under another, without an error", async () => {
    // The first gallery is still open under the folders and this one; its timer ticks there.
    await b.settle(2500);
  });

  await step("a radio choice is an edit", async () => {
    await b.click(`${part("formatRadio")} input[value="Cd"], ${part("formatRadio")} label:nth-of-type(2) input`);
    const cd = () => b.eval(`[...document.querySelectorAll('${part("formatRadio")} input')].findIndex(i => i.checked)`);
    same(await cd(), 1, "CD chosen");
    await undoIn("formatRadio");
    same(await cd(), 0, "taken back");
  });

  await step("a date typed is an edit; Ctrl+Z takes back the typing, and the day once taken", async () => {
    // Clicked where its month is, as a person starts typing a date.
    await b.click(`${part("releasedOn")} input`, 0.1, 0.5);
    await b.typeKeys("02142024");
    await undo();
    same(await value("releasedOn"), "", "typing taken back in the input");
    await b.click(`${part("releasedOn")} input`, 0.1, 0.5);
    await b.typeKeys("02142024");
    await b.click(`${part("priceBox")} input`);
    same(await value("releasedOn"), "2024-02-14", "taken when left");
    await undoIn("releasedOn");
    same(await value("releasedOn"), "", "the day taken back");
  });

  await step("a password and a folder typed are edits", async () => {
    await edit("tokenBox", "s3cret");
    await undoIn("tokenBox");
    same(await value("tokenBox"), "", "password taken back");
    await edit("scansBox", "papers");
    same(await value("scansBox"), "papers", "folder typed");
    await undoIn("scansBox");
    same(await value("scansBox"), "scans", "folder taken back");
  });

  await step("a picture loaded into an image is an edit", async () => {
    await press("Cover", part("galleryTabs"));
    await b.files(`${part("coverImage")} input[type=file]`, [`${process.env.CLARITY_FIXTURE}/resources/banner.png`]);
    await b.until(`document.querySelector('${part("coverImage")} img.picture')`, "the picture");
    await b.eval(`document.querySelector('${part("coverImage")} button')?.focus()`);
    await undo();
    await b.until(`!document.querySelector('${part("coverImage")} img.picture')`, "the picture taken back");
    await press("Albums", part("galleryTabs"));
  });

  await step("a drag on a canvas and its undo say nothing wrong", async () => {
    const from = await b.at(part("dial"), 0.3, 0.5);
    await b.drag(from, { x: from.x + 60, y: from.y + 20 });
    await undoIn("dial", "canvas");
  });

  await step("a sound plays and an engine call answers", async () => {
    await b.click(part("playButton"));
    await b.click(part("planButton"));
    await b.settle(800);
  });

  await step("a tab shows its markdown", async () => {
    await press("About", part("galleryTabs"));
    check((await b.eval(`document.querySelector('${part("aboutText")}')?.textContent ?? ""`)).trim().length > 0, "the text shows");
    await press("Albums", part("galleryTabs"));
  });

  await step("an input with no undo keeps what was typed", async () => {
    await b.click(`${part("contactBox")} input`);
    await b.type("ann@example.com");
    await b.key("Tab");
    await b.click(`${part("contactBox")} input`);
    await undo();
    same(await value("contactBox"), "ann@example.com", "kept");
  });

  // ---- Folders: a tree that reorders, and a tree table that reorders without undo.
  await step("a tree row moves down, and Ctrl+Z takes it back", async () => {
    await openWindow("foldersButton");
    await b.until(`document.querySelectorAll('${part("folderTree")} [data-row]').length >= 2`, "the folders");
    same(await shownRows("folderTree"), ["Music", "Docs"], "the top level");
    await focusRow("folderTree", Number(await b.eval(`document.querySelector('${part("folderTree")} [data-row]').dataset.row`)));
    await b.key("ArrowDown", ["Alt"]);
    await b.settle(500);
    same(rows("SELECT name FROM folder WHERE parent IS NULL ORDER BY position"), ["Docs", "Music"], "saved order");
    await undo();
    same(rows("SELECT name FROM folder WHERE parent IS NULL ORDER BY position"), ["Music", "Docs"], "taken back");
  });

  await step("Alt+Right puts a tree row under the row before it; Ctrl+Z takes it out again", async () => {
    const docs = await b.eval(`[...document.querySelectorAll('${part("folderTree")} [data-row]')].find(r => r.textContent.includes("Docs")).dataset.row`);
    await focusRow("folderTree", docs);
    await b.key("ArrowRight", ["Alt"]);
    await b.settle(500);
    same(sql("SELECT p.name FROM folder f JOIN folder p ON p.id = f.parent WHERE f.name = 'Docs'"), "Music", "Docs under Music");
    check((await shownRows("folderTree")).includes("Docs"), "Docs shows, its parent open");
    await undo();
    same(sql("SELECT count(*) FROM folder WHERE name = 'Docs' AND parent IS NULL"), "1", "Docs back at the top");
  });

  await step("a tree row dragged onto another goes under it", async () => {
    const rowsNow = await b.eval(`[...document.querySelectorAll('${part("folderTree")} [data-row]')].map(r => [r.dataset.row, r.textContent.trim()])`);
    const docs = rowsNow.find((r) => r[1].includes("Docs"))[0];
    const music = rowsNow.find((r) => r[1].includes("Music"))[0];
    await b.drag(await b.at(`${part("folderTree")} [data-row="${docs}"]`), await b.at(`${part("folderTree")} [data-row="${music}"]`, 0.5, 0.5));
    same(sql("SELECT p.name FROM folder f JOIN folder p ON p.id = f.parent WHERE f.name = 'Docs'"), "Music", "dropped into Music");
    await undo();
    same(sql("SELECT count(*) FROM folder WHERE name = 'Docs' AND parent IS NULL"), "1", "taken back");
  });

  await step("a tree table that reorders with no undo keeps its move", async () => {
    const first = await b.eval(`document.querySelector('${part("folderTable")} [data-row]').dataset.row`);
    await focusRow("folderTable", first);
    await b.key("ArrowDown", ["Alt"]);
    await b.settle(500);
    const moved = rows("SELECT name FROM folder WHERE parent IS NULL ORDER BY position");
    same(moved, ["Docs", "Music"], "moved");
    await undo();
    same(rows("SELECT name FROM folder WHERE parent IS NULL ORDER BY position"), ["Docs", "Music"], "kept: no undo");
  });

  // ---- What a table, a tree and a chart show, as the runtime decides it (DESIGN 1.7).
  await step("a table shows its first order a page at a time, and turns pages", async () => {
    await b.until(`document.querySelectorAll('${part("albumGrid")} [data-row]').length >= 2`, "the albums");
    await sameSoon(() => shownRows("albumGrid"), ["Alpha 1", "Beta 2"], "the first page, by title");
    // The page buttons are icons, named by their label.
    await b.click(`${part("albumGrid")} button[aria-label="Next page"]`);
    await b.settle(400);
    await sameSoon(() => shownRows("albumGrid"), ["Gamma 3"], "the second page");
  });

  await step("a header click sorts the other way, from the first page", async () => {
    await b.click(`${part("albumGrid")} th[data-field="title"]`);
    await b.settle(400);
    await sameSoon(() => shownRows("albumGrid"), ["Gamma 3", "Beta 2"], "sorted the other way");
  });

  await step("a tree row's toggle opens and closes the rows under it", async () => {
    const toggle = async () => {
      await b.eval(`[...document.querySelectorAll('${part("folderTree")} [data-row]')].find(r => r.textContent.includes("Music")).querySelector("button.toggle").setAttribute("data-live", "toggle")`);
      await b.click('[data-live="toggle"]');
      await b.eval(`document.querySelector('[data-live="toggle"]')?.removeAttribute("data-live")`);
      await b.settle(400);
      return (await shownRows("folderTree")).includes("Rock");
    };
    const before = (await shownRows("folderTree")).includes("Rock");
    same(await toggle(), !before, "the first press turns it");
    same(await toggle(), before, "the second press turns it back");
  });

  await step("a table in a dialog reorders, saved, and Ctrl+Z takes it back", async () => {
    await openWindow("albumsButton");
    await b.click(part("pickButton"));
    await b.until(`document.querySelectorAll('${part("grid")} [data-row]').length >= 3`, "the friends");
    await focusRow("grid", 2);
    await b.key("ArrowUp", ["Alt"]);
    await b.settle(500);
    same(rows("SELECT name FROM friend ORDER BY position"), ["Ann", "Cid", "Bob"], "saved order");
    await undo();
    same(rows("SELECT name FROM friend ORDER BY position"), ["Ann", "Bob", "Cid"], "taken back");
  });

  await step("a popover's checkbox runs its handler, which closes it", async () => {
    await b.click(part("filterButton"));
    await b.until(`document.querySelector('${part("namedBox")} input')`, "the popover");
    await b.click(`${part("namedBox")} input`);
    await b.settle(400);
    check(!(await b.eval(`!!document.querySelector('${part("namedBox")} input')?.offsetParent`)), "the popover closed");
  });

  await step("a table row's context menu picks it, and the dialog gives it back", async () => {
    const at = await b.at(`${part("grid")} [data-row="1"]`);
    await b.send("Input.dispatchMouseEvent", { type: "mousePressed", ...at, button: "right", buttons: 2, clickCount: 1 });
    await b.send("Input.dispatchMouseEvent", { type: "mouseReleased", ...at, button: "right", buttons: 0, clickCount: 1 });
    await b.settle(300);
    await press("Pick");
    await b.settle(600);
    check(!(await b.eval(`!!document.querySelector('${part("grid")}')`)), "the dialog closed");
  });
} catch (e) {
  failed.push(current || "start");
  console.log(`FAIL ${current}: ${e.message}`);
} finally {
  await b.close();
}
console.log(failed.length ? `\n${failed.length} failed on ${stack}: ${failed.join("; ")}` : `\nall steps passed on ${stack}`);
process.exit(failed.length ? 1 : 0);
