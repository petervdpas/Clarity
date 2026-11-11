# Your company's models on Microsoft Foundry

A plan, written 2026-11-11 at the user's request. Step 2 is being built first, at the user's request.

## Goal

The user's words: "the acp parts or clarity... it says github, but can copilot also use organisational set copilot?", then "But I need the ms business copilot way too!!!!", and on seeing that the Microsoft sign-in they remembered came from Foundry: "it was foundry!!". Then: "yes, write the plan and start step 2".

So Clarity's agent works on the company's own models in Microsoft Foundry (the company's Azure), signed in with the person's Microsoft work account through the browser, without an API key typed into Clarity.

## What was found

- The Microsoft sign-in the user saw came from the "Foundry MCP" server that GitHub Copilot loads (Copilot's logs on 2026-11-07, 09 and 11: "MCP authorization was cancelled"). That sign-in only gives Copilot Foundry's tools; the model stays GitHub Copilot's.
- Microsoft's own business Copilot (Microsoft 365 Copilot) cannot work on a project: its Chat API answers in text only and takes no tools. Its coding agent, Copilot Code, runs only in Microsoft's own app (Windows, Mac, web), in preview.
- Foundry hosts Claude behind Anthropic's own Messages API, the API Clarity's agent already speaks (Microsoft Learn, "Deploy and use Claude models in Microsoft Foundry"):
  - the address is `https://<resource name>.services.ai.azure.com/anthropic`, and requests go to its `/v1/messages`;
  - the `model` is the deployment's name, which "can be different from the model ID";
  - signing in with Microsoft Entra ID sends `Authorization: Bearer <token>`, with the scope `https://ai.azure.com/.default`; an API key goes in `x-api-key`, as with Anthropic;
  - with Entra ID the person needs the role Cognitive Services User on the resource.
- Microsoft's Rust crate `azure_identity` (1.0) has no browser sign-in for a person: it has service credentials and runs `az` or `azd`. So Clarity signs in itself with OAuth 2.0's authorization code flow with PKCE, the flow Microsoft documents for desktop apps, through a page on `http://localhost` that only Clarity listens on.

## Steps

1. **A `foundry` connection with an API key.** Kind `foundry` on the Vault tab, with `resource` (or a whole `baseUrl`), `deployment` and `apikey`; Clarity's own agent, as with Anthropic.
2. **Sign in with Microsoft.** The same connection without an `apikey` signs in with the person's work account. The panel shows **Sign in**, the browser opens Microsoft's sign-in page, and Clarity keeps what lets it stay signed in (the refresh token) in the vault, never in a plain file. Each turn gets a fresh token from it; an hour's token is renewed by itself.
3. **Foundry's OpenAI models.** Later: models behind Foundry's OpenAI API (`/openai/v1/`), a second way of speaking for Clarity's agent.

Every step comes with the manual, the Vault tab's fields, the IDE's texts and tests.

## How step 2 is built

- The sign-in is Clarity.Auth's `openid` flow (`runtimes/rust-auth/src/openid.rs`, feature `openid`): the sign-in page's address with PKCE, reading the answer on `localhost`, trading the code for tokens and renewing them, with `Provider::entra` for Microsoft. Generated apps sign in with the same code. Its tests run against a small local server, with no network.
- `clarity-agent`'s `foundry` reads a connection's fields and holds `FOUNDRY_SCOPE`.
- The Anthropic client takes either a key (`x-api-key`) or a token source (`Authorization: Bearer`), so Foundry and Anthropic share one client.
- The IDE: a `foundry` connection without a key reads its refresh token from the vault (category `Microsoft`, the connection's name as the key). Without one the turn ends with **Sign in** in the panel, as for Codex and Copilot. **Sign in** listens on a free port of `localhost`, opens the browser, waits for the answer (five minutes at most) and stores the refresh token. When Microsoft hands a new refresh token on renewal, the vault gets it.
- Fields of the connection: `resource` or `baseUrl`, `deployment`, `tenant` (the company's tenant id or domain; leave it out for `organizations`), `clientId` (the app registration it signs in through), and the existing `maxTokens`, `cacheTtl`, `turnBudget`.

## One sign-in for apps and the IDE

The user, on hearing that another session builds sign-in for generated apps: "maybe sso (entra id) brings something useful to this route?!". It does: docs/APP-CAPABILITIES-PLAN.md plans the `openid` strategy (OpenID Connect: Entra ID, Google Workspace, Okta, Keycloak) in `clarity-auth` (runtimes/rust-auth), and on the desktop it is the same flow as this sign-in: "the system browser and a loopback address with PKCE". So there is one implementation: the flow was first written in `clarity-agent`, then moved into `clarity-auth`'s `openid` (2026-11-11, with its tests), and the IDE's Foundry sign-in uses it from there.

## Open questions

- Clarity has no app registration with Microsoft of its own yet, so `clientId` is required: the company (or the person) registers a public client app in Entra ID with the redirect `http://localhost`. A Clarity registration that works for every company would make `clientId` optional; it needs a Microsoft account to own it and each company's admin to allow it.
- Which API permission that app registration needs for the scope `https://ai.azure.com/.default`, and whether a company admin has to consent; to check against a real tenant.
- `apikey` (step 1) came along with step 2: it is the same connection and a few lines in the same place.

## Sources

- Microsoft Learn, "Deploy and use Claude models in Microsoft Foundry": https://learn.microsoft.com/azure/ai-foundry/foundry-models/how-to/use-foundry-models-claude
- Microsoft Learn, "Configure authentication with Microsoft Entra ID" (Foundry): https://learn.microsoft.com/en-us/azure/ai-foundry/foundry-models/how-to/configure-entra-id
- Microsoft identity platform, authorization code flow: https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-auth-code-flow
- Microsoft 365 Copilot APIs: https://learn.microsoft.com/en-us/microsoft-365/copilot/extensibility/copilot-apis-overview
- Copilot Code requirements: https://learn.microsoft.com/en-us/copilot/code/code-requirements
- `azure_identity` 1.0.0 source (no interactive credential): https://crates.io/crates/azure_identity
