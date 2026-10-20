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

Arguments arrive as a tuple in the declared order; results are anything serde writes. `on_open!` defines `on open`; `clock`, `random` and `log` are the services an engine declares with `uses`; `Bytes` crosses as base64.

## License

MIT
