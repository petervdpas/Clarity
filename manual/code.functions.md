# Functions and actions

How to write logic that is not tied to one window: functions and actions in a module, with parameters, results, contracts, effects and who may run them.

## Functions and actions

Logic that more than one window needs goes into a module as a function or an action. Handlers, functions, actions, data events and commands call them by name, from their own module and from the modules it uses.

```clarity
function withinCredit(order: Order) -> bool
    return order.total <= order.customer.creditLimit

function orderTotal(order: Order) -> money
    ensure result >= 0
    let total: money = 0
    for line in order.lines
        total = total + line.amount
    return total

action approve(order: Order)
    require order.status == Pending
    require withinCredit(order)
    effects db.write
    order.status = Approved
    save order
```

- A **function** computes a value. It has no effects: it cannot save or delete, call actions, reach files or the network, or read `me`. That makes it safe to call anywhere, also in conditions. It can read records with `find` (see [Queries](code.queries.md)) and read and write [data formats](code.formats.md).
- An **action** may change data and reach the world, and says so with `effects`.
- Neither one talks to the screen: `message`, `open`, `close`, `ask` and part properties belong in [handlers](code.handlers.md).

Use a function when you compute something from what you get, such as a total or a check. Use an action when the work saves, deletes, touches files or calls a web API.

## Writing them in the IDE

Open a module from Modules in the project tree. Its view lists the functions and actions under **Functions and actions**; pick one to edit it as text, checked while you type. **New function** and **New action** add one with a free name.

## Declarations

### `function`

    function name(parameter: type [= default], ...) [-> type]
        require condition
        ensure condition
        statements

```clarity
function greeting(name: text, times: integer = 1) -> text
    require times > 0
    return "{name} x{times}"
```

### `action`

    action name(parameter: type [= default], ...) [-> type]
        require condition
        ensure condition
        effects effect, ...
        allow when condition
        statements

```clarity
action sendOrder(order: Order) -> integer
    effects net
    let reply = http post "https://api.example.com/orders" with json of order header "Content-Type": "application/json"
    return reply.status
```

The `require`, `ensure`, `effects` and `allow` lines come before the statements, in any order: "'effects' goes before the statements".

## Parameters and results

The signature is `name(parameter: type, ...)`, optionally followed by `-> type` for the result. A parameter or result is any type code knows: `text`, `integer`, `decimal`, `money`, `bool`, `date`, `datetime`, `duration`, an entity (one record), a [record](code.modules.md#record), an enum, `file`, `bytes`, and lists of them (`list<Order>`, `list<Node>`, `list<text>`); see [Expressions](code.expressions.md#types). Parameter names are camelCase.

A parameter with a default is optional: `limit: integer = 0`. The default is a value written out: a number, a text, `true` or `false`, `none` or an enum's value. The parameters with one come after those without. A call leaves them out, or gives any of them by name after the values in order:

```clarity
action listTemplates(status: text, limit: integer = 0, sort: text = "name") -> integer
    effects net
    let reply = http get "/templates" from forms query "status": status query "limit": "{limit}" when limit != 0
    return reply.status

code Templates
    handler loadClicked
        listTemplates("published")
        listTemplates("published", limit: 10)
        listTemplates(sort: "date", status: "draft")
```

Each parameter is given once, and every one without a default is given. Handlers, built-in functions and an engine's functions take their values in order, without names.

A function or action with a result returns a value on every path. One without a result can leave early with a bare `return`.

The checker says: "parameter 'sort' needs a default, as 'limit' before it has one"; "the default of 'limit' is a fixed value, such as 0, "", true or an enum's value"; "the default of 'limit' is text, not integer"; "listTemplates has no parameter 'count'"; "listTemplates gets 'limit' twice"; "listTemplates needs a value for 'status', which has no default"; "listTemplates takes at most 3 arguments, not 4"; "argument 'status' of listTemplates is a text, not integer"; "a value without a name cannot follow a named one: write name: value"; "function 'orderTotal' does not return a value on every path"; "orderTotal must return money, not text"; "function 'withinCredit' is declared twice"; "unknown function 'withinCredt'".

## Contracts

- `require condition` checks the arguments before the body runs.
- `ensure condition` checks the result after it, which it calls `result`.

Both are bool. When one does not hold, the call fails with an error. "a condition must be bool, not integer"; "'ensure' needs a result; approve returns nothing".

## Effects

`effects` lists what an action does, comma separated:

| Effect | Needed for |
|---|---|
| `db.write` | `save` and `delete` |
| `files.read` | listing files, `read text of` and `read bytes of` |
| `files.write` | `write ... to` |
| `net` | calling a web API with `http` |
| `db.read` | accepted; reading records needs no effect |

An action that calls another action declares that action's effects too. Handlers and commands do all of these as they are, without `effects`. A data event has every effect but `net`.

The checker says: "function 'withinCredit' cannot have effects; make it an action"; "unknown effect 'db.delete'; the effects are db.read, db.write, files.read, files.write and net"; "action 'approve' saves but does not declare 'effects db.write'"; "action 'export' writes a file, which needs 'effects files.write'"; "action 'sendOrder' calls out, which needs 'effects net'"; "action 'closeAll' calls 'approve', which needs 'effects db.write'"; "'save' is not allowed in a function; make it an action"; "function 'export' cannot reach files; an action can, with 'effects files.write'"; "function 'fetch' cannot call out; an action can, with 'effects net'"; "function 'withinCredit' cannot call action 'approve'".

## Who may run an action

`allow when condition` on an action says who may run it, reading `me`, the record of who signed in. It is written once; join conditions with `or`. The project file needs sign-in (`signin Member`); see [Signing in](project.signin.md).

```clarity
action overwrite(order: Order)
    effects db.write
    allow when me.role == Admin
    save order anyway
```

"only an action has 'allow when', not withinCredit"; "'allow when' is written once; join the conditions with 'or'"; "'allow when' says who may: write signin and the entity of people in the project file, like: signin Member"; "expected 'when' and a condition, as in 'allow when me.role == Admin'".

## Refusing

`refuse "text"` stops the work with that text as the error. Inside a save or delete (see [Data events](code.handlers.md#data-events)) the save or delete does not happen. Text with slots works here: `refuse "{loan.album.title} is with another friend already"`. See [Statements](code.statements.md#refuse).

See also [Statements](code.statements.md), [Expressions](code.expressions.md) and [Engines](code.engines.md).
