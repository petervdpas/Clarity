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

A key or a token does not belong in the code, where everyone who reads the project sees it. Keep it in a [setting](../LANGUAGE.md#settings) and put it in with a slot, as above. A header named `Authorization`, or one whose name holds `key`, `token` or `secret`, with a plain text as its value is a warning on its line.

## Where code calls out

- **Handlers** and **commands** call out as they are.
- **An action** that calls out declares the effect `net`: `effects net`, beside any others. Its caller then knows the action reaches the network.
- **A function**, **an engine** and **a data event** cannot call out. A function is safe to call anywhere, an engine keeps to itself, and a data event runs inside the save, which must not wait on the network. Call out in the handler or action around them, and hand them what came back.

## On each stack

Every stack calls with its own client and gives the same replies: Rust with ureq, Go with net/http, C# with HttpClient, C and C++ with libcurl. A C or C++ project links libcurl only when its code calls out; see [Setting up the build tools](setup.tools.md) for its package.
