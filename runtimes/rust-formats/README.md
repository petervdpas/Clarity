# Clarity.Formats

The data formats of the programs Clarity generates (docs/FORMATS-PLAN.md): one catalog, one implementation of every format, used by every stack.

- `read(format, input, options, kind)` parses the input and fits it to the record (or list, or value) the program wants: names match ignoring case, underscores, hyphens and spaces; what is left out keeps its default; a value that does not fit gives a message that says where. The fitted result is JSON, which each stack decodes with its own record codec.
- `write(format, value, options, kind)` writes a value, given as the JSON its stack's codec writes, in the format: for JSON itself normalised, so every stack writes the same bytes.
- `catalog()` lists the formats and their options; the checker, the designer and the agent read it.

JSON is the first entry. Rust programs use this crate; Go, C, C++ and C# programs use its C table (`clarity-formats-ffi`, `clarity_formats_api()`).
