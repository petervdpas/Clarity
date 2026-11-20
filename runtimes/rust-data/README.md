# Clarity.Data

The runtime data of the programs Clarity generates: records, their lists and their values, decided once for every stack (Rust, Go, C, C++, C#). Each stack keeps its own types and reads and writes its own database, files and screen; Clarity.Data decides the shape.

- `tree`: one value tree, numbers kept exactly as written and object names in their order.
- `kind`: what a program reads or writes, as Clarity's generator describes it once for every stack.
- `numbers`: exact decimal numbers as text.
- `fit`: a value fitted to its kind.
- `normal`: a value in its one plain form.

Clarity.Formats, the data formats (JSON, CSV, XML, YAML) of the same programs, is built on it.
