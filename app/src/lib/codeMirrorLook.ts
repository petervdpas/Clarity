// CodeMirror in the IDE's own colors, so its editors follow the light and
// dark theme: the Clarity editor and the editor of a library's files.

import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { tags } from "@lezer/highlight";

const colors = HighlightStyle.define([
  { tag: [tags.keyword, tags.modifier, tags.controlKeyword, tags.operatorKeyword], color: "var(--code-keyword)" },
  { tag: [tags.typeName, tags.className, tags.namespace], color: "var(--code-type)" },
  { tag: [tags.string, tags.number, tags.bool, tags.null, tags.atom], color: "var(--code-literal)" },
  { tag: [tags.comment, tags.lineComment, tags.blockComment], color: "var(--code-comment)", fontStyle: "italic" },
  { tag: [tags.processingInstruction, tags.meta], color: "var(--code-key)" },
  { tag: [tags.function(tags.variableName), tags.macroName], color: "var(--code-slot)" },
]);

const theme = EditorView.theme({
  "&": { height: "100%", color: "var(--code-text, var(--text))", backgroundColor: "var(--bg)", fontSize: "13px" },
  ".cm-content": { fontFamily: "var(--mono)", caretColor: "var(--text)" },
  ".cm-gutters": { backgroundColor: "var(--panel-alt)", color: "var(--text-muted)", borderRight: "1px solid var(--border)" },
  ".cm-activeLine, .cm-activeLineGutter": { backgroundColor: "color-mix(in srgb, var(--text) 6%, transparent)" },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection": { backgroundColor: "color-mix(in srgb, var(--code-keyword) 25%, transparent)" },
  ".cm-panels": { backgroundColor: "var(--panel)", color: "var(--text)" },
  ".cm-placeholder": { color: "var(--text-muted)" },
  // Clarity's pieces, as the backend's lexer reads them.
  ".cm-clarity-keyword": { color: "var(--code-keyword)" },
  ".cm-clarity-type": { color: "var(--code-type)" },
  ".cm-clarity-literal, .cm-clarity-number": { color: "var(--code-literal)" },
  ".cm-clarity-text": { color: "var(--code-text)" },
  ".cm-clarity-slot": { color: "var(--code-slot)" },
  ".cm-clarity-key": { color: "var(--code-key)" },
  ".cm-clarity-comment": { color: "var(--code-comment)", fontStyle: "italic" },
});

/** The IDE's colors and look for a CodeMirror editor. */
export const ideLook = [syntaxHighlighting(colors), theme];
