# Connections for the agent

How the agent reaches its model through a connection in Clarity's vault: the fields of an Anthropic connection, keeping the key in the vault, and choosing a connection in the panel.

## What a connection is

A connection is a named set of fields, kept by Clarity in `connections.json` beside its [vault](vault.md). Each field is either plain text or a pointer to a secret in the vault, by its category and key. The agent reads its model, address and key from a connection, so the key never sits in a project file.

You make and change connections on the Vault tab: click **Connections** in the ribbon, then **New connection**. Give it a name and add a row per field with **Add field**. In the **From** column, choose **Plain text** for a value you type, or **Vault** to name a secret by category and key.

## An Anthropic connection

The agent looks for a connection named `anthropic` unless you pick another. Its fields, spelled exactly like this:

- `kind`: `anthropic`. A connection named `anthropic` may leave it out.
- `model`: the model to use, as Anthropic names it. Required.
- `apikey`: your Anthropic API key. Required. Take it from the vault.
- `baseUrl`: the address of the API. Leave it out for `https://api.anthropic.com`.
- `maxTokens`: the most the model may write in one answer. Leave it out for 16384.
- `cacheTtl`: how long the model keeps the conversation in its cache between requests: `5m` (leave it out for this) or `1h`. Within a turn the steps follow each other, so five minutes is enough and cheaper. Choose `1h` when you often take longer than five minutes between your requests: without it, the next request sends the whole conversation again at the full price. An hour's cache costs more to write, so check the token line under the conversation to see which pays off for you.
- `turnBudget`: the most one turn may spend, in tokens at full price: what the model reads without its cache, writes to its cache, and writes as its answer (what it reads from its cache is not counted, as that costs a tenth or less). Write it like `200k`, `250000` or `1.5M`; leave it out for 200k, or write `none` for no budget.

A typical connection, as the Vault tab shows it:

```text
Field       From         Value
kind        Plain text   anthropic
model       Plain text   (the model's name)
apikey      Vault        Anthropic / apikey
```

Store the key itself as a secret first: on the Vault tab, under **Secrets**, add a category such as `Anthropic` with **Categories**, then **Add** a secret with that category, the key `apikey` and your API key as its value.

## Values from the vault

When a turn starts, Clarity reads the connection's fields. A plain field is used as written. A vault field needs the vault to be unlocked; when it is locked, Clarity asks for the master password and then starts the turn. Plain fields are stored readable in `connections.json`, so keep the key in the vault.

If a field is missing or wrong, the panel says which: a connection without an `apikey`, without a `model`, or of another kind.

## Choosing a connection

The box at the top of the Agent panel lists the connection named `anthropic` and every connection that has a `kind` field. Pick one there; Clarity remembers your choice. You cannot change it while a turn runs. See [The agent](ai.agent.md).

## Other kinds

The `kind` field says which kind of model a connection reaches, so other providers can be added later without changing how connections work. Today only Anthropic works: a connection of any other kind is refused with a message that says so.
