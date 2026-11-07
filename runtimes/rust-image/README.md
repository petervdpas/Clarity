# Clarity.Image

The picture functions of the programs Clarity generates: `resize`, `crop`, `rotate`, `flip`, `blend` and `adjust`, plus a picture's size. Each takes pictures as their encoded bytes (PNG, JPEG or WebP) and gives a new picture as PNG, with the same pixels on every stack (`conformance/picture-functions.json`).

Rust programs use this crate; Go, C, C++ and C# programs use its C table (`clarity-image-ffi`, `clarity_image_api()`), which Clarity carries with them.
