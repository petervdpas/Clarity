# Picture functions

Status: partial. Built-in functions on pictures (resize, crop, rotate, flip, blend, adjust, pictureWidth, pictureHeight) on Clarity.Image, giving PNG, on every stack.

## Code
- Spec: `crates/clarity-spec/src/code/check.rs:2109` (the functions typed)
- Generators: `crates/clarity-gen/src/lib.rs:344` (`uses_picture_functions`), :972 (`pictures_without_library`); rust/logic.rs, csharp/logic.rs, cpp/logic.rs, go/mod.rs (generated `pictures.go`), c/mod.rs, cpp/mod.rs, `crates/clarity-gen/src/clarity_ui.rs`
- Library: `runtimes/rust-image` (src/lib.rs, src/ops.rs; the `image` crate, PNG, JPEG, WebP in); C table `runtimes/rust-image-ffi` (`clarity_image_api()`)
- Runtimes: R `runtimes/rust/src/picture_functions.rs` (clarity-runtime feature `pictures`); Go generated `pictures.go` linking `clarity_image`; C `runtimes/c/pictures/pictures.c`; C++ `runtimes/cpp/pictures/pictures.cpp`; C# NuGet `Clarity.Image` (`runtimes/csharp/Clarity.Image/PictureFunctions.cs`)
- Clarity.UI: the painting canvas uses the crate directly (see [canvas-toolbox](canvas-toolbox.md))
- IDE: none of its own
- Docs: LANGUAGE.md "Files" (:2791); manual `data.pictures-files.md`

## Tests
- `conformance/picture-functions.json`: run by `runtimes/rust-image/tests/pictures.rs` only; skipped by every stack's wrapper (R, Go, C, C++, C#)
- `crates/clarity-spec/tests/picture_functions.rs`; `crates/clarity-gen/tests/picture_functions_build.rs` (ignored, real tools); `runtimes/csharp/Clarity.Ui.Native.Tests/PictureFunctionsTests.cs`
- Guard: `CLARITY_WRITE_PICTURES=1 cargo test -p clarity-image --test pictures` (CLAUDE.md "Generated-file guards")

## Open
- Picture functions in Go, C and C++ programs that are web-only or cli: PARTIAL. Refused (`crates/clarity-gen/src/lib.rs:971`); promised IMAGE-PLAN.md:28, LANGUAGE.md:2786.
- An example that uses the picture functions: NOT BUILT (IMAGE-PLAN.md:90).
- Picture functions in engines; a video frame as a picture: NOT BUILT (VIDEO-PLAN.md:99; see [engines](engines.md), [video-camera](video-camera.md)).

## Conflicts
- The stack wrappers do not load `conformance/picture-functions.json` (CONFLICTING.md section 3); only the crate checks the pixels.
- IMAGE-PLAN.md has no status line though the functions are built (CONFLICTING.md section 9).

## Decisions
- 2026-11-07: Clarity.Image is a library of its own (crate, C table, NuGet), carried only by a program whose code calls a picture function (user's decision, IMAGE-PLAN.md).
- 2026-11-07: pure Rust `image` crate, no system libraries, so it works offline and alike on every OS.
- 2026-11-07: one picture type, what an `image` field holds; every function gives PNG.
- 2026-11-07: all stacks give the same pixels for the same call, pinned by `conformance/picture-functions.json`.

## Was
- IMAGE-PLAN.md "Decisions" (Clarity.Image, the image crate, one picture type), "The language", "Clarity.Image", Order steps 1, 2, 6
- VIDEO-PLAN.md step 4 (image tools half), "Pictures" (image tools)
