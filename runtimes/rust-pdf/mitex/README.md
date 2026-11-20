# MiTeX's Typst definitions

The Typst side of [MiTeX](https://github.com/mitex-rs/mitex) (Apache-2.0, `LICENSE`), from its
commit f69eb43 (the `mitex` 0.2.4 crate): `mod.typ`, `prelude.typ` and `latex/standard.typ`, which
define what MiTeX's Typst math calls (`mitexsqrt`, `pmatrix` and the rest). Carried so a PDF with
math needs no Typst package from the internet.

Changed for Clarity, in `latex/standard.typ`: `xarrow` comes from `xarrow.typ` (written here)
instead of the `@preview/xarrow` package; `\KaTeX` is written `K a T E X`, as Typst 0.15 has no
`kai` symbol.
