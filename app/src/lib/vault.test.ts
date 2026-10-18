// What the Vault tab shows and refuses, before anything reaches the vault.

import { test } from "node:test";
import assert from "node:assert/strict";

import { connectionProblems, freshName, shownSecrets } from "./vault.ts";
import type { ConnectionRow } from "./generated/ConnectionRow.ts";
import type { SecretRow } from "./generated/SecretRow.ts";

const rows: SecretRow[] = [
  { category: "Anthropic", key: "apikey", description: "anthropic-version: 2023-06-01", created: "2026-04-27T12:15:00Z", updated: "2026-04-27T12:15:00Z" },
  { category: "Azure", key: "apple", description: "fruit:Apple", created: "2026-04-26T13:28:00Z", updated: "2026-04-26T13:28:00Z" },
  { category: "Azure", key: "banana", description: "fruit:Banana", created: "2026-04-26T13:28:00Z", updated: "2026-04-26T13:28:00Z" },
];

test("secrets show by category, and a filter finds text in any column ignoring case", () => {
  assert.equal(shownSecrets(rows, null, "").length, 3);
  assert.deepEqual(shownSecrets(rows, "azure", "").map((r) => r.key), ["apple", "banana"]);
  assert.deepEqual(shownSecrets(rows, null, "BANANA").map((r) => r.key), ["banana"]);
  assert.deepEqual(shownSecrets(rows, null, "anthropic").map((r) => r.key), ["apikey"]);
  assert.deepEqual(shownSecrets(rows, "Azure", "anthropic"), []);
});

test("a connection needs a name, and fields with names of their own", () => {
  const good: ConnectionRow = {
    name: "shop-db",
    fields: [
      ["host", { mode: "plain", value: "localhost" }],
      ["password", { mode: "vault", category: "Postgres", key: "password" }],
    ],
  };
  assert.deepEqual(connectionProblems(good), []);
  assert.deepEqual(connectionProblems({ ...good, name: " " }), ["The connection needs a name."]);
  assert.deepEqual(connectionProblems({ ...good, fields: [...good.fields, ["Host", { mode: "plain", value: "x" }]] }), [
    "Field Host is there twice.",
  ]);
  assert.deepEqual(connectionProblems({ ...good, fields: [["", { mode: "plain", value: "x" }]] }), ["Every field needs a name."]);
  assert.deepEqual(connectionProblems({ ...good, fields: [["token", { mode: "vault", category: "GitHub", key: "" }]] }), [
    "Field token needs the category and key of its secret.",
  ]);
});

test("a new connection gets a name not in use yet", () => {
  assert.equal(freshName([]), "connection");
  assert.equal(freshName(["connection", "Connection 2"]), "connection 3");
});

import { CategoryEditor } from "./vault.ts";

test("the categories editor adds, renames and removes, keeping the secrets with their category", () => {
  const usage = new Map([["postgres", 2]]);
  const editor = new CategoryEditor(["Postgres", "azure", "Mail"], usage);
  assert.deepEqual(editor.items, ["azure", "Mail", "Postgres"]);

  assert.equal(editor.add("mail"), "A category named 'mail' already exists.");
  assert.equal(editor.add(" Anthropic "), null);
  assert.deepEqual(editor.items, ["Anthropic", "azure", "Mail", "Postgres"]);

  assert.equal(editor.remove("Postgres"), "Can't remove 'Postgres': 2 secret(s) still use it. Edit those secrets to reassign, then remove the category.");
  assert.equal(editor.remove("Mail"), null);

  assert.equal(editor.rename("Postgres", "azure"), "A category named 'azure' already exists.");
  assert.equal(editor.rename("Postgres", "Database"), null);
  // A rename of a rename still moves the secrets from the first name.
  assert.equal(editor.rename("Database", "DB"), null);
  // A fresh category renamed is no rename of secrets.
  assert.equal(editor.rename("Anthropic", "Claude"), null);
  assert.deepEqual(editor.result(), { names: ["azure", "Claude", "DB"], renames: [{ from: "Postgres", to: "DB" }] });
  assert.equal(editor.usage("DB"), 2);
});
