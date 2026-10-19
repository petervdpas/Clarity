// The agent's answers are Markdown: shown with bold, code, lists and
// paragraphs, and never as HTML of their own.

import { test } from "node:test";
import assert from "node:assert/strict";

import { markdown } from "./markdown.ts";

test("paragraphs, bold, code and lists", () => {
  const text = "The todo app is built.\n\n- **Data:** a `Task` has a title\n- **Buttons:** Delete\n\nThe Main window is open.";
  assert.equal(
    markdown(text),
    "<p>The todo app is built.</p><ul><li><strong>Data:</strong> a <code>Task</code> has a title</li><li><strong>Buttons:</strong> Delete</li></ul><p>The Main window is open.</p>",
  );
});

test("numbered lists, headings and code blocks", () => {
  assert.equal(markdown("## Steps\n1. Open\n2. Save"), "<h4>Steps</h4><ol><li>Open</li><li>Save</li></ol>");
  assert.equal(markdown("```\nsave grid.selected\n  reload tasks\n```"), "<pre><code>save grid.selected\n  reload tasks</code></pre>");
});

test("HTML in the text is shown as text", () => {
  assert.equal(markdown("<b>hi</b> & **<i>x</i>**"), "<p>&lt;b&gt;hi&lt;/b&gt; &amp; <strong>&lt;i&gt;x&lt;/i&gt;</strong></p>");
});

test("lines of a paragraph keep their breaks", () => {
  assert.equal(markdown("one\ntwo"), "<p>one<br>two</p>");
});

test("an answer shows a link's text, never a link", () => {
  assert.equal(markdown("See [Queries](code.queries.md)."), "<p>See Queries.</p>");
});

test("a table shows its header and rows", () => {
  assert.equal(
    markdown("| Key | Does |\n|---|---|\n| `F1` | Opens the **manual** |"),
    "<table><thead><tr><th>Key</th><th>Does</th></tr></thead><tbody><tr><td><code>F1</code></td><td>Opens the <strong>manual</strong></td></tr></tbody></table>",
  );
});

test("a page has real headings with anchors, links to follow, and flowing paragraphs", () => {
  assert.equal(markdown("# Queries\n\n## Text with slots", { page: true }), '<h1 id="queries">Queries</h1><h2 id="text-with-slots">Text with slots</h2>');
  // A link keeps where it goes for the page to follow; the text is escaped.
  assert.equal(markdown("See [the <b>reference</b>](../LANGUAGE.md#queries).", { page: true }), '<p>See <a href="#" data-link="../LANGUAGE.md#queries">the &lt;b&gt;reference&lt;/b&gt;</a>.</p>');
  // A link that leaves for elsewhere is never followed in the window.
  assert.equal(markdown("[x](https://example.com)", { page: true }), "<p>x</p>");
  assert.equal(markdown("one\ntwo", { page: true }), "<p>one two</p>");
});
