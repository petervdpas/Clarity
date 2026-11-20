// An arrow stretched over its text, as `\xrightarrow{f}` writes it: what
// MiTeX takes from the xarrow package, made here so no package is fetched.
#let xarrow(sym: sym.arrow.r, body) = math.attach(math.stretch(sym, size: 100% + 0.6em), t: body)
