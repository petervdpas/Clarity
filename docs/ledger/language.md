# Language

Status: partial. The Clarity language: lexer, parser, checker, declarations, statements, expressions, operators, built-in functions, effects and access.

Progress: 75%

## Code
- Spec: `crates/clarity-spec/src/lexer.rs`, `parser.rs`, `model.rs`, `check.rs`, `check/`, `names.rs`, `typing.rs`; code in `crates/clarity-spec/src/code/` (`parse.rs`, `mod.rs`, `check.rs`, `print.rs`); loading order in `crates/clarity-spec/src/lib.rs:100-160`
- Generators: `crates/clarity-gen/src/{rust,go,c,cpp,csharp}/logic.rs`; named values and defaults filled in by `crates/clarity-gen/src/calls.rs`; `consts.rs`
- Runtimes: R `runtimes/rust/src/texts.rs`, `lib.rs`; Go `runtimes/go/clarity/values.go`, `texts.go`, `lists.go`, `round.go`; C `runtimes/c/clarity/values.h`, `decimal.c`, `code.c`; C++ `runtimes/cpp/clarity/values.hpp`, `decimal.cpp`, `code.cpp`, `texts.cpp`; C# `runtimes/csharp/Clarity.Runtime/Lists.cs`, `Numbers.cs`, `Program/Members.cs`
- IDE: `app/src/lib/SourceEditor.svelte` (checks while typing)
- Docs: manual `code.modules.md`, `code.statements.md`, `code.expressions.md`, `code.handlers.md`, `code.functions.md`

## Tests
- `conformance/texts.json`, `lists.json`, `rounding.json`: run by R, Go, C, C++, C#
- `conformance/calls.json`: run by `crates/clarity-spec/tests` and `runtimes/rust-ui/tests/screen`; C# parses calls (`Program/Members.cs`) with no test against it
- `crates/clarity-spec/tests/`: code.rs, parse.rs, modules.rs, consts.rs, records.rs, defaults.rs, events.rs, lifecycle.rs, asks.rs, drawing.rs, whole_division.rs, names.rs, typing.rs, values.rs, value_lists.rs
- `crates/clarity-gen/tests/`: whole_division.rs, consts.rs, typing_build.rs

## Open
- `while`, `break`, `continue`, `match`: NOT BUILT. Not in `code/parse.rs` or the lexer.
- Date arithmetic and date members (`d + 1`, `d.year`): NOT BUILT. Dates only compare: arithmetic takes durations and numbers only (`code/check.rs:5007-5025`, `is_numeric` :863).
- `computed` with a formula: PARTIAL. The word only skips the required check (`crates/clarity-gen/src/lib.rs:271`).
- `old(expr)`, `sum(list.field)`, `all(...)`, `any(...)`: NOT BUILT (DESIGN 5.4).
- `require`/`ensure` as generated tests: NOT BUILT. Run-time checks only (`crates/clarity-gen/src/rust/logic.rs:202-221`).
- A context object exposing only declared effects: NOT BUILT. Effects are checked by the checker only.
- `encoded(text)`, `plainText(html)`: NOT BUILT (APP-CAPABILITIES-PLAN.md:43).
- C generator gaps: PARTIAL. List literals other than records or values (`c/logic.rs:1905`), a position in them (:1929), window data `list<bool>`/`list<Enum>` (:437), a list of values given to a window (:1624), `json of` a `list<bool>` (:2030).
- `files f in store where` with a condition that can fail: PARTIAL, missing on Go (`go/logic.rs:701`).
- Reading a stored `list<Entity>`: see [entities](entities.md). `progress n of total`: see [engines](engines.md).

## Conflicts
- Manual: quoted texts that must be keys (`refuse "text"` in code.functions.md:134, code.handlers.md:237, code.statements.md:358); see [docs](docs.md).
- C# ordered comparisons: a plain `(l < r)` (`csharp/logic.rs:1277`); `none < d` is false on C#, true elsewhere (`go/logic.rs:1917`, `runtimes/c/clarity/values.h:37`); text `<` does not compile on C#.
- Decimal division: Go, C, C++ keep 16 decimals (`runtimes/c/clarity/decimal.h:13`), R and C# 28 digits; division by zero is a value error on C and C++, a panic or exception on R, Go, C#.
- Integer overflow in `+ - *`: R panics (debug) or wraps, Go and C# wrap, C and C++ undefined. Decimal range: C/C++ about 38 digits, R/C# 28, Go unlimited.
- Empty values in order (decision needed): table order puts empty last (`runtimes/rust/src/tables.rs:3`); code comparisons put none first on R, Go, C, C++.

## Decisions
- 2026-10-16: handlers in `code` blocks run in the backend; `=` assigns, `==` compares (memory: clarity-direction.md).
- 2026-10-16: action bodies are Clarity (DESIGN 5.10), replacing bodies in the target language (5.3, 6.4).
- 2026-10-25: defaults and named values, chosen by the user out of three (DESIGN 5.24).
- 2026-11-05: records in app code (window data, parameters, locals) on all five stacks.
- 2026-11-06: every lower-case name is camelCase; types PascalCase. User: "camelCase everywhere"
- 2026-11-08: `div` and `mod` on integers, `/` gives a decimal (DESIGN 5.38).
- 2026-11-09: windows and views live in `.cxaml`; a page in a module is an error. User: "1, cxaml is the source, make it!!! and the .cxaml is used to derive the clarity code!!!!"
- 2026-11-10: braces instead of indentation are wanted, not planned. User: "hmmm... I hate the identation convention!"

## Was
- DESIGN 5 (intro), 5.1, 5.3, 5.4, 5.9, 5.10, 5.24, 5.38; 5.12 "Records in app code (built 2026-11-05)"; 20 question 2 (syntax)
- Roadmap S1, L1, L2, E1, X2 (duration, none, today, checked)
- CXAML-SOURCE-PLAN.md (the decision that pages live in `.cxaml`)
- IMPLEMENTED.md section 3 (Loading, Declarations, Pages and events, Statements, Expressions, Built-in functions, Effects and access, What the generators still refuse); UNIMPLEMENTED.md section 1; CONFLICTING.md section 2 (comparisons, division, overflow, range), section 1 (empty values), section 6
- memory: camel-case-names.md, braces-not-indentation.md
