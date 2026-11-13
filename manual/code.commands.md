# Commands

How a project becomes a command-line program too: the `cli` block, commands with their arguments and options, help, what a command may do, and commands offered to AI agents.

## A program without a window

A project whose stack has the `cli` target (`stack go desktop cli`, `stack rust cli`; see [The project file](project.file.md)) is also a program without a window: commands typed in a terminal, over the same records, actions and settings as its windows. One `cli` block, named after the project, gives the program's description. Each command is declared in any module, with its statements below it as a function's are.

```clarity
cli Shop
    help @shopHelp

command customer add(name: text, email: text)
    help @addCustomerHelp
    option vip: bool, help @vipHelp
    option limit: money, default 1000, min 0, max 5000, help @limitHelp
    let c = new Customer
    c.name = name
    c.email = email
    c.creditLimit = limit
    save c
    print @customerAdded(name = c.name)

command customer remove(name: text)
    help @removeCustomerHelp
    let found = find c in Customer where c.name == name
    if count(found) == 0
        refuse @noSuchCustomer(name = name)
    if ask @removeIt(name = name) yes @remove no @keep
        delete found[1]
```

```text
$ shop customer add Ada ada@example.org --vip --limit 500
$ shop customer remove Ada
$ shop help customer add
```

The program is named after the project, in lower case with hyphens: `MyShop` is `my-shop`.

## Declarations

### `cli`

The program's description, once per project.

    cli Name
        help text

- `help` is a key, shown at the top of the program's help.
- "a project has one 'cli' block"; "a cli block holds its 'help', as in 'help @shopHelp'".

### `command`

A command: its words, its arguments, then its `help`, `option`, `allow` and `agent` lines, then its statements.

    command [group] name[(argument: type, ...)]
        help text
        option name: type[, default value][, min value][, max value][, required][, help text]
        allow when condition
        agent
        statements

| Line | What it says | Default |
|---|---|---|
| `help text` | What the command does, a key: its line in the help, and how an agent picks it. | none: a warning, an error for a command for agents |
| `option name: type, ...` | An option, given as `--name value` or `--name=value`. | none |
| `allow when condition` | Who may run it, reading `me`; needs sign-in in the project file. Written once. | anyone |
| `agent` | Offers the command to AI agents as a tool of the app's MCP service. Written once. | off |

- **Its words.** One word, or a group and a word: `command customer add` is typed `shop customer add`. A name in camelCase is typed with hyphens: `addAll` is `add-all`. A group is there when a command names it, and is no command of its own.
- **Arguments** are the parameters, given in order on the command line, all required. An argument is text, a number (`integer`, `decimal`, `money`), a date, a date and time, a duration or an enum, typed by its value's name. A `bool` is only an option, and a record is no argument: look it up with `find`.
- **Options** are `option` lines before the statements: text, a number, a date, a date and time, a duration, an enum or `bool`. A `bool` option is a flag (`--vip`). An option left out has its default, or the empty value of its type. `required` means it must be given, so it has no default. `min` and `max` bound a number. An option `dryRun` is typed `--dry-run`.
- The `help`, `option`, `allow` and `agent` lines come before the statements: "'option' goes before the statements".

The checker says: "there are two commands 'customer add'"; "'customer' is a group of commands, so it is no command of its own"; "command customer add has two arguments or options named 'name'"; "a yes or no is an option, not an argument: option vip: bool"; "an argument is text, a number, a date, a duration or an enum, not Customer; a command looks records up with find"; "an option is text, a number, a date, a duration, an enum or bool, not list<Order>; ..."; "command customer add has no help; write what it does: help @key" (a warning); "option vip of command customer add has no help; write what it does: help @key" (a warning); "the default of limit is text, but limit is money"; "limit is required, so it has no default"; "'min' and 'max' bound a number; name is text"; "unknown 'maximum'; an option has a default, min, max, required and help"; "'max' is written twice"; "'allow when' is written once; join the conditions with 'or'"; "'agent' is written once"; "command mcp: `mcp` is the program's own word, which serves its commands to AI agents; name the command otherwise". Arguments and options are camelCase.

## On the command line

- Numbers are written with a dot and no groups, dates as `2026-10-23`, dates and times as `2026-10-23T14:30`, durations as `3:45`, `1:02:03` or seconds. `--` ends the options.
- A wrong command line (an unknown command or option, a missing value, a value of the wrong kind or past `min` or `max`) writes what is wrong and the command's usage, and ends with exit code 2.
- `--help` anywhere, `help` before a command, or a group alone writes the help: the program's from the `cli` block, each command's and option's from its `help`.
- Clarity's own words of the command line (the help's headings, what it says of a wrong command line) are keys too, as the screen's are; a project with commands gives them their texts in its language files.

## What a command does

- **What it sees:** its arguments and options by name, `settings` (the ones the windows keep, read and set; a change is kept), and the functions, actions, engines, queries, `save`, `delete` and files a handler has.
- **No window:** `open`, `close`, `reload` of window data, `copy` and part properties are errors: "a command has no window: 'open' belongs in a handler".
- **`print`** writes a line to standard output: a text with slots, or a value. `print list columns a, b` writes the records of a list as a table: the fields' labels as headings, the columns lined up, numbers to the right. See [Statements](code.statements.md#print).
- **`message`** and **`toast`** write a line to standard error.
- **`exit n`** ends the program with exit code `n`, 0 to 255. A command that finishes ends with 0; a `refuse` writes its text to standard error and ends with 1. A command returns no value: "a command does not return a value; it ends with 'exit' and a code".

### `ask` in a command

In a command `ask` is a value: the answer typed on the terminal.

```clarity
command order close(number: text)
    help @closeOrderHelp
    let found = find o in Order where o.number == number
    let o = ask @which for Order from found
    let days = ask @howManyDays for integer default 3
    let kind = ask @which for CustomerKind
    if ask @sure yes @go no @stop
        o.status = Closed
        save o
```

- `ask @q yes @a no @b` gives a `bool`; `ask @q for type` gives a value of the type, with `default` and, for a record, `from` the list to choose from.
- It waits on the terminal; an empty answer takes the default. Without a terminal (piped input, a script) a question with a default takes it, and one without ends the program with exit code 2.
- In a command, `ask` is written where its answer is used, never with `then`: "in a command, ask gives its answer, as in 'let answer = ask ...' or 'if ask ...'". A record is asked from a list: "a command asks for a record from a list: 'ask ... for Customer from customers'".

## Commands for AI agents

The line `agent` offers a command to AI agents as a tool of the app's MCP service. A command for agents needs its `help`, which is how an agent picks it, and agents call it by its words joined with hyphens (`customer-add`).

```clarity
command customer add(name: text)
    help @addCustomerHelp
    agent
    let c = new Customer
    c.name = name
    save c
```

- Clarity works out what a command changes: it saves or deletes records, writes a file, sets a setting, or sends something out with an HTTP call that is no `get`, itself or through an action with those effects. The person can then be asked before the change.
- `<program> mcp` serves the commands for agents over MCP on standard input and output: an agent starts `shop mcp` as one of its MCP servers. A call runs the command as if it were typed, with the call's values as its arguments and options; what it prints is the answer, and a refusal, an exit code other than 0 or a wrong value is an error the agent reads. A command for agents needs no `cli` target: the windowed program serves `<program> mcp` itself. No command or group is named `mcp`.
- A question the command asks goes to the agent; when its client cannot answer, a yes or no is a no, a value takes its default, and without one the command stops.
- **While the app is open**, each call goes to it: the first read of a connection and every change ask the person in Clarity's dialog (Allow once, Allow for this session, Refuse), and the call runs on the open app's data. A change is one step: all of it is kept, or none when it is refused; the open window reads its data again, and a toast says what the agent did, with Undo, for 10 seconds or until the person closes it. Several apps run side by side, each answering only its own agents.
- **While the app is closed**, nobody is there to allow a change. The app's settings `agentReadsWhileClosed` and `agentChangesWhileClosed` allow reads and changes, else the call is refused in Clarity's words `agentClosed`: "{app} is not open; open it to allow this.". A setting the person never set is what the project file says (`agents while closed read change`), and off without the line.
- **The guidance for agents** is the project's `agent/guidance.md`: Markdown in four sections under fixed headings (`What the app is for`, `The app's words`, `How work flows`, `Rules the app does not check`), written in the project's language. Agents get it when they connect, followed by the app's data summed up. Agents write it a section at a time with Clarity's tools `guidance` and `set_guidance`. A name in backticks must be one of the app's (a command as typed, an entity, `Entity.field`, an enum or its value); the checker warns of one that is not, and of an app with commands for agents and no guidance.

The Agents part of the IDE, the person's settings and how an agent is set up are on [Apps for AI agents](project.agents.md).

See also [Statements](code.statements.md), [Functions and actions](code.functions.md) and [Signing in](project.signin.md) for `allow when`.
