# Picture functions

Status: partial. Built-in functions on pictures (resize, crop, rotate, flip, blend, adjust, pictureWidth, pictureHeight) on Clarity.Image, giving PNG, on every stack.

Progress: 75%

## Code
- Spec: `crates/clarity-spec/src/code/check.rs:2118` (the functions typed)
- Generators: `crates/clarity-gen/src/lib.rs:345` (`uses_picture_functions`), :1009 (`pictures_without_library`); rust/logic.rs, csharp/logic.rs, cpp/logic.rs, go/mod.rs (generated `pictures.go`), c/mod.rs, cpp/mod.rs, `crates/clarity-gen/src/clarity_ui.rs`
- Library: `runtimes/rust-image` (src/lib.rs, src/ops.rs; the `image` crate, PNG, JPEG, WebP in); C table `runtimes/rust-image-ffi` (`clarity_image_api()`)
- Runtimes: R `runtimes/rust/src/picture_functions.rs` (clarity-runtime feature `pictures`); Go generated `pictures.go` linking `clarity_image`; C `runtimes/c/pictures/pictures.c`; C++ `runtimes/cpp/pictures/pictures.cpp`; C# NuGet `Clarity.Image` (`runtimes/csharp/Clarity.Image/PictureFunctions.cs`)
- Clarity.UI: the painting canvas uses the crate directly (see [canvas-toolbox](canvas-toolbox.md))
- IDE: none of its own
- Docs: manual `code.expressions.md` (Picture functions), `data.pictures-files.md`

## Tests
- `conformance/picture-functions.json`: run by `runtimes/rust-image/tests/pictures.rs` only; skipped by every stack's wrapper (R, Go, C, C++, C#)
- `crates/clarity-spec/tests/picture_functions.rs`; `crates/clarity-gen/tests/picture_functions_build.rs` (ignored, real tools); `runtimes/csharp/Clarity.Ui.Native.Tests/PictureFunctionsTests.cs`
- Guard: `CLARITY_WRITE_PICTURES=1 cargo test -p clarity-image --test pictures` (CLAUDE.md "Generated-file guards")

## Open
- Picture functions in Go, C and C++ programs that are web-only or cli: PARTIAL. Refused (`crates/clarity-gen/src/lib.rs:1009`, `pictures_without_library`); manual `code.expressions.md:203` states the desktop-only limit. See Design "Every target".
- An example that uses the picture functions: NOT BUILT; see Design "An example".
- Picture functions in engines; a video frame as a picture: NOT BUILT; see [video-camera](video-camera.md) Design "Pictures in engines".

## Design

### Every target
- A Go, C or C++ program that is web-only (`stack go web`) or a command-line program (`cli`) runs the picture functions too, on Clarity.Image's C table (`runtimes/rust-image-ffi`), linked without Clarity.UI's library. Today the generator refuses it.
- Every stack's tests load `conformance/picture-functions.json` and give the same pixels (R, Go, C, C++, C#), not only the crate.

### An example
- An example whose code calls the picture functions (resize, crop, rotate, flip, blend, adjust) on its records' pictures, on a stack no picture example uses yet, built in the example builds of the checks.
- Which example, and its stack, is an open question.

## Conflicts
- The stack wrappers do not load `conformance/picture-functions.json`; only the crate checks the pixels. See Design "Every target".

## Decisions
- 2026-11-07: Clarity.Image is a library of its own (crate, C table, NuGet), carried only by a program whose code calls a picture function (user's decision).
- 2026-11-07: pure Rust `image` crate, no system libraries, so it works offline and alike on every OS.
- 2026-11-07: one picture type, what an `image` field holds; every function gives PNG.
- 2026-11-07: all stacks give the same pixels for the same call, pinned by `conformance/picture-functions.json`.

## Was
- IMAGE-PLAN.md "Decisions" (Clarity.Image, the image crate, one picture type), "The language" ("only when used" for web-only and cli: now Design "Every target"), "Clarity.Image", Order steps 1, 2, 6 (the example of step 2: now Design "An example")
- VIDEO-PLAN.md step 4 (image tools half), "Pictures" (image tools)
