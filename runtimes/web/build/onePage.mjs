// The build's last step for a webview that shows one page and no folder
// (the C and C++ stacks, DESIGN 7.8): the script, the styles and the themes
// inlined into the page, and the pictures and sounds the screen finds by
// name as data URLs (`window.clarityResources`). Plain JavaScript, so a
// generated project's vite.config.ts imports it as it is.

import { existsSync, readFileSync, readdirSync } from "node:fs";
import { extname, join } from "node:path";

/** The kind of each picture and sound a page carries. */
export const KINDS = {
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".gif": "image/gif",
  ".bmp": "image/bmp",
  ".webp": "image/webp",
  ".svg": "image/svg+xml",
  ".wav": "audio/wav",
  ".mp3": "audio/mpeg",
  ".flac": "audio/flac",
};

/** Text safe inside a `<tag>` of the page: its closing tag written so it closes nothing. */
const safe = (code, tag) => code.replaceAll(`</${tag}`, `<\\/${tag}`);

/** The pictures and sounds of `folder` (public/resources) as data URLs, by name. */
export function carriedResources(folder) {
  const carried = {};
  for (const name of existsSync(folder) ? readdirSync(folder).sort() : []) {
    const kind = KINDS[extname(name).toLowerCase()];
    if (kind) carried[name] = `data:${kind};base64,${readFileSync(join(folder, name)).toString("base64")}`;
  }
  return carried;
}

/** The built page with its script, styles, themes (from `publicDir`) and resources in it. */
export function onePageHtml(page, script, style, publicDir) {
  page = page.replace(/<script type="module" crossorigin src="\.\/runtime\.js"><\/script>/, () => `<script type="module">${safe(script, "script")}</script>`);
  page = page.replace(/<link rel="stylesheet" crossorigin href="\.\/runtime\.css">/, () => `<style>${safe(style, "style")}</style>`);
  page = page.replace(/<link rel="stylesheet" href="\.\/(themes\/[^"]+)">/g, (_, file) => `<style>${safe(readFileSync(join(publicDir, file), "utf8"), "style")}</style>`);
  const carried = carriedResources(join(publicDir, "resources"));
  if (Object.keys(carried).length) page = page.replace("</head>", () => `<script>window.clarityResources = ${safe(JSON.stringify(carried), "script")};</script>\n</head>`);
  return page;
}

/** The Vite plugin: index.html becomes the whole page, without runtime.js and runtime.css beside it. */
export function onePage(publicDir = "public") {
  return {
    name: "clarity-one-page",
    enforce: "post",
    generateBundle(_, bundle) {
      const html = bundle["index.html"];
      const script = bundle["runtime.js"]?.code ?? "";
      const style = bundle["runtime.css"] ? String(bundle["runtime.css"].source) : "";
      html.source = onePageHtml(String(html.source), script, style, publicDir);
      delete bundle["runtime.js"];
      delete bundle["runtime.css"];
    },
  };
}
