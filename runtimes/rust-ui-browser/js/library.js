// The worker's functions the browser host imports (clarity_ui::browser::web),
// linked into clarity-ui.js by Emscripten (--js-library). Each calls what the
// worker script put on the module: clarityAsk, clarityAskLater, clarityTell,
// clarityResource.

addToLibrary({
  // Asks the program on the server: its answer as text the host frees, 0 for none.
  clarity_browser_ask__deps: ["$UTF8ToString", "$stringToNewUTF8"],
  clarity_browser_ask: function (command, commandLen, args, argsLen) {
    const answer = Module.clarityAsk(UTF8ToString(command, commandLen), UTF8ToString(args, argsLen));
    return answer == null ? 0 : stringToNewUTF8(answer);
  },

  // Asks the program on the server without waiting; the worker gives the answer to _clarity_ui_answered(id).
  clarity_browser_ask_later__deps: ["$UTF8ToString"],
  clarity_browser_ask_later: function (id, command, commandLen, args, argsLen) {
    Module.clarityAskLater(id, UTF8ToString(command, commandLen), UTF8ToString(args, argsLen));
  },

  // Sends the page a message (JSON).
  clarity_browser_tell__deps: ["$UTF8ToString"],
  clarity_browser_tell: function (json, jsonLen) {
    Module.clarityTell(UTF8ToString(json, jsonLen));
  },

  // A resource's bytes, copied into memory the host frees; 0 when there are none.
  clarity_browser_resource__deps: ["$UTF8ToString", "malloc"],
  clarity_browser_resource: function (name, nameLen, lenOut) {
    const bytes = Module.clarityResource(UTF8ToString(name, nameLen));
    if (!bytes) return 0;
    const at = _malloc(Math.max(bytes.length, 1));
    HEAPU8.set(bytes, at);
    HEAPU32[lenOut >> 2] = bytes.length;
    return at;
  },
});
