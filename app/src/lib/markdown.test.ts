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
