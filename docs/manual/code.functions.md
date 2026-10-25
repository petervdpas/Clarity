# Functions and actions

How to write logic that is not tied to one window: functions and actions in a module, with parameters, results, contracts and effects.

## Functions and actions

Logic that more than one window needs goes into a module as a function or an action. Handlers, functions and actions call them by name, from their own module and from the modules it uses.

```clarity
function within_credit(order: Order) -> bool
    return order.total <= order.customer.credit_limit

action approve(order: Order)
    require order.status == Pending
    require within_credit(order)
    effects db.write
    order.status = Approved
    save order
```

- A function computes a value. It cannot save or delete, call actions or reach files. That makes it safe to call anywhere, also in conditions. It can read records with `find`; see [Queries](code.queries.md).
- An action may change data, and says so with `effects`.
- Neither one talks to the screen: `message`, `open`, `close` and part properties belong in [handlers](code.handlers.md).

Use a function when you compute something from what you get, such as a total or a check. Use an action when the work saves, deletes or touches files.

## Parameters and results

The signature is `name(param: type, ...)`, optionally followed by `-> type` for the result. Parameter and result types are `text`, `integer`, `decimal`, `money`, `bool`, `date`, `datetime`, an entity (one record), an enum and `list<Entity>`.

A parameter with a default is optional: `limit: integer = 0`. The default is a value written out, such as `0`, `""`, `true` or an enum's value, and the parameters with one come last. A call leaves them out, or names the ones it gives after the values in order:

```
action list_templates(status: text, limit: integer = 0, sort: text = "name")

list_templates("published")
list_templates("published", limit: 10)
list_templates(sort: "date", status: "draft")
```

Each parameter is given once, and every one without a default must be given. Handlers take their values in order, without names.

A function or action with a result must `return` a value on every path; the checker reports a path that does not. One without a result can leave early with a bare `return`.

## The body

The body is ordinary statements: `let` makes a local, `if`, `else if` and `else` choose, and `for` runs once per item of a list.

```clarity
function order_total(order: Order) -> money
    ensure result >= 0
    let total: money = 0
    for line in order.lines
        total = total + line.amount
    return total
```

## Contracts

- `require expr` checks the arguments before the body runs.
- `ensure expr` checks the result after it, which it calls `result`.

Both must be bool, and both come before the statements, together with `effects`. When one does not hold, the call fails with an error.

## Effects

`effects` lists what an action does, comma separated:

| Effect | Needed for |
|---|---|
| `db.write` | `save` and `delete` |
| `files.read` | listing files, `read text of` and `read bytes of` |
| `files.write` | `write ... to` |
| `db.read` | accepted; reading records needs no effect |

An action that calls another action must declare that action's effects too. A function cannot have effects.

## Refusing

`refuse "text"` stops the work with that text as the error. Inside a save or delete (see [Data events](code.data-events.md)) the save or delete does not happen. Text with slots works here: `refuse "{loan.album.title} is with another friend already"`.

## Writing them in Clarity

Open a module from Modules in the project tree. Its view lists the functions and actions under **Functions and actions**; pick one to edit it as text, checked while you type. **New function** and **New action** add one with a free name.

The full rules are in [the language reference](../LANGUAGE.md#9-functions-and-actions).
