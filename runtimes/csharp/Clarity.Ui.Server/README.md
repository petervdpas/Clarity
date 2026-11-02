# Clarity.Ui.Server

The server of a [Clarity.UI](https://github.com/petervdpas/Clarity) program shown in browsers. Each browser tab has a session of the program's windows on the server, where their handlers run against the server's databases. The browser program (Clarity.Ui.Browser) draws each window from its markup over a mirror of that state and sends what the person does.

```csharp
using var db = Databases.Open();
var sessions = new Sessions(() => new MainViewModel(db));
new WebServer(sessions, "wwwroot", typeof(MainViewModel).Assembly).Serve("Shop", WebServer.Address(args, "127.0.0.1:8080"));
```

It answers:

- `POST /clarity/session`: a session's request (`start`, `run`, `answer`, `fetch`, `close`) and its reply, as JSON.
- `GET /clarity/markup`: the windows' and app frames' markup and the themes the application carries, as one JSON object by name (`Windows/Main.cxaml`).
- Any other `GET`: a file of the browser program's folder, its Brotli or gzip copy to a browser that takes it.

It speaks HTTP/1.1 over a plain socket, so on Windows a person who is not an administrator can listen on 127.0.0.1 or 0.0.0.0. `--address host:port`, or the environment variable `CLARITY_ADDRESS`, makes it listen elsewhere.
