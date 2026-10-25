# Calling web APIs

How code calls a web API with `http get` and the others, what a reply holds, and where keys go.

## A call and its reply

`http` calls a web address and gives its reply, which code then reads:

```clarity
code Status
    handler refreshClicked
        let reply = http get "https://status.example.com/api/now"
        if reply.ok
            statusText = reply.text
        else
            message "The status service answered {reply.status}"
```

There are four calls:

- `http get url`, to read something;
- `http post url with body`, to send something new;
- `http put url with body`, to replace something;
- `http delete url`, to remove something.

The url and the body are text, so a slot puts a value in: `http get "https://api.example.com/orders/{order.number}"`. Only post and put send a body.

The reply has three details, which code reads and does not set:

- `status`: what the service answered, such as 200 or 404, or 0 when no answer came;
- `ok`: true for a status from 200 to 299;
- `text`: what the service sent back, or why no answer came.

## When no answer comes

A call never stops the code. A server that is not there, a wrong address, or no answer within 30 seconds gives a reply with status 0, and its text says what happened, such as `cannot reach status.example.com: ...`. The code decides what that means: show a message, `refuse`, or try another way. Only `http` and `https` addresses are called; anything else is a reply with status 0 that says so.

The window waits while a handler calls out, as it waits for anything a handler does.

## Headers and keys

`header "Name": value` sends a header with the call, as many as you need:

```clarity
let reply = http post "https://api.example.com/orders" with order.summary header "Content-Type": "text/plain" header "Authorization": "Bearer {settings.apiToken}"
```

A key or a token does not belong in the code, where everyone who reads the project sees it. Keep it in a [secret setting](../LANGUAGE.md#secret-settings), which the system's protected store keeps, and put it in with a slot, as above, or sign the source's calls with it: `auth bearer "setting:apiToken"`. A header named `Authorization`, or one whose name holds `key`, `token` or `secret`, with a plain text as its value is a warning on its line.

## JSON

Most services answer in JSON, and many want it sent. `json of` writes a record, a list or a value as JSON, and `read ... from json` reads records from it:

```clarity
action send_order(order: Order) -> integer
    effects net
    let reply = http post "https://api.example.com/orders" with json of order header "Content-Type": "application/json"
    return reply.status

code Orders
    handler loadClicked
        let reply = http get "https://api.example.com/orders"
        orders = read list<Order> from json reply.text at "data"
```

- `read Order from json text` reads one record, `read list<Order> from json text` a list. `at "data"` starts inside the JSON, at the names given with dots between: `at "result.items"`.
- A service names its fields its own way, so names match also ignoring case, underscores and hyphens: `creditLimit` fills `credit_limit`. A number fills a money or decimal field, and a string of a number fills an integer. A date and time fills a date with its date. An enum value matches in any case.
- What the JSON leaves out keeps its default, and what the entity does not have is left aside. The id is never read, nor references, pictures, files and lists of stored records. The records are new: `save` adds them.
- An entity of a web API holds the records its service nests: `owner: User` holds one, `lines: list<Line>` several. They are read from the nested object and list, each fitted to its own entity, and `json of` writes them nested. Code reads them as any record and list: `order.owner.name`, `for line in order.lines`.
- JSON that does not fit stops the handler with a message that says where, such as `field 'lines' of Order is an integer, not "three"`, as a refused save does.

## A web API data source

A web API the application works with is a data source of the project, with its address and where its key is. The project tree lists them under **Web APIs**: **+ New web API** adds one by its name, and picking one opens it, with its address, its key and its OpenAPI description. In the project file it reads:

```
datasource forms rest "https://forms.example.com/api"
    auth bearer "env:FORMS_TOKEN"
```

`http get "/templates" from forms` calls a path of it: at its address, signed with its key. `query "status": status` adds a query parameter, encoded, as many as needed. `when` after a header or query pair leaves it out when its condition does not hold, which suits an optional value: `query "limit": "{limit}" when limit != 0`. A module with `source forms` holds the records the service keeps: they are not stored in a database, so no migration covers them, and `find`, `save` and `delete` do not reach them. Its actions read and send them.

## An OpenAPI description

A service that describes itself with OpenAPI (Swagger) does not need these actions written by hand. Put its description, a YAML or JSON file, in the project's `apis` folder, or choose it with **Choose file...** where the web API opens from the project tree (or **API...** beside the data source in the project settings). Then pick what the application uses:

```
datasource forms rest "https://forms.example.com/api"
    auth bearer "env:FORMS_TOKEN"
    openapi "forms.yaml"
    pick "listTemplates", "getTemplate", "createTemplate"
```

- Clarity writes the module `apis/forms.clarity` from the description: an action for each picked operation (`list_templates(status: text) -> list<Template>`), and an entity for each schema they take or give. `pick schema "Entry"` makes a schema an entity on its own.
- Only what is picked is written, so a service with many operations and schemas gives the application just what it uses. **API...** lists them all with a box to pick each.
- The module is Clarity's: do not edit it. Clarity writes it again when the description or the picks change, so a new revision of the description updates the entities and actions. A pick the new revision no longer has is a warning.
- Your own modules use it with `use forms`, and call its actions as any others: `templates = list_templates("published")`.
- An optional parameter of an operation is an optional parameter of its action, with the description's default or else `""`, `0` or `false`, and it is sent only when it differs: `list_templates(status: text, limit: integer = 0)`, called as `list_templates("published")` or `list_templates("published", limit: 10)`. A yes-or-no value goes to the service as `true` or `false`.
- A nested object is a record inside the record (`owner: User`), and a list of objects a list of them (`sections: list<Section>`); the schemas they name are written as entities too. One that would make a record hold itself, and an object without a schema of its own, are named in a comment above the field instead.
- Not yet read: patch operations.

## Where code calls out

- **Handlers** and **commands** call out as they are.
- **An action** that calls out declares the effect `net`: `effects net`, beside any others. Its caller then knows the action reaches the network.
- **A function**, **an engine** and **a data event** cannot call out. A function is safe to call anywhere, an engine keeps to itself, and a data event runs inside the save, which must not wait on the network. Call out in the handler or action around them, and hand them what came back.

## On each stack

Every stack calls with its own client and gives the same replies: Rust with ureq, Go with net/http, C# with HttpClient, C and C++ with libcurl. A C or C++ project links libcurl only when its code calls out; see [Setting up the build tools](setup.tools.md) for its package.
