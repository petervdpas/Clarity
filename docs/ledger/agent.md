# Clarity's agent

Status: partial. The IDE's own agent: a chat panel that designs the open project with the shared tools, on Anthropic or Microsoft Foundry models.

Progress: 60%

## Code
- Spec: none
- Generators: none
- Runtimes: none (Foundry sign-in uses Clarity.Auth's `openid`, `runtimes/rust-auth/src/openid.rs`; see [signin](signin.md))
- Clarity.UI: none
- Agent core: `crates/clarity-agent/src/lib.rs` (loop: tool calls until none, Stop, 40 steps or the token budget, default 200k, :216-278), `anthropic.rs` (Messages API, streaming, prompt caching 5m or 1h; `Auth` key or token, :45), `foundry.rs` (connection fields, `FOUNDRY_SCOPE`, :13)
- IDE: `app/src-tauri/src/agent.rs` (`Runs` Ask/Allow/Never :52, build tool left out with Never :108, :212; other kinds refused :393), `app/src-tauri/src/foundry.rs`, `app/src-tauri/src/acp.rs` (`agent_sign_in` :358); `app/src/lib/AgentPanel.svelte`, `agentStore.svelte.ts`, `agent.ts`
- Tools it calls: [mcp-tools](mcp-tools.md); connections kept in the [vault](vault.md)
- Docs: manual/ai.agent.md, manual/ai.connections.md

## Tests
- `crates/clarity-agent/tests/agent.rs`, `anthropic.rs`, `foundry.rs`
- `app/src-tauri/src/agent.rs:536-732` (`only_the_build_tool_executes_and_it_asks_first` :536)
- `app/src/lib/agent.test.ts`
- conformance: none

## Open
- Local models (`ollama`, then `openai`-compatible) as a `Model` beside `anthropic.rs`: NOT BUILT. Other kinds refused at `app/src-tauri/src/agent.rs:393`.
- Smaller tool set for small models (structural and reading tools, shorter instructions, one manual section in the prompt): NOT BUILT. Waits with local models.
- Foundry's OpenAI models (`/openai/v1/`): NOT BUILT.
- A Clarity Entra app registration so `clientId` is optional: NOT BUILT. Today the company registers a public client with redirect `http://localhost`.
- Foundry sign-in against a real tenant (which API permission the scope needs, admin consent): UNTRIED.
- AI proposals inside a project (layout, data model, slow handlers, duplicated logic), patterns across projects, run-time measurements: NOT BUILT. The only proposals are for diagrams ([diagrams](diagrams.md)).
- The panel's words from translations: NOT BUILT (IDE words are English in the Svelte files; see [ide-shell](ide-shell.md)).

## Conflicts
- Whether the agent executes (needs decision): CLAUDE.md:21 "Clarity's own agent never executes anything; anything that runs asks first" against `app/src-tauri/src/agent.rs:52, 108, 212` (with Runs = Allow it builds and runs unasked), manual/ai.agent.md:38, README.md:145.

## Decisions
- 2026-10-21: the agent runs nothing by itself; anything that executes asks in the panel first. User: "the ai can never execute anything and should ask permission when executing"
- 2026-11-10: small-model tool set and local models wait for a stronger machine. User: "3 not yet... couldnt get the pi give any performance, need a better machine first!"
- 2026-11-11: Clarity's agent on the company's Foundry (Claude behind the Anthropic Messages API, Bearer token, scope `https://ai.azure.com/.default`), signed in with a Microsoft work account. User: "But I need the ms business copilot way too!!!!", then "it was foundry!!"
- 2026-11-11: one OpenID implementation for apps and the IDE: Foundry's sign-in lives in clarity-auth `openid` (`Provider::entra`), never a second copy. User: "maybe sso (entra id) brings something useful to this route?!"
- 2026-11-11: `apikey` (step 1) came with step 2: a `foundry` connection with a key uses `x-api-key`, without one it signs in.

## Was
- DESIGN 1.2 (the "AI" paragraph, DESIGN.md:82-89)
- DESIGN roadmap A2
- AGENT-TOOLS-PLAN.md steps 2 and 3
- FOUNDRY-PLAN.md steps 1, 2 and 3, "One sign-in for apps and the IDE", open questions
- memory: foundry-plan.md, agent-never-executes.md
