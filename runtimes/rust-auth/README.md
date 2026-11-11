# Clarity.Auth

The sign-in of the programs Clarity generates (docs/APP-CAPABILITIES-PLAN.md, step 4): one catalog of strategies, one implementation of each, used by every stack.

- `catalog()` lists the strategies: the fields of the project's entity of people each one matches (by role), the inputs it takes, the targets it works on. The checker, the designer and the generators read it.
- `find(strategy, inputs)` says which field to look the person up by and the value; `check(strategy, inputs, stored)` says whether the person found is signed in. A runtime does only that, for every strategy.
- `firstAccount(strategy, inputs)` gives the fields (by role) of the first account, when the entity of people has no record yet and the strategy can make one (password, account).
- `hash(method, text)` keeps a secret as a hash (Argon2id for passwords, SHA-256 for tokens Clarity makes); `newToken()` makes one.
- Failed tries per key within a window, and sessions (a token naming a record until it expires or is closed), kept in the library for the whole program.

With the feature `openid`, the `openid` module is OpenID Connect's authorization code flow with PKCE for a desktop program: a provider by its endpoints (`Provider::new`, or `Provider::entra` for Microsoft Entra ID), its sign-in page's address, the code coming back to a loopback address, and the tokens redeemed and renewed. The IDE signs in to Microsoft Foundry with it too.

Rust programs use this crate; Go, C, C++ and C# programs use its C table (`clarity-auth-ffi`, `clarity_auth_api()`), which takes an operation's name and its arguments as JSON (`call`).
