// The clipboard of a running application (`copy text`): the text goes on the
// system's clipboard. A desktop webview may refuse the clipboard API, as it
// asks for a secure page or a gesture of the person; then a hidden text area
// is selected and copied, as pages did before that API.

/** Puts `text` on the clipboard; false when the page may not. */
export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    const area = document.createElement("textarea");
    area.value = text;
    area.setAttribute("readonly", "");
    area.style.position = "fixed";
    area.style.opacity = "0";
    document.body.append(area);
    area.select();
    try {
      return document.execCommand("copy");
    } catch {
      return false;
    } finally {
      area.remove();
    }
  }
}
