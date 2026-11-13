# Releases and CI

Status: partial. GitHub workflows run the checks on push and publish by tag: IDE installers, the Clarity.UI libraries, crates, NuGets, the VS Code extension; versions per runtime.

Progress: 80%

## Code
- Workflows: `.github/workflows/checks.yml` (push to main, or by hand for one part: Linux; the ffi libraries, browser files, workspace tests, app `npm run check` and tests, Go, C, C++, C# runtime tests, examples each on its stack); `release.yml` (tag `v*`: Windows NSIS and MSI, Linux .deb amd64 and arm64, .rpm Fedora 43 and 44, each install-tested, the `.vsix`; the tag stamps the app version, :8-10); `release-clarity-ui.yml` (tag `runtimes/rust-ui-ffi/v*`: native libraries for linux, win, osx, x64 and arm64, and browser files); `release-csharp-runtime.yml` (tag `runtimes/csharp/v*`: 14 NuGets with Clarity.AppMcp, checked at :198, trusted publishing); `release-rust-{runtime,ui,audio,video,image,formats,graph,auth,engine,app-mcp}.yml` (tag `runtimes/rust*/v*`: one crate each); `release-vscode.yml` (tag `editors/vscode/v*`)
- Scripts: `.github/scripts/build-linux.sh`, `install-test.sh`, `stage-clarity-ui.sh`, `stage-clarity-cli.sh`, `fetch_openh264.py`
- Go: tag `runtimes/go/v*`, no workflow; sums recorded after the tag in `crates/clarity-gen/src/go/runtime.sum`
- Versions in tree: `runtimes/rust/Cargo.toml` 0.59.0, `runtimes/go/clarity/version.go` v0.62.0, `runtimes/csharp/Directory.Build.props` 0.96.0, `runtimes/rust-ui-ffi` 0.24.0, `runtimes/rust-ui` 0.25.0, `runtimes/rust-app-mcp` 0.1.0, `runtimes/rust-engine` 0.3.1, workspace 0.1.0; C and C++ runtimes are copied into programs and have no version
- Release plans: `releases/<YYYY-MM-DD-HHMM>.json` (gitignored), run by the Release Helper (`~/Projects/ReleaseHelper/README.md`)
- Rules: CLAUDE.md "Releasing" (:58-66) and "Generated-file guards" (:42-56); see [os-checks](os-checks.md) for systems

## Tests
- The workflows themselves; generated-file guards (CLAUDE.md:48-54) fail a release's checks when a file is behind

## Open
- A Windows (and macOS) job in `checks.yml`: NOT BUILT (DESIGN.md:1704, 2484). Windows runners appear only in release workflows.
- A macOS IDE installer: NOT BUILT.
- Go's file-source tests are skipped: they carry the `clarityformats` tag but CI runs `-run Formats` (`checks.yml:207`, `runtimes/go/clarity/file_source_test.go`).
- The server-database tests are not run (no database variables in CI).
- An offline CI job: see [offline](offline.md).

## Conflicts
- Two Clarity.UI versions: `runtimes/rust-ui/Cargo.toml` 0.25.0, `runtimes/rust-ui-ffi/Cargo.toml` 0.24.0; downloads follow the ffi version.
- The tree's version (workspace, `app/package.json`, `tauri.conf.json`) is 0.1.0; releases are v0.44.0, stamped from the tag.
- `runtimes/rust/README.md:26` shows `clarity-runtime = "0.10.0"`; the crate is 0.59.0.
- `release-csharp-runtime.yml:4` says eleven packages; it publishes 14.
- `editors/vscode/README.md:34` installs 0.1.3; the extension is 0.1.38.
- Plans say unreleased for work in tags: FORMATS-PLAN.md:3 (steps 2-5 in v0.39.0), APP-CAPABILITIES-PLAN.md:3 (sign-in and row versions before v0.40.0), GRAPH-PLAN.md:3 and AGENT-TOOLS-PLAN.md:3 (in v0.39.0).

## Decisions
- 2026-10-21: crates publish by tag through GitHub Actions, never by hand. User: "sure, but I didnt do this for nothing eh!!!!!"
- 2026-10-23: one runtime version bump per batch of slices. User: "put more changes into one runtime release change!!!"
- 2026-11-04: tags in order (a crate before what needs it, a library before what fetches it, Go sums after the Go tag), enforced by the Release Helper.
- 2026-11-09: what GitHub Actions does (packing NuGets, release libraries, installers, whole suites) is not redone locally. User: "dont do stuff git actions normally does eh"
- 2026-11-10: releases only through a Release Helper plan. User: "I only release via a release plan with the release helper!!!!!!!"
- 2026-11-10: a new NuGet id needs nothing on nuget.org; the trusted publishing policy covers the workflow. User: "There is already a trust!!!"
- 2026-11-11: never weaken a step's check; `"next_after": "workflow"` on the C# step keeps the check and frees the next step.
- 2026-11-11: before a plan, run every test target of every crate changed since the last tag, one at a time. User: "this is the 20th time or so that this goes wrong"

## Was
- memory: release-helper.md, release-by-tags.md, runtime-release-status.md, go-sums-after-tag-check.md, nuget-trusted-publishing.md, nuget-build-workflow.md, nuget-packages.md, release-ci-builds.md, release-timing.md, release-test-targets.md, ready-before-release.md, generated-file-guards.md
- WINDOWS-TESTS.md release steps (since removed; only "After releases" checks remain, :61-68)
