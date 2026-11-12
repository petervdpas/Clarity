// What Clarity's file browser gives back for each use:
// a project, a folder, a file, or a new file's place and name.

import { test } from "node:test";
import assert from "node:assert/strict";
import { chosen, withKind, type Pickable } from "./fileBrowser.ts";

const project = { file: "/work/shop.clarity", title: "Shop", stack: "Rust, Clarity.UI, Desktop", problem: null };
const shopFile: Pickable = { path: "/work/shop.clarity", folder: false, project, projects: [] };
const holder: Pickable = { path: "/work/Todo", folder: true, project: null, projects: [{ ...project, file: "/work/Todo/todo.clarity", title: "Todo" }] };
const twoInside: Pickable = { path: "/work/Two", folder: true, project: null, projects: [project, { ...project, title: "Other" }] };
const plain: Pickable = { path: "/work/notes", folder: true, project: null, projects: [] };
const yaml: Pickable = { path: "/work/forms.yaml", folder: false, project: null, projects: [] };

test("opening a project takes a project file, or a folder that holds exactly one", () => {
  assert.equal(chosen("project", shopFile, "/work", "", []), "/work/shop.clarity");
  assert.equal(chosen("project", holder, "/work", "", []), "/work/Todo/todo.clarity");
  assert.equal(chosen("project", twoInside, "/work", "", []), null, "two projects: go in and pick one");
  assert.equal(chosen("project", plain, "/work", "", []), null);
  assert.equal(chosen("project", yaml, "/work", "", []), null);
  assert.equal(chosen("project", null, "/work", "", []), null);
});

test("a folder is the one picked, else the one shown", () => {
  assert.equal(chosen("folder", plain, "/work", "", []), "/work/notes");
  assert.equal(chosen("folder", null, "/work", "", []), "/work");
});

test("a file is a file picked", () => {
  assert.equal(chosen("file", yaml, "/work", "", ["yaml"]), "/work/forms.yaml");
  assert.equal(chosen("file", plain, "/work", "", ["yaml"]), null);
  assert.equal(chosen("file", null, "/work", "", ["yaml"]), null);
});

test("saving puts the name in the folder shown, with its kind", () => {
  assert.equal(chosen("save", null, "/work", "shop", ["db", "sqlite"]), "/work/shop.db");
  assert.equal(chosen("save", null, "/work", "shop.sqlite", ["db", "sqlite"]), "/work/shop.sqlite");
  assert.equal(chosen("save", null, "C:\\work", "shop", ["db"]), "C:\\work\\shop.db", "Windows paths keep their separator");
  assert.equal(chosen("save", null, "/work", "  ", ["db"]), null);
  assert.equal(chosen("save", null, "/work", "a/b", ["db"]), null, "a name, not a path");
});

test("a name gets the first kind unless it has one of them", () => {
  assert.equal(withKind("data", ["db"]), "data.db");
  assert.equal(withKind("data.DB", ["db"]), "data.DB");
  assert.equal(withKind("data", []), "data");
});
