# The language

A guide to the parts of Clarity you write code with: lines and names, values and types, text with slots, operators, statements and expressions.

## Lines and blocks

Clarity is line based. Each statement is one line, and a block is the lines indented below it. Indent with spaces, four per level; a tab is an error. `if`, `else` and `for` take an indented block.

Comments start with `//` and run to the end of the line, on a line of their own or after code. Blank lines are free. The designer keeps both when it rewrites a file. See [Lines, names and values](../LANGUAGE.md#1-lines-names-and-values).

## Names

A name starts with a letter or `_`, followed by letters, digits and `_`. Names are case sensitive. Entities, enums and windows are `PascalCase` by convention (`OrderLine`), and fields, data and functions `snake_case` (`vat_number`). The parts of a window and their handlers must be `camelCase`: letters and digits only, starting with a small letter (`newGame`, `saveClicked`).

Every part has a name, which its events and code go by. The designer names a part when it is placed (`button1`), and you can rename it in its properties. A part without a name is an error; **Name them** under Problems names every such part in the file. For a project written before names were required, `clarity spec names shop.clarity` names its parts and makes its part and handler names camelCase, in its modules and translations.

Words like `page` or `data` are keywords only where the grammar expects them, so a field may be called `data`. In expressions, `and`, `or`, `not` and `in` are reserved.

## Values and types

- Text in double quotes: `"Customer card"`. `\"` is a quote, `\\` a backslash, `\n` a new line.
- Numbers without quotes: `12`, `9.95`.
- `true` and `false`.
- Enum values, bare (`Business`) where the type says which enum is meant, else qualified (`CustomerKind.Business`).
- `none`, the empty value of a date, a date and time, a reference or a picture. Empty text is `""`.

Every value has a type that follows the data dictionary: `text`, `integer`, `decimal` (also money), `bool`, `date`, `datetime`, `duration`, an enum, a record, a list of records, `image`, and `file` for files of a file store. An integer is accepted where a decimal is expected; nothing else converts by itself. See [Types](../LANGUAGE.md#types).

## Text with slots

Text that people read names its values in braces instead of adding them together:

```clarity
message "Order {order.number} of {order.total} is approved"
approve_button.caption = "Approved on {order.ordered_on}"
```

A slot is a name or a member path. Each value shows as its field does: money with its currency, a date in the person's format, an enum by its label. A translator can move the slots; see [Languages](languages.md). Write `{{` and `}}` for a brace.

Text with slots belongs in `message`, `refuse`, and a part's `caption` or `error`. Text joined with `+` there is a warning, since it cannot be translated. See [Text with slots](../LANGUAGE.md#text-with-slots).

## Operators

From tightest to loosest:

| Operator | Meaning |
|---|---|
| `a.b` | member access |
| `-x` | negation |
| `*`, `/` | multiplication, division |
| `+`, `-` | addition, subtraction; `+` also joins text |
| `==`, `!=`, `<`, `<=`, `>`, `>=` | comparison |
| `x in [a, b]`, `a contains b` | one of the values; text holds text, ignoring case |
| `not` | negation of a bool |
| `and`, `or` | both, either |

Parentheses group.

## Statements

- `let total = 0` or `let total: money = 0` makes a local.
- `target = expr` sets a local, a field (`customer.credit_limit`) or a part property (`save.enabled`).
- `if`, `else if`, `else` and `for item in list` run blocks.
- `name(args)` calls a function or action; `handler_name()` runs another handler.
- `return` and `return expr` leave.
- `save`, `delete` and `refuse` work on records; see [Data events](code.data-events.md).
- `message`, `open`, `close` and `reload` are for [handlers](code.handlers.md).
- `write t to store "path"` writes a file; see [Queries](code.queries.md).

## Expressions

Besides values, names and operators: calls (`within_credit(order)`, `count(list)`, `round(total, 2)`, `today()`, `now()`; `round` keeps at most that many decimals, rounding half away from zero), `new Entity` for a new record with its defaults, `find` and `files` queries, and `read text of f`.

The full grammar is in [the language reference](../LANGUAGE.md#12-grammar-summary), with every statement under [Statements](../LANGUAGE.md#statements) and every expression under [Expressions](../LANGUAGE.md#expressions).
