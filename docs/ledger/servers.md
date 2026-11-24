# Servers

Status: not built. Servers a desktop app runs beside its window, each declared in the project: a built-in kind (SCIM, wiki, REST API) or the project's own endpoints and handlers; beside each desktop first, or replaced by one central server.

Progress: 0%

## Code
- None yet. Related: the web target's served program ([web-target](web-target.md)), sign-in and its entity of people ([signin](signin.md)), server databases ([databases](databases.md)).

## Tests
- None yet.

## Open
- Declared servers beside a desktop app: NOT BUILT; see Design.
- The SCIM kind: NOT BUILT, next after Entra ID sign-in ([signin](signin.md) "Entra ID single sign-on"); see Design "SCIM".
- The wiki kind: NOT BUILT; see Design "Wiki".
- The REST API kind: NOT BUILT; see Design "REST API".
- One central server in place of the ones beside each desktop: NOT BUILT; see Design "Where a server runs".
- Open questions (Design): needs decision.

## Design

Asked 2026-11-23, from Entra ID and SCIM. User: "okay, background servers for desktop apps then (as a more generic proclamation)".

### What a server is

- A project declares its servers by name; a desktop app can have several running beside it. User: "maybe a desktop app can have multiple servers running next to it?!"
- A server is of a built-in kind, or declared with its own endpoints and the handlers behind them. User: "both, declared servers with built-in kinds like SCIM".
- The built-in kinds are SCIM, wiki and REST API, each a server of its own. User: "yes, SCIM, wiki and REST API, each its own server, although its very dependent on context of the app of course". So an app has only the servers its project declares; none is there by default.
- The model the user pointed at (a desktop app of theirs; port the construct only): the app runs an internal HTTP server beside its window, switched on by a setting with a port, started and stopped while the app runs and showing whether it runs and on which port; a wiki (HTML pages of the app's data) and a REST API (create, read, update, delete, with Swagger UI) reading through one read-only layer over the data; small helper servers on loopback with a free port.

### Where a server runs

- A project choice. User: "that is a project choice/option... but both... per desktop first and can be replaced with a centralized one".
- Beside each desktop app first: the server runs on the same machine with the same data as the desktop program.
- Replaced by one central server: one program on a company machine, on the shared server database the desktop apps use. SCIM needs this in practice: Entra calls one fixed address.

### SCIM

- RFC 7643 and 7644: the identity provider (Entra ID) pushes people into the app: `/scim/v2/Users` and `/Groups`, create, update, deactivate, checked by a bearer token. A person removed in Entra is deactivated in the app without signing in.
- Maps onto the sign-in entity of people (and a group entity) of [signin](signin.md).
- Order: right after Entra ID sign-in. User: "first entra, then scim".

### Wiki

- HTML pages of the app's data, served beside the app (as in the model). What it shows and how it is designed: to ask.

### REST API

- The app's entities over HTTP: create, read, update, delete, with an OpenAPI description and Swagger UI (as in the model). Which entities, and who may call: to ask (sign-in's `allow when`, a token).

### Open questions (names and choices for the user, none used yet)
- How a server is declared in the project file (a `server` line or block? its name, kind, port, where it runs). The word "server" is the user's; the syntax is not decided.
- How a server is started: with the desktop app, or as a system service (Windows service, systemd, launchd) running without it.
- How the desktop app and a server beside it share data: the same SQLite file (writes from two processes) or the server owning it.
- How a declared server's own endpoints are written: paths and the handlers (actions) behind them, on every stack.
- How the wiki is designed (windows/views in the IDE, Markdown, or generated from the model) and which data it shows.
- Ports: fixed per server, chosen by the person, or free on loopback for local-only ones.
- What runs where on each stack and system (every stack writes its own server code, as the web target does).

## Conflicts
- None yet.

## Decisions
- 2026-11-23: servers beside desktop apps, as a general thing. User: "okay, background servers for desktop apps then (as a more generic proclamation)".
- 2026-11-23: where a server runs is a project option, beside each desktop first, replaceable by a central one; several per app. User: "that is a project choice/option... but both... per desktop first and can be replaced with a centralized one... maybe a desktop app can have multiple servers running next to it?!"
- 2026-11-23: declared servers and built-in kinds; SCIM, wiki and REST API, each its own server. User: "both, declared servers with built-in kinds like SCIM", "yes, SCIM, wiki and REST API, each its own server, although its very dependent on context of the app of course".
- 2026-11-23: order. User: "first entra, then scim".
