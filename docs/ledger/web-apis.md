# Web APIs and HTTP

Status: partial. `http` calls from code with a reply that never throws, `rest` data sources with auth and paging, and OpenAPI descriptions turned into a generated module of entities and actions.

## Code
- Project file: `datasource name rest "url"` with `auth bearer|api-key|basic "env:..|setting:.."`, `paging page|offset|cursor|link ...`, `openapi "file"`, `pick ...`, `pick schema ...` (`crates/clarity-project/src/file.rs:474-540`)
- OpenAPI: `crates/clarity-project/src/openapi.rs` (OpenAPI 3, Swagger 2), `refresh_apis` (`crates/clarity-project/src/lib.rs:1560`) writes `apis/<source>.clarity`
- Spec: `ExprKind::Http` (`http get|post|put|patch|delete url [from source] [with body] [header ..] [query ..] [when ..] [every page]`), type `http reply`, effect `net`; records held inside records only in web-API entities (`code/check.rs:5252`); web-API entities are never found or saved (`code/check.rs:5336`)
- Generators: calls filled in before a stack is written (`crates/clarity-gen/src/calls.rs`); `when` per stack in each `logic.rs`
- Runtimes: R `runtimes/rust/src/http.rs` (ureq, feature `http`; 1000-page cap :178); Go `runtimes/go/clarity/http.go`; C `runtimes/c/http/http.c` and C++ `runtimes/cpp/http/http.cpp` (libcurl, linked only when code calls out); C# `Clarity.Runtime/Http.cs`
- IDE: `app/src/lib/ApiPanel.svelte` (pick operations and schemas, Paging form), `ApiView.svelte`, `app/src-tauri/src/apis.rs`; agent tools `api`, `pick_api` (`crates/clarity-tools/src/lib.rs:827-941`)
- Docs: manual `code.web-apis.md`; LANGUAGE.md section 10 "Calling web APIs" (:2850), section 2 (`auth` :90)

## Tests
- `conformance/http.json` (calls against each runtime's own test server, paging on `/paged`, `/offset`, `/cursor`, `/linked`): run by R, Go, C, C++, C#
- `crates/clarity-spec/tests/http.rs`, `web_sources.rs`, `nested.rs`; `crates/clarity-project/tests/openapi.rs`
- `crates/clarity-gen/tests/http.rs`, `paging.rs`, `http_build.rs` (ignored; reads a web API, sends records, signs with a secret setting on every stack)

## Open
- REST entities mapped onto endpoints (list, get, create, update, delete), browsed and saved through the API: PARTIAL (DESIGN.md:140). An OpenAPI file gives entities and one action per operation; entities are never found or saved through the API.
- ETag and `If-Match` on writes: NOT BUILT, open question; see [row-versions](row-versions.md).
- Background calls so a slow call does not hold the window: NOT BUILT; see [background-jobs](background-jobs.md).
- Mail as the next Internet connection: see [mail](mail.md).

## Conflicts
- HTTP timeout text: C# says "no answer within 30 seconds" (`runtimes/csharp/Clarity.Runtime/Http.cs:260`); the others pass the library's text; `http.json` checks only the start.
- The 30-second timeout is written in each runtime (CONFLICTING.md 4; equal today).
- manual `code.web-apis.md` uses snake_case action names (:117-121; OpenAPI actions are camelCase, `crates/clarity-project/src/openapi.rs:260-283`) and a quoted text in `message` (:16), which must be a key.
- LANGUAGE.md grammar summary (:3028-3149) lacks `http patch` and `every page`.

## Decisions
- 2026-10-24: a call is an expression and never stops the handler: a failure is a reply with status 0 and why (DESIGN 5.23).
- 2026-10-24: calls in handlers, commands and actions with `effects net`; never in functions, engines or data events.
- 2026-10-24: a header holding a key as plain text is a warning; keys come from a setting or the environment.
- 2026-10-24: only http and https; 30-second timeout; redirects followed; each stack its own client.
- 2026-10-25: an OpenAPI description in the project's `apis` folder; schemas become entities, a new revision updates them, the project picks what it uses (D2).
- 2026-10-25: optional query and header parameters are parameters with defaults, named values at calls, `when` leaves a pair out; filled in before generation (DESIGN 5.24, chosen out of three).
- 2026-10-25: nested objects are records inside records, only in web-API entities (DESIGN 5.25, chosen out of three).
- 2026-10-29: paging is said once on the source; `every page` gives one reply of every page's items; at most 1000 pages.
- 2026-10-29: the agent lists and picks operations (`api`, `pick_api`).

## Was
- DESIGN 1.3 (REST and other web APIs paragraph), 5.23, 5.24 (as used by http calls; parameter defaults in general are in [language](language.md)), 5.25, roadmap HC1, D2
- LANGUAGE.md section 10 "Calling web APIs"
- IMPLEMENTED.md 6 "HTTP and web APIs"
- UNIMPLEMENTED.md 2 "Databases" (REST entities)
- CONFLICTING.md 2 (timeout text), 4 (timeout), 6 (grammar: http), 8 (code.web-apis.md)
