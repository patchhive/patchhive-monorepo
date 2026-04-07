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
- HiveCore should inherit and coordinate these caps, not bypass them.

## 3. Shared API And Lifecycle Contracts

HiveCore should not have to normalize ten slightly different product APIs.

Every product backend should converge toward:

- standard request and response envelopes
- shared error object shape
- consistent `request_id`, `run_id`, `job_id`, and `event_id` formats
- shared async lifecycle states for long-running operations
- consistent webhook and SSE event semantics

See:

- [Product API Contract v1](/home/coemedia/Documents/code/patchhive/docs/product-api-contract-v1.md)

## 4. Implementation Notes

Current status:

- RepoReaper already has repo list controls and now supports `allowlist`, `denylist`, and `opt_out`.
- `@patchhive/ai-local` already uses explicit internal contracts for its Rust <-> Node adapter boundary.
- Cross-product HTTP contracts are still a platform task and should be treated as an early shared-infrastructure requirement, not a cleanup pass for later.
