# Statements

Every statement of Clarity's code, one section each: what it does, how it is written, where it may stand and what the checker refuses.

## How statements are written

A handler, function, action, data event, command and an engine's function hold statements, one per line. `if`, `else` and `for` take an indented block below them. A statement starts with its word (`let`, `save`, `message`); a line that starts with no such word sets something (`total = 0`) or calls something (`approve(order)`).

A statement word followed by `.` or `=` is a name, not the statement: `save.enabled = false` sets the `enabled` of a part called `save`, while `save customer` stores a record. `let`, `if` and `for` are always statements.

Where a statement may stand depends on what holds it. A handler talks to the screen; a function computes and changes nothing; an action changes data and declares its effects; a data event is an action on its record; a command has no window but writes to the terminal; an engine keeps to itself. The sections below say where each statement goes, and the checker reports one in the wrong place, such as "'message' belongs in a handler, not in action approve" or "a command has no window: 'open' belongs in a handler".

```clarity
code CustomerForm
    handler saveClicked
        name.error = ""
        if customer.name == ""
            name.error = @nameIsRequired
            return
        save customer
        message @savedName(name = customer.name)
        close
```

## Statements

### `let`

Makes a local: a name for a value, inside the block it is written in.

```text
let name = expression
let name: type = expression
```

```clarity
function orderTotal(order: Order) -> money
    let total: money = 0
    for line in order.lines
        total = total + line.amount
    return total
```

- Without a type, the local has the type of the expression. With one, the value must fit it: `let total: duration = 0`, `let found: list<Project> = []`. An empty list needs its type.
- Everywhere: handlers, functions, actions, data events, commands, engines.
- The name is camelCase. The checker says "'total' is already defined" for a second local of that name in reach, "'total' is already a name in this window" for a name the window's data or parts have, and "cannot set money to text" for a value that does not fit the type.

### Setting a value

Sets a local, a field of a record, an item of a list, a part's property, a setting, or the splash screen's status.

```text
target = expression
```

```clarity
code OrderForm
    handler kindChanged
        vatNumber.enabled = customer.kind == Business
        customer.creditLimit = 1000
        squares[3] = 1
        settings.lastFolder = "exports"
```

- A target is a local, a data field (`customer.creditLimit`), a position of a list (`squares[3]`, counted from 1), a part property (`vatNumber.enabled`) or a setting (`settings.lastFolder`). Part properties are set in handlers only; see [Handlers](code.handlers.md#part-properties).
- A bound input is changed through its data: `customer.name = "Ada"` changes the text box bound to `customer.name`. Setting its `text` instead is refused: "textbox 'name' is bound to customer.name; use customer.name".
- A picture field is set to another picture, a file of a file store, or `none`; a file field to another record's file, a file of a store, or `none`.
- `splash.status = @loadingData` sets the line the splash screen shows while the app starts, from the app's `on start` handler.
- Everywhere, within what each place can reach.

The checker says "only a local, a data field or a part property can be set" for anything else; "cannot set integer to text"; "lives is a constant and cannot be set"; "CustomerKind is an enum and cannot be set"; "button 'save' is a part; set one of its properties"; "button 'save' has no property 'colour'"; and, for what a part only reports, such as "dial.x is read, not set: it is where the canvas was clicked".

### `if`

Runs a block when a condition holds; `else if` and `else` follow it at the same indent.

```text
if condition
    statements
else if condition
    statements
else
    statements
```

```clarity
function sizeOf(count: integer) -> text
    if count > 100
        return "large"
    else if count > 10
        return "medium"
    else
        return "small"
```

- The condition is `bool`: "the condition must be bool, not integer".
- Everywhere.
- `'else' without 'if'` for an `else` that follows no open `if`; "'if' needs an indented block" when nothing is indented below it.

### `for`

Runs its block once per item of a list, the item named by the word after `for`.

```text
for name in list
    statements
```

```clarity
action closeOld()
    effects db.write
    for order in find o in Order where o.status == Pending
        order.status = Closed
        save order

function squares(n: integer) -> list<integer>
    let all: list<integer> = []
    for i in 1 to n
        all = all + [i * i]
    return all
```

- The list is a list of records (`list<Order>`, a query, a window's list), a list of values (`list<integer>`, `[1, 2, 3]`), a list of files (`files f in photos`), or `1 to n`, which counts from 1 to n.
- Everywhere. In an engine written in Clarity, `to` stands only here, in a `for`.
- "'for' needs a list, not text"; "'for' needs an indented block". The item's name is camelCase.

### `return`

Leaves the handler, function, action or data event, with a result where it gives one.

```text
return
return expression
```

```clarity
function withinCredit(order: Order) -> bool
    return order.total <= order.customer.creditLimit
```

- A function or action with a result returns a value of its type on every path: "orderTotal must return money, not text"; "function 'orderTotal' does not return a value on every path".
- One without a result, a handler and a data event leave with `return` alone: "a handler does not return a value"; "approve does not return a value; use 'return' alone".
- A command ends with `exit` and a code: "a command does not return a value; it ends with 'exit' and a code".

### `message`

Shows a message: a toast that goes by itself, or a dialog the person closes.

```text
message expression
message info expression
message warning expression
message error expression
```

```clarity
code AlbumCard
    handler saveClicked
        save album
        message @savedName(name = album.title)

    handler checkClicked
        if album.copies == 0
            message warning @noCopiesLeft
```

- The default level is `info`: a toast at the top right that goes by itself after 5 seconds, or by its small x. A `warning` or `error` shows in Clarity's dialog and is closed with OK. A level word counts only when a value follows it, so `message warning` alone shows a local called `warning`.
- The text is a key, a text with slots or any single value. Text joined with `+` is a warning: "write this as one text with slots, like "Saved {customer.name}", so it can be translated and its values shown in the person's format".
- In handlers. In a command it writes a line to standard error.
- "a message is text or a value, not list<Order>"; "'message' belongs in a handler, not in function sizeOf".

See [Toasts](code.toasts.md) for toasts with their own lifespan, level, icon and corner.

### `toast`

Shows a toast the project declares, its text's slots filled by name.

```text
toast name
toast name(slot = expression, ...)
```

```clarity
code Card
    handler saveClicked
        save customer
        toast saved(name = customer.name)
```

- In handlers. In a command it writes its text to standard error, as `message` does.
- "there is no toast savd; the toasts are saved, imported"; "a slot shows a value, and customer is Customer".

The declaration, its lines and the IDE's form are on [Toasts](code.toasts.md).

### `ask`

Asks the person something in Clarity's dialog. In a handler the answer runs another handler; in a command `ask` is a value, the answer typed on the terminal.

```text
ask question [yes text] [no text] then handler[(values)]
ask question for type [default value] [from list] then handler[(values)]
ask Dialog [with name = value, ...] then handler[(values)]
```

```clarity
code AlbumCard
    handler deleteClicked
        ask @deleteAlbum(title = album.title) yes @delete no @keep then deleteConfirmed

    handler deleteConfirmed
        delete album

    handler lendClicked
        ask @lendTo for Friend from friends then lend(album)

    handler lend(what: Album, friend: Friend)
        what.copies = what.copies - 1
        save what
```

- **What is asked decides the dialog.** `yes @word no @word` asks yes or no: the handler runs on yes and takes no answer; without `yes` and `no` the buttons say OK and Cancel. `for text`, `integer`, `decimal`, `money`, `date`, `datetime` or `duration` asks for a typed value, and `default value` fills it first. `for` an enum offers its values with their labels. `for` an entity offers its records to choose from; `from list` offers the records of a list instead.
- **The answer** is the last value of the handler after `then`. Values written in the call come first (`then lend(album)`): that is how what the asking handler knew reaches the answering one, as it was when it asked.
- **A dialog of your own** answers too: `ask PickFriend then lend(album)` opens it as `open` does, and what it closes with (`close with grid.selected`) is the answer; `ask PickFriend with friends = mine then lend(album)` fills its data first. See [`close`](#close).
- **Cancelling** (no, Escape, a click beside the dialog, or a dialog closed without a value) runs nothing.
- **When.** The statements after `ask` still run; the dialog shows once the handler has finished, with whatever else it changed. A handler that asks twice asks one question after the other.
- **Where.** In handlers, with `then`; and in commands, as a value (see [Commands](code.commands.md#ask-in-a-command)). A function has no effects, and an action may run where nobody is at the screen.
- The question and the words of its buttons are keys, as every text people read.

The checker says: "handler 'deleteClicked' asks and closes its window: the answer would have nowhere to go"; "AlbumCard has no handler 'lnd' to run with the answer"; "handler 'deleteConfirmed' runs on yes and takes no answer: only the values written after 'then'"; "handler 'lend' takes 2 parameters, not 1: 0 written and the answer"; "the answer is Friend, but 'what', the last parameter of handler lend, is Album"; "ask for text, a number, a date, a duration, an enum or an entity, not bool"; "the default is text, but the answer is integer"; "'from' gives the records to choose from; integer is no entity"; "'from' gives the choices: a list<Friend>, not list<Album>"; "PickFriend is no dialog; ask a dialog that gives a value, or open PickFriend instead"; "in a command, ask gives its answer, as in 'let answer = ask ...' or 'if ask ...'".

### `open`

Opens a window over this one, its data filled by name.

```text
open Window
open Window with name = expression, ...
```

```clarity
code CustomerBrowse
    handler openClicked
        open CustomerForm with customer = grid.selected
```

- When the opened window closes, the window below runs its `on return` handler, the place to read a list again.
- In handlers.
- "unknown window 'CustomerFrom'"; "CustomerForm has no data 'client'"; "'customer' of CustomerForm is a Customer, not text".

### `close`

Closes this window; in a dialog that gives a value, `close with` closes it with that value for the handler that asked.

```text
close
close with expression
```

```clarity
code PickFriend
    handler picked
        close with grid.selected
```

```xml
<!-- loans/design/PickFriend.cxaml -->
<Dialog class="loans.PickFriend" gives="Friend">
  <Data name="friends" type="list&lt;Friend&gt;" />
  <Table name="grid" at="1,1" span="12" rows="6" bind="friends" columns="name" />
  <Button name="okButton" at="1,7" span="2" caption="@ok" onclick="picked" />
</Dialog>
```

- In handlers. `close with` only in a dialog that says what it gives (`gives="Friend"`), and every `close with` in it gives that type.
- "only a dialog closes with a value; AlbumCard is a window"; "PickFriend gives nothing; ... to close with a value"; "PickFriend gives Friend, not Album".

### `save`

Stores a record in its data source. A new record gets its id.

```text
save expression
save expression anyway
```

```clarity
action approve(order: Order)
    require order.status == Pending
    effects db.write
    order.status = Approved
    save order
```

- A record someone else saved since it was read is refused: the handler stops, the person sees Clarity's `savedSince` text (or the entity's own), and nothing is written. `reload order` then shows their change, and the person decides. `save order anyway` writes regardless of the version: overwriting on purpose, written out, never guessed. With sign-in, keep it to the people allowed with `allow when` on the action that does it. See [Entities](data.entities.md#a-save-never-overwrites-someone-elses-change).
- A save runs the entity's data events in one transaction with it; see [Handlers](code.handlers.md#data-events).
- In handlers, commands, data events and actions that declare `effects db.write`.
- "'save' needs a record, not text"; "Dot is a record, which is never stored"; "'save' is not allowed in a function; make it an action"; "action 'approve' saves but does not declare 'effects db.write'"; "saving order in its own 'before save' would run the event again, without end"; "Invoice is in data source erp, which is read-only; its records cannot be saved".

### `delete`

Removes a record from its data source.

```text
delete expression
```

```clarity
code AlbumCard
    handler deleteConfirmed
        delete album
        close
```

- A delete runs the entity's delete events in one transaction with it.
- In handlers, commands, data events and actions that declare `effects db.write`.
- The checker says what it says of `save`, with "delete" and "deleting".

### `reload`

Reads data again as it is stored now: the window's data, or a record held in a local or a parameter.

```text
reload name
```

```clarity
code CustomerBrowse
    handler windowReturned
        reload customers

action retry(order: Order)
    effects db.write
    reload order
    order.status = Pending
    save order
```

- `reload` of the window's data reads it again as when the window opened: a list loads all its records again, a record loads again by its id (a new record stays as it is). A table or list showing a list read again has no row selected afterwards, as when the list is given new rows (`orders = find ...`): they are other records. This form is for handlers.
- `reload` of a local or parameter that holds a record reads that record again, with its version, as after a refused save. Locals never share a name with the window's data, so the name says which form it is. This form stands in handlers, functions, actions, data events and commands.
- "CustomerBrowse has no data 'clients'"; "'shown' is a plain value; only records and lists are read again"; "'reload' reads a record again, and total is integer"; "Dot is a record, which is never stored, so 'reload' has nothing to read"; "unknown name 'orders': outside a window's handlers, reload reads a record variable again".

### `refuse`

Stops with this text as the error: a save or delete it is part of does not happen.

```text
refuse expression
```

```clarity
code Order
    before delete
        if order.status != Pending
            refuse @onlyPendingDeletable
```

- In a handler the person sees the text as an error message, and the window keeps working. In a data event the save or delete is not kept. In an engine it is the engine's refusal, in its words. In a command it writes the text to standard error and ends the program with exit code 1.
- Text with slots works here: `refuse "{loan.album.title} is with another friend already"`.
- Everywhere.
- "'refuse' needs text or a value, not list<Order>".

### `write`

Writes text, or bytes as they are, as a file of a file store, at a path inside it, making its folders.

```text
write expression to store path
```

```clarity
action exportCollection()
    effects files.write
    let lines = ""
    for album in find a in Album order by a.title
        lines = lines + album.title + "\n"
    write lines to exports "collection.txt"
```

- The store is a file store of the project file (`filestore exports "data/exports"`). A path that would leave the store is refused when it runs. In `write`, the `to` belongs to `write`, not to a range.
- In handlers, commands, data events and actions with `effects files.write`.
- "what to write must be text or bytes, not Album"; "the path to write it to must be text, not integer"; "function 'export' cannot reach files; an action can, with 'effects files.write'"; "action 'exportCollection' writes a file, which needs 'effects files.write'".

See [Queries](code.queries.md) for listing and reading files.

### `print`

Writes a line to standard output, in a command; `columns` writes a list of records as a table.

```text
print expression
print list columns field, field
```

```clarity
command customer list
    help @listCustomersHelp
    let found = find c in Customer order by c.name
    print found columns name, email
    print @customerCount(count = count(found))
```

- `print` writes a text, with slots, or a value. With `columns`, the fields' labels are the headings, the columns are lined up and numbers stand to the right.
- In commands.
- "'print' belongs in a command, not in handler saveClicked"; "print a list of records as a table, with its columns: 'print found columns name'"; "Customer has no field 'mail'"; "'columns' prints a list of records as a table, not text".

### `exit`

Ends the command-line program with this exit code.

```text
exit expression
```

```clarity
command check
    help @checkHelp
    let late = find o in Order where o.status == Pending
    if count(late) > 0
        exit 3
```

- The code is an integer from 0 to 255. A command that finishes ends with 0; a `refuse` ends with 1; a wrong command line ends with 2.
- In commands.
- "'exit' belongs in a command, not in action approve"; "the exit code is an integer, not text"; "the exit code is 0 to 255".

### `tell`

Tells the window something, with values: from an engine beside the screen, or from a view to the window it shows in.

```text
tell name(values)
```

```clarity
engine Sky
    beside the screen
    tells finished(time: integer)
    keep elapsed: integer = 0

    function clarity_tick(ms: integer)
        elapsed = elapsed + ms
        if elapsed > 10000
            tell finished(elapsed)
```

- In an engine beside the screen, the tell goes into the answer of the call it is in, and the canvas that runs the engine runs the handler wired to it; see [Engines](code.engines.md#an-engine-beside-the-screen).
- In a view's code, it tells the window the view shows in: the view's file says what it tells (`<Tells message="toResearch(title: text)" />`) and the window wires it on the view (`<View name="ChaptersPart" ontoResearch="openResearch" />`).
- "'tell' is for an engine beside the screen or a view, which tell the window they show in"; "engine Sky tells no 'done'"; "view ChaptersPart tells no 'chosen'; say it on the view's page: tells chosen"; "tell finished gives (time: integer), not 2 values".

### `copy`

Puts a text (or a value, as it shows in the person's language and format) on the system's clipboard once the handler has run, for the person to paste anywhere.

```text
copy expression
```

```clarity
code OrderCard
    handler copyNumberClicked
        copy order.number
        message @copied
```

- Of several copies in one handler, the last counts. On the web the page uses the clipboard of the browser or desktop window it runs in. Pair it with a message so the person knows.
- In handlers.
- "copy takes text or a value, not list<Order>".

### Calls

A call on its own line runs a function, an action, another handler, or tells a part something.

```text
function(values)
action(values, name: value)
handler(values)
part.hear(values)
canvas.shape(values)
```

```clarity
code CustomerBrowse
    handler windowReturned
        filterChanged()

    handler filterChanged
        customers = find c in Customer where c.name contains nameFilter.text order by c.name

    handler restartClicked
        arena.restart(2)
```

- **Functions and actions** of the module and the modules it uses; see [Functions and actions](code.functions.md). Values go in order, then any by name (`list_templates("published", limit: 10)`).
- **Another handler** of the same window or app, with values for its parameters, in order: `digit(3)`. It gives no value: "handler 'digit' does not return a value".
- **A part's hear**: a custom part's (`arena.restart(2)`; see [Custom parts](design.custom-parts.md)), an engine beside the screen through its canvas (`sky.start(2)`), a text box's or memo's `insert(text)` (see [Handlers](code.handlers.md#part-properties)), a tree's `reload()`, and a window's content showing another view: `main.swap(OrderEditor)`.
- **A shape** on a canvas, in its draw handler: `dial.line(0, 0, 10, 10, "text")`; see [Parts](design.parts.md).
- "unknown function 'aprove'"; "handler 'digit' takes 1 value, not 2"; "part Arena hears no 'reset'"; "Arena.restart takes 1 value, not 0"; "main shows no view 'X'; list it in main as view X".

See also [Expressions](code.expressions.md) for what goes on the right of these statements, and [Handlers](code.handlers.md) for what a handler sees.
