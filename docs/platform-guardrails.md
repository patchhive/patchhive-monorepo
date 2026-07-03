# PatchHive Platform Guardrails

These rules are meant to protect PatchHive's reputation and keep future products aligned as the suite grows toward HiveCore.

## 1. Discovery Safety

Every product that discovers repositories or work autonomously should support the same three repo policy controls:

- `allowlist`
  Use only these repos when the list is non-empty.
- `denylist`
  Never discover, score, clone, patch, or open PRs against these repos.
- `opt_out`
  Strongest exclusion. Treat this as a durable "do not touch" signal across the whole PatchHive suite.

Precedence:

1. `opt_out`
2. `denylist`
3. `allowlist`
4. default autonomous discovery

Rules:

- If an `allowlist` is present, products should only work inside that allowlist after removing anything also present in `denylist` or `opt_out`.
- `opt_out` should be visible in product UIs and later centralized in HiveCore.
- Maintainer and operator opt-out signals should be durable and easy to inspect.
- Products should fail closed when policy data is ambiguous rather than acting aggressively.

## 2. Reputation And Output Limits

PatchHive's GitHub reputation compounds over time, which means bad output scales just as fast as good output.

Every autonomous write-capable product should eventually enforce hard limits for:

- maximum PRs opened per run
- maximum PRs opened per repo per 24 hours
- maximum PRs opened per owner or organization per 24 hours
- minimum confidence required before opening a PR
- cooldown after a PR is closed without merge
- cooldown after repeated failed attempts on the same repo
- provider and spend ceilings per run

Operational rules:

- Quality gates should be stricter than discovery gates.
- Products should prefer sending no PR over sending a weak PR.
- Rate limits should be enforced in the backend, not just the UI.
- Every product backend should layer `patchhive-product-core` API rate limiting so auth, mutating, and run-triggering routes share the same guardrail.
- HiveCore should inherit and coordinate these caps, not bypass them.

## 3. Shared API And Lifecycle Contracts

HiveCore should not have to normalize ten slightly different product APIs.

Every product backend should converge toward:

- standard request and response envelopes
- shared error object shape
- consistent `request_id`, `run_id`, `job_id`, and `event_id` formats
- shared async lifecycle states for long-running operations
- consistent webhook and SSE event semantics
- product-owned `/capabilities` and `/runs` endpoints that HiveCore can consume without private database access

See:

- [Product API Contract v1](/home/coemedia/Documents/code/patchhive/docs/product-api-contract-v1.md)
- [Suite runs and fix capabilities](/home/coemedia/Documents/code/patchhive/docs/suite-runs-and-fix-capabilities.md)

## 4. Git Credential Isolation

PatchHive products must not inherit a developer machine's ambient Git credentials.

Any product that runs `git clone`, `git fetch`, `git push`, or similar Git-over-HTTPS operations should:

- pass credentials explicitly for that operation when credentials are required
- use `GIT_ASKPASS` or an equivalent non-interactive credential path instead of relying on global Git credential helpers
- set `GIT_TERMINAL_PROMPT=0` so backend workers never hang waiting for a shell prompt
- run Git commands with `-c credential.helper=` when the operation must not use cached desktop credentials
- keep tokens out of command-line arguments, logs, PR bodies, SSE events, and saved run history
- report the authenticated identity mismatch clearly when a bot token cannot push to the expected fork

RepoReaper is the first write-capable product to enforce this because it opens outbound PRs. RefactorScout applies the same isolation to temporary read-only GitHub clones so local credentials are not accidentally consulted during public repo scans.

## 5. Implementation Notes

Current status:

- RepoReaper already has repo list controls and now supports `allowlist`, `denylist`, and `opt_out`.
- `patchhive-product-core` provides shared CORS, API-key auth helpers, startup helpers, and API rate limiting for all product backends.
- `@patchhive/ai-local` already uses explicit internal contracts for its Rust <-> Node adapter boundary.
- Cross-product HTTP contracts are still a platform task and should be treated as an early shared-infrastructure requirement, not a cleanup pass for later.
