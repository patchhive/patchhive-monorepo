# Suite Backend Direction

PatchHive should move toward one shared backend runtime for the suite while keeping each product as a distinct specialist experience.

The preferred long-term shape is **Option A: one shared PatchHive backend Docker image**.

```text
SignalHive frontend      \
ReviewBee frontend       \
TrustGate frontend        \
RepoMemory frontend        > patchhive-backend
MergeKeeper frontend      /
...                      /
HiveCore frontend        /
```

HiveCore becomes the cockpit and control-plane frontend. The unified backend becomes the suite runtime that owns auth, credentials, product registry, run history, shared config, product APIs, and cross-product orchestration.

The canonical implementation lives in `services/patchhive-backend/` in the PatchHive monorepo. The standalone `patchhive/patchhive-unified-backend` repository is an exported mirror target, not a second source of truth. Update it from the monorepo with `./scripts/export-service.sh patchhive-backend ...` instead of manually editing or copying a second checkout.

## Decision

Standalone product repositories should eventually launch the shared PatchHive backend Docker image with only their product enabled, instead of carrying long-term independent backend implementations.

For example:

```yaml
services:
  backend:
    image: patchhive/patchhive-backend:latest
    environment:
      PATCHHIVE_PRODUCTS: signal-hive

  frontend:
    build: ./frontend-v2
    environment:
      VITE_API_URL: http://localhost:8010/api
```

For the full suite:

```yaml
services:
  backend:
    image: patchhive/patchhive-backend:latest
    environment:
      PATCHHIVE_PRODUCTS: all

  hivecore:
    build: ./products/hive-core/frontend-v2
    environment:
      VITE_API_URL: http://localhost:8100/api
```

This keeps product repos real while avoiding twelve permanent backend forks.

## Why

The product-per-backend shape was useful while products were becoming real, but it creates repeated local friction:

- separate API keys per product
- separate GitHub token and provider setup
- separate `.env` load paths
- many local backend ports to remember
- repeated auth/bootstrap UI
- repeated health, run, and capability glue
- repeated GitHub clients, rate limits, and error handling
- more ways for HiveCore and product frontends to disagree

One backend makes PatchHive easier to manage and lets shared improvements land once.

## Product Identity

One backend does not mean one product.

Each product should keep:

- its name, accent, UI surface, and workflow language
- product-owned decision logic
- product-specific safety boundaries
- product-specific run types and evidence
- product-specific storage where it still makes sense
- standalone repo packaging and launch docs

The backend should host specialist product engines. It should not flatten every product into a generic scanner.

## Runtime Modes

PatchHive should support three runtime modes.

### Suite Mode

Suite mode runs the shared backend with every enabled product.

```bash
PATCHHIVE_PRODUCTS=all patchhive-backend
```

HiveCore and product frontends all connect to the same API. HiveCore can show product health, trigger runs, manage shared defaults, rotate credentials, and route handoffs because it is talking to the same runtime the product frontends use.

### Product Mode

Product mode runs the same backend with one product enabled.

```bash
PATCHHIVE_PRODUCTS=signal-hive patchhive-backend
```

This is the normal standalone developer and product-repo experience. SignalHive still feels like SignalHive, but it uses the shared backend runtime.

### Gateway Mode

Gateway mode is temporary migration support.

The unified backend can proxy to existing product backends while product logic is being moved into the shared runtime. This keeps current products working while the suite backend grows product by product.

Gateway mode should be treated as a bridge, not the end state.

Gateway targets are declared in product registry manifests with optional
`[gateway]` blocks. The unified backend validates a request against the
product's `[[routes]]` claims before forwarding it to the configured target.
SignalHive is the first gateway target, defaulting to `http://127.0.0.1:8010`
and overrideable with `SIGNAL_HIVE_GATEWAY_URL`.

Product availability is also manifest-driven. Each product declares a `[health]`
endpoint, timeout budget, and expected healthy status. Gateway mode checks that
health contract before routing non-health product traffic, so dead product
services fail fast as `product-unavailable` instead of leaking raw proxy errors
through normal product workflows.

## HiveCore Role

HiveCore should connect to `patchhive-backend` like every other frontend.

HiveCore owns the operator cockpit:

- suite radar and product status
- launch links and local stack awareness
- shared defaults and product config
- product capability visibility
- suite-wide run history
- credential status and rotation workflows
- cross-product handoffs
- suite-wide schedules and orchestration controls

The backend owns the authority:

- operator auth
- GitHub and AI provider credentials
- product registry
- product route mounting
- product run dispatch
- schedule claiming and dispatch
- SQLite storage
- rate limits
- audit events
- action approval enforcement

HiveCore should not need private product database reads or browser-exposed service tokens to control the suite.

## Suite Runs And Fixes

The unified backend should eventually support HiveCore-owned suite runs: one orchestration run can dispatch product-owned scans, combine the resulting findings, and expose which findings have explicit product-owned fix actions.

The guiding rule is simple:

- every product can scan
- some products can fix
- every fix is explicit, auditable, policy-constrained, and routed through advertised product capabilities

Scan actions should stay read-only. Fix actions should be separate mutating capabilities with approval metadata, required scopes, quality gates, and run history. Products that naturally own a fix type should eventually advertise a fix action for that type, while guard products such as TrustGate and memory products such as RepoMemory strengthen or block fixes instead of silently mutating code.

See [Suite runs and fix capabilities](suite-runs-and-fix-capabilities.md).

AI-capable products should also converge on a shared Squad setup contract:
product-owned roles backed by shared provider/model discovery, model testing,
encrypted per-agent secret storage, presets, readiness checks, and HiveCore
visibility. RepoReaper is the first implementation, but the shared pieces should
move into `patchhive-product-core` only as a second product needs AI roles. See
[Shared Squad architecture](shared-squad-architecture.md).

## First Frontend To Connect

HiveCore should be the first frontend wired to `patchhive-backend`.

That does not mean HiveCore owns the product engines first. It means the first useful backend skeleton should expose enough suite-level routes for HiveCore to prove the control-plane shape:

- operator auth/session
- product registry
- product health/status
- shared credential status
- run index
- event log
- gateway dispatch to existing product backends

Once HiveCore can see and control the suite through the unified backend,
product frontends can follow the same API base one by one. MergeKeeper is the
first in-process product-engine proof because its PR-readiness loop is small
enough to mount cleanly while still exercising auth, history, GitHub reads, and
optional external write/report scopes. ReleaseSentry is the second in-process
product proof because it broadens the read-only GitHub surface to Actions,
tags, releases, issues, and changelog contents without introducing mutating
product behavior. DepTriage is the third in-process product proof because it
adds dependency PR grouping and Dependabot alert permission edge cases while
remaining read-only. VulnTriage is the fourth in-process product proof because
it exercises GitHub security-feed permissions, disabled-alert messaging, and
security-read startup posture without adding mutation. FlakeSting is the fifth
proof because it adds Actions history, workflow/job evidence, and trend logic.
ReviewBee is the sixth proof because it adds PR review/thread reads, signed
webhooks, and explicit maintained-comment publishing. TrustGate is the seventh
proof because it adds durable policy state, diff review, GitHub report writes,
and FailGuard handoff. RepoMemory is the eighth proof because it adds durable
memory tables, prompt packs, consumer-aware context retrieval, operator
curation, and the FailGuard candidate review lifecycle. These eight engines
are now integrated; SignalHive is next in the migration ladder below.

## API Shape

Product APIs should be mounted under stable namespaces:

```text
/api/products/signal-hive/scans
/api/products/review-bee/review/github/pr
/api/products/trust-gate/review/github/pr
/api/products/repo-memory/prompt-pack
/api/products/merge-keeper/assess/github/pr
/api/products/release-sentry/check/github/release
/api/products/dep-triage/scan/github/dependencies
/api/products/flake-sting/scan/github/actions
```

Shared suite APIs should live outside product namespaces:

```text
/api/auth/session
/api/config/defaults
/api/credentials/github
/api/products
/api/runs
/api/events
/api/schedules
```

Product frontends should not need to know whether a request is handled by an in-process product module or by temporary gateway proxying.

## Suite Scheduling

Scheduling should be a suite-level capability in v2, not a separate schedule UI
rebuilt inside every product.

SignalHive and RepoReaper already prove that product-local schedules are useful,
but the long-term owner should be the unified backend with HiveCore as the
operator surface:

- HiveCore shows schedules, cadence, next run, last run, owner product, target
  scope, enabled state, and recent failures.
- `patchhive-backend` stores suite schedule records, claims due schedules, and
  dispatches the product-owned action through the same capability contract used
  for manual runs.
- Products advertise scheduleable actions in their manifests or capabilities,
  including whether the action is read-only, mutating, requires approval, opens
  PRs, or needs specific credentials.
- A scheduled execution creates the same product run record and suite event
  trail as a manual click. The UI should never have a separate "scheduled run"
  history that cannot be inspected like a normal run.
- Product-local schedule endpoints can remain during gateway mode for backward
  compatibility, but new v2 schedule UX should point at `/api/schedules`.
- Shared Rust schedule and dispatch request contracts live in
  `patchhive-product-core::contract`; product-local schedules should expose a
  suite-shaped compatibility view while their storage remains product-owned.

Minimum suite schedule fields:

```json
{
  "id": "sched_01K4Y92A3B1M3A9JX9Y3R5J0A",
  "name": "daily-maintenance-scan",
  "product": "signal-hive",
  "action_id": "run_scan",
  "cadence": "daily",
  "cron": "0 6 * * *",
  "timezone": "America/Indiana/Indianapolis",
  "enabled": true,
  "target_scope": {
    "mode": "discovery",
    "query": "bug triage, maintenance",
    "language": "rust"
  },
  "approval_policy": "read_only_auto",
  "next_run_at": "2026-07-04T10:00:00Z",
  "last_run_id": "run_01K4Y91P8V3M0M7XJZQ8Q4V4FP"
}
```

Scheduling defaults:

- read-only scan schedules may run automatically when credentials and allowlist
  rules are valid
- mutating schedules should create approval-needed dispatches unless the suite
  policy explicitly permits an automated fix lane
- missed schedules should be visible as events instead of silently skipped
- schedules should be pausable suite-wide and per product
- schedules should respect global PR caps, rate limits, allowlists, denylists,
  opt-outs, and product health

## Suite Scope Policy

Allowlists, denylists, opt-outs, and saved discovery scopes should become
suite-level policy owned by `patchhive-backend` and surfaced by HiveCore. They
are not merely SignalHive or RepoReaper settings.

The old product UIs proved these controls are necessary before autonomous
behavior feels trustworthy. In the unified backend shape, they should move
toward shared records that every product can consult before discovery, cloning,
commenting, PR creation, schedule dispatch, or other external effects.

Required policy concepts:

- `allowlist`: when present, autonomous discovery may only act inside this set
  after stronger exclusions are applied
- `denylist` / `blocklist`: never discover, score, clone, patch, comment on, or
  open PRs against matching repos
- `opt_out`: strongest exclusion; durable across the suite and visible in every
  relevant product surface
- saved scope: named topic/org/language/repo filters that can power manual,
  autonomous, or scheduled runs
- policy audit: each run should record which policy entries and saved scope
  constrained it

Precedence:

1. opt-out
2. denylist/blocklist
3. allowlist
4. saved scope
5. default product discovery

New v2 UI should expose policy status before a run starts and in run history
after the run completes. Product-local repo-list endpoints may remain during
gateway mode, but the long-term API should be suite-owned so HiveCore can pause
or constrain the whole system without editing twelve product databases.

## Product Registry Manifests

The unified backend should treat product registration as a manifest-driven plugin
boundary, not as a hardcoded list in `main.rs`.

The current v1 format lives in `services/patchhive-backend/registry/products/*.toml`.
Each product manifest declares:

- product identity: `key`, `code`, `name`, and `role`
- in-process mounting target: `module_path`
- API namespace: `route_prefix`
- migration state: `migration_stage`
- capability metadata: `[[capabilities]]` with optional `mutating`
- safety boundaries: `[safety]` read-only state, external writes, repo mutation, approval requirements, credential scopes, and evidence expectations
- gateway target: optional `[gateway]` default URL and env override while product logic still lives in the old backend
- health contract: `[health]` endpoint, timeout budget, and expected healthy status for availability checks
- route claims: `[[routes]]` with method, path, and description

This gives HiveCore and the backend one source of truth for what a product is,
what routes it owns, what it can do, and what safety posture applies before
capability dispatch is implemented.

## Product Engine Rule

Product logic should be portable inside the shared backend.

The long-term code shape should be:

```text
patchhive-backend
  shared platform services
  products/
    signal_hive/
    review_bee/
    trust_gate/
    repo_memory/
    merge_keeper/
```

Each product module should own its scoring, heuristics, reports, and product-specific evidence. Shared behavior such as auth, GitHub transport, rate limiting, SQLite pools, run envelopes, credential handling, and health contracts should live in shared backend/platform code or shared crates.

Standalone product repos should package the shared backend image, not copy product backend source as a permanent fork.

## Storage

The preferred long-term storage shape is one backend-owned SQLite database with namespaced product tables and shared suite tables.

The first shared DB implementation now lives in `services/patchhive-backend/src/db.rs`
and is configured with `PATCHHIVE_DB_PATH`. It initializes suite-level tables
for events, run summaries, registry overrides, and shared config.

Shared tables should cover:

- operator auth/session state
- credentials and credential metadata
- product registry
- run index and event log
- schedules
- shared defaults
- audit events

Product modules may keep product-owned tables for product-specific evidence and detail. Table names should be namespaced by product, such as `signal_hive_scans` or `trust_gate_reviews`.

During migration, the unified backend can attach or import existing product SQLite files where that is safer than a one-shot migration.

## Security Rules

- Browser code should never receive GitHub tokens, AI provider keys, product service tokens, or launcher secrets.
- The unified backend should enforce operator auth before routing product requests.
- Product actions should advertise capability metadata before HiveCore dispatches them.
- Destructive, mutating, or outbound contribution actions need explicit approval flows.
- Product-specific risk decisions should stay inside the product module.
- Shared rate limits should apply to auth, GitHub, provider, and mutating routes.
- The backend should log dispatches and preserve enough evidence to debug cross-product runs.

## Standalone Repositories

Existing product repositories remain useful.

Their future shape should be:

```text
patchhive/signalhive
  frontend-v2/
  docker-compose.yml
  README.md
  config examples
  uses patchhive/patchhive-backend with PATCHHIVE_PRODUCTS=signal-hive
```

The product repo still gives users a focused standalone product experience. It just does not own a permanent independent backend implementation.

Standalone product repos should pin a backend image version for releases and may use `latest` only for local development docs.

## Migration Path

This should be gradual.

1. Keep current product backends working.
2. Create the shared `patchhive-backend` runtime skeleton.
3. Add product enable flags such as `PATCHHIVE_PRODUCTS=signal-hive` and `PATCHHIVE_PRODUCTS=all`.
4. Wire HiveCore frontend to the unified backend first for suite auth, product registry, product health, run index, and gateway dispatch.
5. Add gateway routes so the unified backend can proxy existing product APIs during migration.
6. Point product v2 frontends at the unified backend after HiveCore proves the control-plane path.
7. Move product engines in-process according to the no-proxy ladder below.
8. Update each standalone product repository to use the shared backend image with only that product enabled.
9. Retire old individual product backends only after each product's logic is safely hosted by the unified backend and standalone repo packaging is updated.
10. Keep HiveCore's control-plane frontend connected throughout the migration, but move its remaining backend behavior last after enough specialist engines share the runtime.

## No-Proxy Product Migration Ladder

This order is the suite's working migration sequence. It deliberately grows
from narrow read paths into security, durable state, discovery, local
filesystem work, autonomous mutation, and finally control-plane consolidation:

1. **MergeKeeper** — PR read state, simple decision, low mutation risk.
2. **ReleaseSentry** — broader GitHub read surface: Actions, tags, releases, issues, and changelog.
3. **DepTriage** — dependency PRs plus alert-permission edge cases.
4. **VulnTriage** — security permissions and degraded/fallback messaging.
5. **FlakeSting** — Actions history, workflow/job evidence, and trend/history logic.
6. **ReviewBee** — PR review/comment surfaces and maintained-comment behavior when enabled.
7. **TrustGate** — policy/rule storage, diff review, and FailGuard handoff.
8. **RepoMemory** — durable memory tables, prompt packs, and FailGuard candidates.
9. **SignalHive** — discovery/scanning, presets, schedules, repository lists, and larger scan payloads.
10. **RefactorScout** — local/GitHub repository intake, filesystem safety, and temporary clone lifecycle.
11. **RepoReaper** — AI squad, secrets, cloning, patching, testing, pull requests, and long-running jobs.
12. **HiveCore** — cockpit/control-plane consolidation once enough products are actually unified.

The ladder governs in-process engine migration, not whether HiveCore may use the
unified backend earlier as a frontend and gateway client. The first eight
products through RepoMemory are integrated as of 2026-07-12; SignalHive is
next in the engine-migration ladder.

## Open Questions

- What should the first `patchhive-backend` crate/package layout be?
- Should the shared backend image be published as `patchhive/patchhive-backend` or another image name?
- How should product-specific migrations be versioned inside one backend binary?
- How much existing product SQLite data needs migration versus fresh v2 storage?
- What route names should become permanent for the namespaced product API?
- What is the minimum approval contract before RepoReaper can run through the unified backend?

The direction is clear: one backend runtime, many product faces.
