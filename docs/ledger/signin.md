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
- `conformance/signin.json`: run by R, Go, C, C++, C# (`SigninTests.cs`), spec (`crates/clarity-spec/tests/signin.rs`)
- `runtimes/rust-auth/tests/auth.rs`, `attempt.rs`, `providers.rs`, `openid.rs`; spec `signin.rs`, `signin_views.rs`; `crates/clarity-project/tests/signin.rs`; gen `signin_{rust,go,c,cpp,csharp}.rs`; `runtimes/rust-ui/tests/windows/signin.rs`, `signin_part.rs`, `signin_view.rs`; `crates/clarity-cli/tests/users.rs`; `app/src/lib/signinView.test.ts`
- Examples: todo (`signin Person on desktop`, password, account, remember), shop (Clerk, guided window)

## Open
- Providers `github`, `microsoft`, `openid` (register off by default, desktop and web targets): PARTIAL: built on the desktop (released 0.41.0): the catalog (`runtimes/rust-auth/src/catalog.rs:43-45`), spec, project file (`client`, `secret`, `tenant`, `issuer`, `register` lines), the IDE setup, every generator passing a provider's settings (C `crates/clarity-gen/src/c/mod.rs:707`, C++ `cpp/mod.rs:201`, R, Go, C#) and every runtime reading them (C `runtimes/c/auth/auth.h` `clarity_signin_options`, C++ `runtimes/cpp/auth/auth.cpp`, Go `signin.go:43`, C# `Signins.cs:28`), and the browser sign-in run by Clarity.UI (`runtimes/rust-ui/src/window/signin.rs:146`, called at :121, `providers::through_browser`; rust-ui turns on feature `openid`, `runtimes/rust-ui/Cargo.toml:54`). Not on: clarity-runtime's `auth` feature (`runtimes/rust/Cargo.toml:36`) and `rust-auth-ffi` (`Cargo.toml:19`) build without `openid`, so a provider decided there says "built without the feature openid" (`runtimes/rust-auth/src/lib.rs:84`). Web flow, command-line flow and a register part: NOT BUILT.
- `maillink`: NOT BUILT (needs [mail](mail.md)). TOTP, passkeys, LDAP, strategies from a library: NOT BUILT.
- Switching person on the desktop without a restart: NOT BUILT (open question).
- Each generator's sign-in emitting in a module of its own: NOT BUILT (MODULAR-PLAN.md order 1, next).
- Open questions: the hand-built way on the splash (proposed: kept); GitHub's client secret on the desktop (device flow or loopback).

## Conflicts
- Sign-in defaults (8 h, 5 tries, 15 min) are written in `crates/clarity-spec/src/signin.rs:145-147`, `runtimes/rust-auth/src/attempt.rs:157`, `runtimes/rust/src/signin.rs:78`, `runtimes/go/clarity/signin.go:225-231`, `runtimes/cpp/clarity/people.hpp:69-72` (equal today).
- APP-CAPABILITIES-PLAN.md:3 says step 4 is unreleased; it is before tag v0.40.0.

## Decisions
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
- APP-CAPABILITIES-PLAN.md step 4 (all), "Steps 4, 5 and 6 as one slice" ("One person per call"), "Where the code lives" (sign-in), open questions (SSO, passkeys, strategies, switching person, guided sign-in, providers)
- FOUNDRY-PLAN.md "One sign-in for apps and the IDE"; MODULAR-PLAN.md order 1
- IMPLEMENTED.md 6 "Sign-in"; UNIMPLEMENTED.md 6 "Sign-in", 12 step 1; CONFLICTING.md 4, 5, 6, 9
- memory: guided-signin.md, options-not-fixtures.md
