# Sign-in

Status: partial. Who is using the program: a `signin` block names the entity of people and its strategies from Clarity.Auth's catalog; `me`, `allow when`, `readable/writable when`; a guided sign-in window or the splash's sign-in part.

Progress: 70%

## Code
- Shared core: `runtimes/rust-auth` (`clarity-auth`: `catalog.rs`, `attempt.rs` (ops `signinBegin`, `signinDecide`), `hash.rs` Argon2id, `sessions.rs`, `tries.rs`, `remember.rs`, `account.rs`; `openid.rs` and `providers.rs` behind feature `openid`); C table `runtimes/rust-auth-ffi` (one JSON `call`)
- Project file: `crates/clarity-project/src/signin_file.rs` (`signin E on targets`, strategy lines, `minimum`, `session`, `tries`, `remember N days [when asked]`, `shows`, `language picker`, provider settings :75)
- Spec: `crates/clarity-spec/src/signin.rs` (model, defaults :145-147, `ProviderSettings` :83), `check/signin.rs`, `templates/signin.rs`; `me`, `hashed`, `allow when`, `readable/writable when`, `newToken`, `newRecoveryCodes`, page kind `signin`, targets `signin <strategy>`, `signout`; parts `signin`, `signinField`, `signinButton`, `signinRemember`, `signinLanguage`, `signinOther`, `signinRefusal` (`widgets.rs:62-74`, :709-733)
- Generators: one shared plan `crates/clarity-gen/src/signin.rs`, emitted from each stack's `model.rs`
- Runtimes: R `runtimes/rust/src/signin.rs` (crate direct, `attempt::signin_begin` :359); Go `runtimes/go/clarity/signin.go`, `signin_cli.go`, `runtimes/go/internal/authlib`; C `runtimes/c/auth/auth.c`, `clarity/people.c`; C++ `runtimes/cpp/auth/auth.cpp`, `clarity/people.cpp`; C# NuGet `Clarity.Auth` (`Signins.cs`, `AuthCalls.cs`), `Clarity.Runtime/Signin.cs`
- Clarity.UI: `runtimes/rust-ui/src/parts/signin.rs`, `parts/signin_view.rs`, `window/signin.rs`, `window/host/signin.rs`
- IDE: `app/src/lib/SigninSettings.svelte` (Sign-in setup, provider settings), `SigninView.svelte`, `signinView.ts`; Allow on pages and commands; entity editor Hashed column; New window "Sign-in"
- CLI: `clarity users <project> list|add|password|token|codes|remove` (`crates/clarity-cli/src/users.rs`)
- Docs: manual `project.signin.md`, `parts.signin.md`

## Tests
- `conformance/signin.json`: run by R, Go, C, C++, C# (`SigninTests.cs`), spec (`crates/clarity-spec/tests/spec/signin.rs`)
- `runtimes/rust-auth/tests/auth.rs`, `attempt.rs`, `providers.rs`, `openid.rs`; spec `signin.rs`, `signin_views.rs`; `crates/clarity-project/tests/signin.rs`; gen `signin_{rust,go,c,cpp,csharp}.rs`; `runtimes/rust-ui/tests/windows/signin.rs`, `signin_part.rs`, `signin_view.rs`; `crates/clarity-cli/tests/users.rs`; `app/src/lib/signinView.test.ts`
- Examples: todo (`signin Person on desktop`, password, account, remember), shop (Clerk, guided window)

## Open
- Entra ID single sign-on on desktop and web, with Entra groups: NOT BUILT, FIRST in line; see Design "Entra ID single sign-on".
- Provider tests on every stack (a conformance case against a fake Entra, a generated app signing in per stack): NOT BUILT; see Design "Entra ID single sign-on".
- Providers `github`, `microsoft`, `openid` (register off by default, desktop and web targets): PARTIAL: built on the desktop (released 0.41.0): the catalog (`runtimes/rust-auth/src/catalog.rs:43-45`), spec, project file (`client`, `secret`, `tenant`, `issuer`, `register` lines), the IDE setup, every generator passing a provider's settings (C `crates/clarity-gen/src/c/mod.rs:707`, C++ `cpp/mod.rs:201`, R, Go, C#) and every runtime reading them (C `runtimes/c/auth/auth.h` `clarity_signin_options`, C++ `runtimes/cpp/auth/auth.cpp`, Go `signin.go:43`, C# `Signins.cs:28`), and the browser sign-in run by Clarity.UI (`runtimes/rust-ui/src/window/signin.rs:146`, called at :121, `providers::through_browser`; rust-ui turns on feature `openid`, `runtimes/rust-ui/Cargo.toml:54`). Not on: clarity-runtime's `auth` feature (`runtimes/rust/Cargo.toml:36`) and `rust-auth-ffi` (`Cargo.toml:19`) build without `openid`, so a provider decided there says "built without the feature openid" (`runtimes/rust-auth/src/lib.rs:84`). Web flow, command-line flow and a register part: NOT BUILT; see Design "Providers on the web and the command line".
- `maillink` and mail resets: NOT BUILT (needs [mail](mail.md)); see Design "maillink".
- TOTP: NOT BUILT; see Design "A second factor".
- Passkeys: NOT BUILT; see Design "Passkeys".
- LDAP, SAML, strategies from a library: NOT BUILT; see Design "Later strategies".
- Switching person on the desktop without a restart: NOT BUILT; see Design "Open questions".
- Each generator's sign-in emitting in a module of its own: NOT BUILT ([modularity](modularity.md) step 1, next).

## Design

Each new strategy is one more entry of Clarity.Auth's catalog (`runtimes/rust-auth/src/catalog.rs`), named on a line of the `signin` block with the fields it matches; the checker, the Sign-in setup and the generators read it from the catalog.

### Providers on the web and the command line

- Web: the browser goes to the provider and back to the served program; Clarity.Auth exchanges the code and checks the token against the provider's keys. Then the `clarity_session` cookie as for every strategy.
- Command line: the device code flow.
- Both need the feature `openid` in clarity-runtime's `auth` and in `rust-auth-ffi`.
- A register part: optional, on a sign-in view, for a provider with `register`.

### Entra ID single sign-on

Asked 2026-11-23. User: "I noticed that authentication might also need more tests (strategies)?! I have an urgent need for SSO", then "Entra ID, both desktop and web, groups yes", "first entra, then scim".

Where it stands: the `microsoft` provider signs in on the desktop only, through Clarity.UI's browser sign-in; clarity-runtime's `auth` and `rust-auth-ffi` build without `openid`. The person is found through the userinfo endpoint with the access token: the ID token is not read, its signature not checked, no `nonce`. No groups.

What it needs:
- Desktop and web: the web flow through the served program (Design "Providers on the web and the command line"), the feature `openid` on in clarity-runtime's `auth` and in `rust-auth-ffi`, so every stack has it.
- The ID token checked: signature against Entra's published keys (discovery's `jwks_uri`), `nonce`, issuer (the tenant), audience (the client), expiry.
- Entra groups with the person, usable in `allow when`.
- Tests: a conformance case of the provider flow against a fake Entra (discovery, keys, token, a refused, expired or wrongly signed token, a wrong `state`, another tenant) run by every stack, and a generated app per stack signing in through it.

Open questions (names and choices for the user, none used yet):
- How groups reach the program: Entra's `groups` claim (object ids; over 200 groups Entra sends an overage pointer to Graph instead), or app roles (`roles` claim, names the app registration defines). Which one, or both.
- How `allow when` names a group: by Entra's object id, or by a name the project maps to an id. The word for it in Clarity (for example `me.groups`): to ask.
- Whether a person met for the first time through Entra is added to the sign-in entity by itself (`register` exists for providers, off by default) or must exist first (as SCIM will make them).
- The command line (device code flow): not asked for now ("both desktop and web").

### maillink

- `maillink email from outbox`: a link sent to the address in that field through a mail source ([mail](mail.md)); Clarity.Auth makes and checks the one-time codes.
- Mail resets of a `password` come with it.

### A second factor

- TOTP by an authenticator app, with `recovery` codes as its backup. Agreed to come right after sign-in ("1, totp right after step 4").

### Passkeys

- The next strategy after TOTP ("passkeys come later then").
- Web first (WebAuthn; the project names its domain; HTTPS or localhost), then the desktop (Windows Hello and macOS through their system APIs, security keys through libfido2 on Linux).
- Several per person, so a `list<Passkey>` field, and a click target to add one once signed in.

### Later strategies

- LDAP for an office directory; SAML; browser sign-in through a Windows domain (Kerberos). `account` already covers the Windows user on the desktop.
- A strategy written in a project's library, in the project's own language, as a library format is.

### Open questions

- Whether a signed-in desktop program keeps one person for the run, or may switch person without restarting (a shared family computer).
- The hand-built way (inputs named after a strategy's inputs, `signin <strategy>` buttons) on the splash: proposed kept beside the sign-in part.
- GitHub's client secret on the desktop: its device flow, or the loopback flow with the secret carried; to decide from GitHub's documentation.

## Conflicts
- Sign-in defaults (8 h, 5 tries, 15 min) are written in `crates/clarity-spec/src/signin.rs:145-147`, `runtimes/rust-auth/src/attempt.rs:157`, `runtimes/rust/src/signin.rs:78`, `runtimes/go/clarity/signin.go:225-231`, `runtimes/cpp/clarity/people.hpp:69-72` (equal today).

## Decisions
- 2026-11-23: Entra ID single sign-on first, on desktop and web, with groups. User: "Entra ID, both desktop and web, groups yes", "first entra, then scim".
- 2026-11-12: strategies from one catalog, several per project, each naming its fields. User: "are you going to use authentication strategies?"
- 2026-11-12: steps 4, 5, 6 as one slice, built 4 first. User: "think it kinda combines with multi user app... so 4 too then?", then "yes, build it"
- 2026-11-12: `allow when` per case, no fixed roles; rows never filtered by magic (`readable when` is a check).
- 2026-11-12: minimum, sign-out on a change, recovery codes in step 4; TOTP and SSO right after. User: "1, totp right after step 4"; passkeys: "passkeys come later then"
- 2026-11-12: the splash can be the sign-in window. User: "ui wise... the splash screen can double for a login screen, right?", choosing "One window does both"
- 2026-11-12: a sign-in part whose settings are the project's. User: "this needs a signin part that can be put on it own window or the splash and should be configurable from the 'back-end' parts", "Keep both", "Move them to the project"
- 2026-11-12: the sign-in window is window, content, sign-in views, guided. User: "a sign in window cannot hold every sort of part, it must be guided!", "one sign in must be able to switch strategy on demand... mail login or those github logins... what is normal on webpages, should be normal on desktop!"
- 2026-11-11: extras such as the language picker are options, off by default. User: "not permanent!!!! NOTHING IS PERMANENT!!!"
- 2026-11-11: one OpenID flow in clarity-auth for apps and the IDE's Foundry sign-in. User: "maybe sso (entra id) brings something useful to this route?!"
- 2026-11-12: sign-in first gets one module per layer. User: "yes, do that first... and that needs to happen for every specialized (modular) piece in the ecosystem!!!"

## Was
- docs/APP-CAPABILITIES-PLAN.md step 4 (all), "Steps 4, 5 and 6 as one slice" ("One person per call"), "Order" 4, "Where the code lives" (sign-in), open questions (SSO, passkeys, strategies, switching person, guided sign-in, providers)
- FOUNDRY-PLAN.md "One sign-in for apps and the IDE"; MODULAR-PLAN.md order 1
- IMPLEMENTED.md 6 "Sign-in"; UNIMPLEMENTED.md 6 "Sign-in", 12 step 1; CONFLICTING.md 4, 5, 6, 9
- memory: guided-signin.md, options-not-fixtures.md
