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

There are five calls:

- `http get url`, to read something;
- `http post url with body`, to send something new;
- `http put url with body`, to replace something;
- `http patch url with body`, to change part of something;
- `http delete url`, to remove something.

The url and the body are text, so a slot puts a value in: `http get "https://api.example.com/orders/{order.number}"`. Only post, put and patch send a body.

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

A key or a token does not belong in the code, where everyone who reads the project sees it. Keep it in a [secret setting](data.settings.md#secret-settings), which the system's protected store keeps, and put it in with a slot, as above, or sign the source's calls with it: `auth bearer "setting:apiToken"`. A header named `Authorization`, or one whose name holds `key`, `token` or `secret`, with a plain text as its value is a warning on its line.

## JSON

Most services answer in JSON, and many want it sent. `json of` writes a record, a list or a value as JSON, and `read ... from json` reads records from it:

```clarity
action sendOrder(order: Order) -> integer
    effects net
    let reply = http post "https://api.example.com/orders" with json of order header "Content-Type": "application/json"
    return reply.status

code Orders
    handler loadClicked
        let reply = http get "https://api.example.com/orders"
        orders = read list<Order> from json reply.text at "data"
```

- `read Order from json text` reads one record, `read list<Order> from json text` a list; `at "data"` starts inside the JSON. `json of` writes a record nested as the service sent it.
- A service names its fields its own way, so names match also ignoring case, underscores, hyphens and spaces, and numbers and strings of numbers stand for each other. What the JSON leaves out keeps its default; JSON that does not fit stops the handler with a message that says where.
- An entity of a web API holds the records its service nests: `owner: User` holds one, `lines: list<Line>` several, read each into its own entity. Code reads them as any record and list: `order.owner.name`, `for line in order.lines`.

- An entity of a web API may hold records of the web API only, and a record never holds itself, through any number of records; a list may, as it can be empty. A stored entity refers to another with `ref User` instead, and keeps its lists in their own table. The checker says "field 'owner' of Order holds a User inside the record, which only an entity of a web API does; a stored entity refers to one with ref User", "field 'owner' of Order holds a User, which is stored; a record inside a web API's record is of the web API too" and "record Order holds itself: Order.parent holds an Order".

JSON is the first of Clarity's data formats; how every format reads and writes is on [Data formats](code.formats.md).

## A web API data source

A web API the application works with is a data source of the project, with its address and where its key is. The project tree lists them under **Web APIs**: **+ New web API** adds one by its name, and picking one opens it, with its address, its key and its OpenAPI description. In the project file it reads:

```
datasource forms rest "https://forms.example.com/api"
    auth bearer "env:FORMS_TOKEN"
```

`http get "/templates" from forms` calls a path of it: at its address, signed with its key. `query "status": status` adds a query parameter, encoded, as many as needed. `when` after a header or query pair leaves it out when its condition does not hold, which suits an optional value: `query "limit": "{limit}" when limit != 0`. A module with `source forms` holds the records the service keeps: they are not stored in a database, so no migration covers them, and `find`, `save` and `delete` do not reach them. Its actions read and send them.

## Every page of a list

A web API often hands out a long list a page at a time, each API in its own way. Say once how, under the data source:

```
datasource forms rest "https://forms.example.com/api"
    auth bearer "env:FORMS_TOKEN"
    paging page "page" size "per_page" 100 items "data"
```

- `page "page" size "per_page" 100` asks for page 1, 2, 3 and so on, 100 items each.
- `offset "offset" size "limit" 100` asks from item 0, 100, 200 and so on.
- `cursor "after" next "meta.next"` asks for the first page, then for the one each page names at `meta.next` in its JSON.
- `link` follows the address the service names as the next page in its `Link` header.
- `items "data"` says where a page's items are in its JSON, with dots for deeper names; leave it out when a page is a list.

The same line is set in the IDE: open the project settings, press **API...** on the data source and pick the kind under **Paging**. The form asks only for what that kind needs and writes the line for you.

Then `http get "/templates" from forms every page` reads page after page until the last, and its reply's text is one JSON list of the items of every page, which `read list<Template> from json reply.text` reads. A page the service does not answer well is the reply, so the code sees why. Only `get` reads every page, from a source that pages. Paging stops after 1000 pages, with status 0.

## An OpenAPI description

A service that describes itself with OpenAPI (Swagger) does not need these actions written by hand. Put its description, a YAML or JSON file, in the project's `apis` folder, or choose it in Clarity's file browser with **Choose file...** where the web API opens from the project tree (or **API...** beside the data source in the project settings). Then pick what the application uses:

```
datasource forms rest "https://forms.example.com/api"
    auth bearer "env:FORMS_TOKEN"
    openapi "forms.yaml"
    pick "listTemplates", "getTemplate", "createTemplate"
```

Clarity's agent can pick too: ask it to build something on the API, and it reads what the description offers and picks the operations it needs.

- Clarity writes the module `apis/forms.clarity` from the description: an action for each picked operation (`list_templates(status: text) -> list<Template>`), and an entity for each schema they take or give. `pick schema "Entry"` makes a schema an entity on its own.
- Only what is picked is written, so a service with many operations and schemas gives the application just what it uses. **API...** lists them all with a box to pick each.
- The module is Clarity's: do not edit it. Clarity writes it again when the description or the picks change, so a new revision of the description updates the entities and actions. A pick the new revision no longer has is a warning.
- Your own modules use it with `use forms`, and call its actions as any others: `templates = list_templates("published")`.
- An optional parameter of an operation is an optional parameter of its action, with the description's default or else `""`, `0` or `false`, and it is sent only when it differs: `list_templates(status: text, limit: integer = 0)`, called as `list_templates("published")` or `list_templates("published", limit: 10)`. A yes-or-no value goes to the service as `true` or `false`.
- A nested object is a record inside the record (`owner: User`), and a list of objects a list of them (`sections: list<Section>`); the schemas they name are written as entities too. One that would make a record hold itself, and an object without a schema of its own, are named in a comment above the field instead.
- On a source that pages, a picked list reads every page: its action calls `http get ... every page` and gives the whole list, and the parameters paging sends are not the action's.

## Where code calls out

- **Handlers** and **commands** call out as they are.
- **An action** that calls out declares the effect `net`: `effects net`, beside any others. Its caller then knows the action reaches the network.
- **A function**, **an engine** and **a data event** cannot call out. A function is safe to call anywhere, an engine keeps to itself, and a data event runs inside the save, which must not wait on the network. Call out in the handler or action around them, and hand them what came back.

## The `http` expression

```text
http get|post|put|patch|delete url [from source] [every page] [with body]
    [header "Name": value [when condition]]... [query "name": value [when condition]]...
```

All of it is written on one line. The url, the body, and each header's and query parameter's value are text; a `when` condition is a bool. The value is an `http reply`, whose `status` (integer), `ok` (bool) and `text` (text) are read.

The checker says: "an http call is get, post, put, patch or delete, not 'fetch'"; "the url of an http call is text, not integer"; "http post sends a body: write 'with' and the text"; "http get sends no body; post, put and patch do"; "the body of an http call is text, not Order" (write `json of order`); "the header 'Accept' is text, not integer"; "the query parameter 'limit' is text, not integer" (write `"{limit}"`); "the condition after 'when' is a bool, not text"; "the header 'Authorization' holds a key written in the code; keep it in a setting and put it in with a slot" (a warning); "unknown data source 'shp'; the project has ..."; "'main' is no web API: 'from' names a rest data source of the project"; "every page reads a web API data source: write 'from' and its name before it"; "write every page, not every item"; "an http reply has status, ok and text, not 'body'"; "action 'sendOrder' calls out, which needs 'effects net'"; "function 'fetch' cannot call out; an action can, with 'effects net'"; "an engine cannot call out: ..."; "a data event cannot call out: it runs inside the save's transaction, which must not wait on the network".

## On each stack

Every stack calls with its own client and gives the same replies: Rust with ureq, Go with net/http, C# with HttpClient, C and C++ with libcurl. A C or C++ project links libcurl only when its code calls out; see [Setting up the build tools](setup.tools.md) for its package.
