# Clarity.Ui.Server

The server of a [Clarity.UI](https://github.com/petervdpas/Clarity) program shown in browsers. Clarity.UI runs in the browser (its browser files, compiled to WebAssembly once for every language) and asks the program on the server for everything a window does, as the native library asks it on the desktop; the windows' view models live here, where their handlers run against the server's databases.

```csharp
using var db = Databases.Open();
new WebServer(new MainViewModel(db)).Serve("Shop", WebServer.Address(args, "127.0.0.1:8080"));
```

The application's assembly carries what it serves: the browser files under `browser/` (`clarity-ui.js`, `clarity-ui.wasm`, `clarity-ui-worker.js`, `clarity-ui-page.js`, which Clarity's generator puts in), the windows' and app frames' markup, the themes, `model.json` and the files of the resources folder.

It answers:

- `POST /clarity/invoke`: a command of the library (`{"command", "args", "browser", "settings"}`), answered `{"value"}` or `{"error"}`, and `{}` where the program leaves the library's default. `browser` names the page (one per tab) and `settings` are its person's settings as the page keeps them: a handler runs with them and in that page's language, `quit` and `keys` from code go to that page's reply alone, and settings a handler changed come back in the answer's `settings` for the page to keep.
- `GET /` (or `/index.html`): the page, a canvas and the page script.
- `GET /clarity/files.json`: the program's files as the library reads them (title, size, windows, apps, themes, model, the resources' names).
- `GET /clarity/resources/<name>`: a file of the resources folder.
- `GET /<browser file>`: a browser file, compressed with Brotli or gzip (once) to a browser that takes it.

Each browser's windows have their own long call, and a window a browser left alone for half an hour (a tab that closed) is let go.

It speaks HTTP/1.1 over a plain socket, so on Windows a person who is not an administrator can listen on 127.0.0.1 or 0.0.0.0. `--address host:port`, or the environment variable `CLARITY_ADDRESS`, makes it listen elsewhere.
