# clarity-engine

Write engines for [Clarity](https://github.com/petervdpas/Clarity) in Rust. An engine is processing compiled once to WebAssembly, which every stack Clarity generates runs the same way; see the manual page Engines.

```rust
clarity_engine::interface!();

clarity_engine::function!(add, |(a, b): (i64, i64)| Ok(a + b));
```

Build it as a `cdylib` for `wasm32-unknown-unknown`:

```shell
cargo build --release --target wasm32-unknown-unknown
```

Arguments arrive as a tuple in the declared order; results are anything serde writes. `on_open!` defines `on open`; `clock`, `random`, `log` and `language` are the services an engine declares with `uses`; `Bytes` crosses as base64.

The texts an engine gives people to read are written as the project writes its own, and listed in its declaration with `says`, so Clarity translates them. `language()` gives the language the person picked, such as `nl`, for an engine that sorts or formats by it.

## License

MIT
