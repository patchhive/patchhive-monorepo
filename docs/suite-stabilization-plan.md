# Suite Stabilization Plan

PatchHive is already documented heavily, but the next risk is not missing
vision. The risk is transitional drift: v1/v2 UI overlap, gateway/in-process
backend overlap, exported repo mirrors, product-local conventions, and growing
autonomous write behavior all moving at once.

This plan turns the recurring review recommendations into concrete suite
stabilization lanes.

## Stabilization Principles

- The monorepo stays the source of truth.
- CI should catch drift before humans rediscover it visually.
- Gateway mode is a bridge, not the final architecture.
- HiveCore should reduce product boilerplate, not become another product shell.
- Shared code should move into shared crates/packages as soon as the second
  product needs it.
- Product autonomy should increase only when evidence, logs, budgets, and
  approval gates increase with it.

## 1. CI Enforcement

Current state:

- `rust-check.yml` runs `scripts/check-rust-packages.sh`.
- `suite-drift.yml` runs `scripts/check-suite-drift.sh`.
- Package publish workflows exist for shared frontend packages.

Needed gates:

- suite drift must stay required on pull requests
- Rust formatting/checks should include the shared backend and every product
  that can build in the monorepo
- add `cargo clippy` for shared crates, `services/patchhive-backend`, and active
  product backends
- add frontend package install/build checks for `packages/*` and active
  `frontend-v2` apps
- add export validation so a product/service export can be generated cleanly
  before release
- add dependency/security hygiene checks that report findings without blocking
  early alpha work until the noise level is understood
- make CI read the product registry wherever possible so product additions do
  not require updating multiple hardcoded product lists

Acceptance criteria:

- A pull request cannot merge if product registry, docs, package metadata, or
  exported surfaces drift from the monorepo contract.
- A pull request cannot merge if shared crates or active in-process products fail
  Rust checks.
- CI output tells the operator which product or export target drifted.
- Standalone mirrors can be regenerated from the monorepo without manual cleanup.

## 2. HiveCore And Unified Backend

HiveCore should become the operator cockpit for one suite backend, not a
workaround for twelve permanent product backends.

Near-term priorities:

- finish moving low-risk read products into `services/patchhive-backend`
- keep product frontends pointed at the same suite API shape whether a product
  is in-process or still proxied
- move shared operator auth, product registry, schedules, scope policy, run
  index, and event log into the backend
- keep HiveCore focused on health, launch, shared defaults, policy, run history,
  and orchestration
- make product gateway mode visibly temporary by tracking which routes are
  proxied and which routes are native in-process

Acceptance criteria:

- A standalone product repo can run the shared backend image with
  `PATCHHIVE_PRODUCTS=<product>`.
- HiveCore can show every enabled product's health, capabilities, runs,
  schedules, credentials posture, and contract drift from the suite backend.
- Product frontends do not need product service tokens in the browser.
- Product-local backend deletion only happens after the unified backend owns the
  same behavior and standalone export path.

See [Suite backend direction](suite-backend-direction.md).

## 3. Shared Extraction Rules

PatchHive should extract more aggressively, but only at real seams.

Extract now or next:

- auth/bootstrap/rate-limit behavior in `patchhive-product-core`
- run envelopes, run status vocabulary, and run history summaries
- suite schedule records and dispatch contracts
- scope policy: allowlist, denylist, opt-out, saved scopes
- RepoReaper's Squad substrate once a second product needs AI roles
- GitHub token validation and typed permission errors
- per-run log artifacts and phase/event persistence
- shared GitHub token scope descriptions and startup validation messages
- shared SQLite migration helpers, backup guidance, and "database busy"
  operator-facing errors

Keep product-owned:

- scoring heuristics
- product-specific evidence
- report language
- safety/risk decisions
- issue, PR, dependency, security, release, and refactor ranking logic

Acceptance criteria:

- Any seam repeated in two products has an extraction issue or an active shared
  implementation.
- New products use shared starter/auth/run contracts by default.
- Product-specific code can still be exported without carrying unrelated suite
  behavior.
- Repeated startup/auth/DB/run lifecycle code does not appear in a third product.

See [Shared Squad architecture](shared-squad-architecture.md).

## 4. Integration Tests

Unit checks are not enough for the suite direction. PatchHive needs repeatable
fixtures for the full loop.

Required fixture coverage:

- SignalHive finds repo maintenance signals
- RepoMemory ingests context and emits prompt/memory evidence
- TrustGate blocks or warns on risky diffs
- ReviewBee reads review pressure
- MergeKeeper decides ready/hold/block from PR state
- ReleaseSentry makes ready/watch/hold release calls
- DepTriage and VulnTriage handle missing permissions clearly
- RepoReaper can dry-run, hold, and open a draft PR in a controlled repository
- product startup checks report missing tokens, degraded permissions, database
  health, and sandbox/test posture consistently

Acceptance criteria:

- The sandbox repo can exercise the read-only products without hitting unrelated
  public repos.
- Write-capable tests use draft PRs, low budgets, and cleanup instructions.
- The suite can run a signal -> trust/memory -> action path and leave a readable
  evidence trail.
- CI can run a reduced fixture path without depending on random public repos.

## 5. Frontend Type Safety

PatchHive frontends are intentionally lightweight JavaScript right now. That is
still fine for speed, but the risk grows as shared v2 components carry more
state, run history, auth, provider setup, and dashboards.

Near-term path:

- add JSDoc typedefs for shared `ui-v2` primitives and product API payloads
- document expected prop shapes for shared radar, queue, shell, and card
  components
- prefer typed API adapter modules over ad hoc response access inside panels
- revisit TypeScript once v2 surfaces stop moving quickly
- keep v1/v2 parity checklists current until old frontends are retired
- standardize empty, loading, auth-failed, degraded, and permission-failed states
  in shared UI instead of patching them product by product

Acceptance criteria:

- Shared frontend primitives have documented prop contracts.
- Product API adapters normalize backend payloads before UI panels render them.
- A future TypeScript migration has clear boundaries if it becomes worth the
  churn.
- A product can be visually checked from a shared v2 checklist without
  rediscovering missing sidebars, phantom buttons, stale selections, or bad
  history labels.

See [UI v2 migration](ui-v2-migration.md).

## 6. Performance And Cost Budgets

Autonomy gets expensive through AI calls, GitHub API calls, cloning, Docker
tests, and long retries. Budgeting should be visible and enforceable before
suite runs grow.

Required controls:

- per-run AI budget
- lifetime cost reporting
- per-product GitHub request caps
- clone and test concurrency caps
- dry-run mode for every write-capable product
- global suite caps for scheduled and autonomous runs
- clear budget-exceeded status in run history
- sandbox resource limits for CPU, memory, disk, process count, and timeout
- provider/model failure guidance for empty completions, auth failures, 429s,
  and model list discovery errors

Acceptance criteria:

- Every mutating or AI-backed action advertises cost and concurrency posture.
- HiveCore can show active budget usage and blocked/held runs.
- Scheduled runs respect the same budget and policy caps as manual runs.
- A runaway clone/test/provider loop fails closed with a durable reason instead
  of silently burning budget or host resources.

## 7. Observability

As products move into the unified backend, "look at stdout" stops scaling.
HiveCore needs structured visibility into runs and failures.

Required observability:

- run phase events persisted in SQLite
- product/action/run IDs on every significant log line
- per-run log artifacts for long-running products
- gateway/in-process routing visible in backend events
- GitHub/provider permission failures preserved with enough detail to fix scopes
- clear phase labels for judge, reaper, apply, smith, tests, fork, branch, PR,
  publish, and webhook work
- outbound GitHub comments, check runs, and pull requests linked back from run
  history
- route provenance in events so operators can see whether a response came from
  the suite backend, gateway proxy, or product backend

Acceptance criteria:

- A failed run explains where it failed without needing terminal logs.
- HiveCore can show suite events, product events, and run detail from one API.
- Product history views link to durable event/log evidence.
- A maintainer-facing GitHub artifact can be traced back to the PatchHive run
  that created it.

See [Fix soon](fix_soon.md) for tactical reliability items that support this
lane.

## 8. Secrets, Tokens, And Permissions

PatchHive is intentionally operator-owned, but write-capable products still
hold high-impact credentials.

Required guardrails:

- keep least-privilege token scope docs current per product
- distinguish "token missing", "token invalid", "token lacks scope", and
  "target repo disabled this feature"
- validate tokens at startup where possible and preserve per-request permission
  failures when repository-specific access differs
- warn clearly when encryption keys are absent and secret fields fall back to
  memory-only behavior
- document key rotation and product-specific token rotation before broader use
- ensure browser-facing APIs return redacted secret state only

Acceptance criteria:

- Each product doc lists the minimal GitHub token permissions it needs.
- Startup checks and failed runs tell the operator whether to rotate, rescope, or
  enable a GitHub feature.
- Secret storage behavior is understandable before HiveCore starts managing
  credentials across the suite.

See [GitHub token scopes](github-token-scopes.md).

## 9. Storage And Local Operations

SQLite is the right alpha storage choice, but the operational story still needs
to be explicit.

Required docs and behavior:

- document SQLite concurrency limits, backup expectations, and migration rules
- keep product tables namespaced as engines move into the suite backend
- add clear recovery notes for corrupted local target/build directories versus
  corrupted databases
- provide one-command local startup paths for "suite backend plus one product"
  and "suite backend plus HiveCore"
- keep ports, env files, and service-token quoting behavior documented during
  gateway/in-process migration

Acceptance criteria:

- A local reboot or killed process has an obvious recovery path.
- Operators can back up and restore product databases without reverse
  engineering paths.
- Startup docs match the actual unified-backend direction instead of preserving
  obsolete per-product habits.

## Stabilization Order

1. Keep CI drift and Rust checks green.
2. Continue in-process unified backend migration product by product.
3. Move suite schedules and scope policy into the unified backend.
4. Add per-run log artifacts and phase events.
5. Finish token-scope docs, startup permission messages, and secret-state
   warnings for every product.
6. Build the sandbox integration path for read-only products and RepoReaper
   draft PRs.
7. Add frontend JSDoc/API adapter contracts where shared v2 state is most
   active.
8. Revisit TypeScript only after v2 surfaces stabilize.

## Non-Goals For This Phase

- Replacing SQLite.
- Rewriting all frontends in TypeScript immediately.
- Removing every old product backend before the unified backend has equivalent
  product behavior.
- Turning gateway mode into the permanent architecture.
- Adding new autonomous write capability without corresponding policy,
  budget, run-history, and observability support.
