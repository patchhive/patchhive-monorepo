# AGENTS.md — PatchHive Project Context For Codex

This file gives Codex the repo context that was previously captured in `CLAUDE.md`.
Keep it up to date when the architecture, conventions, or product inventory changes.

## What PatchHive Is

PatchHive is a software maintenance platform: a family of focused tools that help engineering teams find, prioritize, and automate maintenance work without losing reviewability or trust.

Core principles:
- Maintenance work should be continuously visible.
- Automation should be constrained and reviewable.
- Trust should be earned through signal quality, not hype.

Builder: Jeremy Coe (`@coe0718`). PatchHive is being built for personal use first; outside adoption is a bonus.

## North Star

PatchHive is not "another AI coding assistant." Its distinct identity is autonomous, outbound contribution:
- PatchHive should find work on its own, act on repos the operator did not hand-pick individually, and contribute under the PatchHive identity.
- The operator delegates at a high level by choosing topics, languages, auth, and settings; the products should discover repos, issues, and PR opportunities themselves.
- Reputation should accrue to the PatchHive GitHub account through consistent, high-quality output, not by trying to look like a human contributor.
- Positioning should stay centered on radical delegation and autonomous contribution, not on interactive pair-programming.

## Transparency Policy

- Autonomous PRs should come from the PatchHive GitHub account, not the operator's personal account.
- PR bodies should clearly disclose that the work was generated autonomously by the relevant PatchHive product.
- Keep attribution direct and confident, not apologetic. The work should stand on its own while remaining clearly labeled.
- Trust is built through visible history: maintainers should be able to inspect PatchHive's past contributions and judge the work accordingly.

## Operator Experience

The intended UX across PatchHive products is:
- User authenticates with GitHub and/or AI provider access.
- User chooses broad topics and language areas to work on.
- User clicks Run.
- The product discovers repos and candidate work on its own instead of asking the user to pick exact repos, issues, or PRs manually.
- Per-product defaults can live inside each product, and HiveCore should eventually provide global settings across the suite.

## Product System Shape

PatchHive is a suite of specialist products that should eventually behave like one coherent agent:
- SignalHive is the reconnaissance / signal-discovery layer.
- TrustGate is the safety / trust layer.
- RepoMemory is the durable memory / conventions layer.
- RepoReaper is the autonomous patch-and-PR execution layer.
- HiveCore is the eventual brain / orchestration layer that connects the specialist products into one system.

The general pattern to preserve:
- visibility first
- trust and memory second
- autonomous write actions after that foundation exists

RepoReaper was built first because it descended from Jeremy's earlier GitFix experimentation. That means the highest-autonomy product exists early, but the long-term suite should still mature toward a full pipeline of signals -> memory/trust -> action.

## Monorepo Structure

```text
patchhive/
  packages/
    ui/                     @patchhivehq/ui shared React component library
    ui-v3/                  @patchhivehq/ui-v3 Lovable-derived specialist product UI
    product-shell/          @patchhivehq/product-shell shared frontend shell/auth helpers
    ai-models/              @patchhivehq/ai-models shared AI provider/model selector UX
    ai-local/               @patchhive/ai-local localhost AI gateway
  crates/
    patchhive-product-core/ shared Rust auth + startup helpers
    patchhive-github-pr/    shared Rust GitHub PR/diff/check helpers
    patchhive-github-data/  shared Rust GitHub repo/issue/history/actions reads
    patchhive-github-security/ shared Rust GitHub security/advisory reads
  templates/
    product-starter/        shared starter for new PatchHive products
  services/
    patchhive-backend/     shared PatchHive suite backend runtime
    patchhive-launcher/     localhost-only host-control daemon for HiveCore first-stack start/stop
    patchhive-registry/     opt-in registry service for sanitized public suite snapshots
  products/
    repo-reaper/            built first, current active product
    signal-hive/
    review-bee/
    trust-gate/
    repo-memory/
    merge-keeper/
    flake-sting/
    dep-triage/
    vuln-triage/
    refactor-scout/
    release-sentry/
    hive-core/
  package.json              npm workspaces root
  README.md
  CLAUDE.md                 legacy Claude-oriented context file
  AGENTS.md                 Codex-oriented context file
```

## Tech Stack

### Warning-Free Code Policy

- Do not leave compiler, clippy, linter, type-checker, test, or production-build
  warnings in the codebase. Fix the underlying issue before considering work
  complete.
- Rust verification should include `cargo clippy --all-targets -- -D warnings`
  for every changed crate or service, in addition to formatting and tests.
- Frontend work should run the strictest configured lint/type/build checks for
  the changed package and any shared-package consumers it affects.
- Do not silence a warning with a broad `allow`, disabled rule, or ignored
  result merely to make verification green. A narrowly scoped suppression is
  acceptable only when the warning is demonstrably unavoidable and the reason
  is documented beside it.
- Product-domain warnings returned by scans or startup diagnostics are runtime
  evidence and are not prohibited by this policy; the prohibition applies to
  warnings produced by the code-quality toolchain.

Backend:
- Rust
- `axum`, `rusqlite`, `reqwest`, `tokio`, `serde`, `serde_json`, `chrono`, `uuid`, `anyhow`, `tracing`
- Shared API rate limiting defaults to 300 standard requests/minute and 30 auth or mutating requests/minute; tune with `PATCHHIVE_RATE_LIMIT_MAX`, `PATCHHIVE_RATE_LIMIT_SENSITIVE_MAX`, and `PATCHHIVE_RATE_LIMIT_WINDOW_SECS`.
- Shared SQLite pools default to 4 connections; tune with `PATCHHIVE_DB_POOL_SIZE` or a product-specific `<PRODUCT>_DB_POOL_SIZE`.
- Write-capable validation uses `patchhive_product_core::validation::TestExecutionStatus`; only `passed` permits a non-draft autonomous PR.

Frontend:
- React + Vite
- Existing production and v2 product frontends use JavaScript and shared CSS variables.
- UI v3 may use Tailwind utility classes where they are part of the canonical Lovable implementation; do not translate them into a different visual system merely to preserve the older no-framework convention.
- TypeScript is allowed when extracting code directly from the Lovable UI, but product v3 frontends may remain JSX when that preserves the same rendered result with less deployment churn.

AI provider integration:
- Direct HTTP via `reqwest`
- Preserve support for Anthropic, OpenAI, Gemini, Groq, Ollama, and custom OpenAI-compatible endpoints
- No provider SDK dependencies unless there is a compelling repo-wide change
- Prefer `PATCHHIVE_AI_URL` for PatchHive-wide OpenAI-compatible local gateways before falling back to raw provider endpoints

Data/storage:
- SQLite only
- `rusqlite` with raw SQL, no ORM

Packaging:
- Each product should have `docker-compose.yml`, `backend/Dockerfile`, and `frontend/Dockerfile`
- `@patchhive/ai-local` is the shared localhost gateway for user-owned Codex/Copilot sessions

Shared platform guidance:
- Shared auth/provider infrastructure should live in a shared package instead of being reimplemented per product.
- Keep product APIs close enough that HiveCore can orchestrate them without heavy translation layers.
- Standardize request/response envelopes, error shapes, run/job identifiers, and async webhook/run lifecycle patterns as products are built out.
- Treat repo discovery safety, output caps, and cross-product contracts as platform guardrails, not optional product polish.
- The unified backend product registry lives in `services/patchhive-backend/registry/products/*.toml`; product modules should declare identity, route claims, capabilities, safety boundaries, gateway targets, health contracts, and module paths there instead of being hardcoded in `main.rs`.
- The unified backend shared SQLite DB is configured with `PATCHHIVE_DB_PATH`; suite tables stay backend-owned, while product tables should be product-namespaced as engines migrate in-process.
- All product routers should layer `patchhive_product_core::rate_limit::rate_limit_middleware` so auth, mutating, and run-triggering routes share backend rate limiting.
- GitHub-enabled products should use `patchhive_product_core::github_auth::verify_github_token` at startup. Token presence is configuration, `github_ready` means GitHub accepted the authenticated identity request, target read access is verified during the run, and write readiness is only proven by a successful target-specific write.
- When the same Rust backend seam exists in 2 or more products, prefer extracting it into `crates/patchhive-product-core` before starting another product.
- See [docs/platform-guardrails.md](/home/coemedia/Documents/code/patchhive/docs/platform-guardrails.md) and [docs/product-api-contract-v1.md](/home/coemedia/Documents/code/patchhive/docs/product-api-contract-v1.md).

## Shared UI Package

Location: `packages/ui/`

Every product frontend should import shared theme, primitives, layout shell, and reusable components from `@patchhivehq/ui`.

Rules:
- If a component will appear in 2 or more products, put it in `packages/ui/src/components/` and re-export it from `packages/ui/src/index.js`.
- If a component is product-specific, keep it inside that product.
- Product apps should call `applyTheme("<product-key>")` from `App.jsx`.

## UI v2 Track

Location: `packages/ui-v2/`

PatchHive is prototyping a suite-wide UI v2 direction separately from the current production frontends.

Rules:
- Keep v2 experiments in `products/<product>/frontend-v2/` until the new design system is ready to replace production UI code.
- Keep current production frontends in `products/<product>/frontend/` during the transition.
- Do not mix v2 work into old UI code while the direction is still being tested.
- SignalHive's active v2 prototype lives in `products/signal-hive/frontend-v2/`.
- HiveCore does not currently follow the v2 direction, but it should converge before old UI code is legacy or removed.
- Once every product has moved to v2, old UI code should either move to a clearly named legacy path or be removed.
- See [docs/ui-v2-migration.md](/home/coemedia/Documents/code/patchhive/docs/ui-v2-migration.md).

## UI v3 Track

Location: `packages/ui-v3/`

UI v3 is the next specialist-product interface. Its canonical visual source is
`unified-ui-revamp-main/`, the Lovable project. MergeKeeper, ReleaseSentry,
DepTriage, VulnTriage, FlakeSting, ReviewBee, and TrustGate have v3 frontends because their engines
are mounted in-process by the unified backend. Do not start another product's
v3 frontend until its unified-backend engine reaches `integrated`.

Rules:
- Use the actual Lovable component structure, theme tokens, typography, spacing, radii, glass surfaces, shadows, backgrounds, and responsive behavior. Do not approximate it from screenshots or replace it with a static mockup.
- Every specialist product remains an independent frontend, Docker image, API integration, and workflow. Share only the stable visual shell and primitives through `@patchhivehq/ui-v3`.
- Reuse `@patchhivehq/ui-v3` progressive lists, saved dashboard views, filter/sort controls, and activity timelines across specialist products; products supply their own field and event mappings.
- Product differences belong in product name/icon, accent colors, copy, tabs, data, forms, actions, and workflow-specific panels.
- Keep the specialist footer identity aligned with v2: `<Product> by PatchHive`, the product subtitle, and `Autonomous maintenance suite`.
- Preserve the suite-wide light/dark preference under the `patchhive.theme` localStorage key and apply it before React mounts to prevent a theme flash.
- Keep current v2 frontends functional until each product's v3 parity audit passes. Do not rewrite v2 in place.
- VulnTriage passed its final parity audit on 2026-07-10; its canonical v3 UI lives in `products/vuln-triage/frontend/`, and its v1/v2 trees have been removed.
- MergeKeeper and ReleaseSentry passed their final parity audits on 2026-07-11;
  their canonical v3 UIs live in `products/<product>/frontend/`, and their
  v1/v2 trees have been removed.
- DepTriage and FlakeSting passed their final parity audits on 2026-07-12;
  their canonical v3 UIs live in `products/<product>/frontend/`, and their
  v1/v2 trees have been removed.
- ReviewBee passed its final parity audit on 2026-07-12; its canonical v3 UI
  lives in `products/review-bee/frontend/`, and its v1/v2 trees have been
  removed.
- TrustGate is integrated in-process and its parity candidate lives in
  `products/trust-gate/frontend-v3/`. Keep `frontend/` and `frontend-v2/`
  until live diff review, policy persistence, history, diagnostics, and
  GitHub publishing pass final acceptance.
- Prefer finishing and validating v3 for the current integrated product set before moving another product engine into the unified backend.
- HiveCore is intentionally outside the specialist-product v3 migration and keeps its control-plane UI.
- `prototypes/vuln-triage-calm-mockup.html` is reference material only; it is not the v3 source of truth.
- See [docs/ui-v3-migration.md](/home/coemedia/Documents/code/patchhive/docs/ui-v3-migration.md).

## Shared Product Shell Package

Location: `packages/product-shell/`

Every product frontend that uses PatchHive's API-key login flow should import shared auth/bootstrap behavior from `@patchhivehq/product-shell`.

Rules:
- If API-key login bootstrap is the same across 2 or more products, keep it in `product-shell`, not inside a product `App.jsx`.
- If authenticated backend `fetch` behavior is repeated across 2 or more products, keep it in `product-shell`.
- If setup, readiness, or first-run wizard UI is shared across 2 or more products, keep it in `product-shell` unless HiveCore-specific orchestration behavior is required.
- Avoid direct `localStorage` reads across individual panels when the app shell can pass the resolved API key down instead.

## Shared AI Models Package

Location: `packages/ai-models/`

AI-capable product frontends should import provider catalog and model selector behavior from `@patchhivehq/ai-models` instead of carrying one-off provider/model dropdowns.

Rules:
- Keep frontend provider labels, fallback model lists, live/static model status copy, and model refresh UX here.
- Product backends should expose `GET /models/:provider` and `POST /models/:provider` when they use this package.
- Browser code should not call third-party AI providers directly. It may pass a user-entered provider key to the local product backend for one-time model discovery.
- Keep actual AI request execution in the product backend or a shared Rust crate once 2 or more products need the same backend model-discovery/runtime seam.
- Custom providers should use OpenAI-compatible chat and model-list APIs and carry an explicit base URL in product config or agent config.
- RepoReaper's agent team is the seed of a shared PatchHive Squad architecture: product-owned AI roles backed by shared provider/model discovery, model testing, noisy model filtering, encrypted per-agent secret storage, presets, readiness checks, and HiveCore visibility. Do not clone the RepoReaper team builder into future products; extract the common Squad substrate into `patchhive-product-core` when a second AI-capable product needs it. See [docs/shared-squad-architecture.md](/home/coemedia/Documents/code/patchhive/docs/shared-squad-architecture.md).

## Shared Rust Product Core

Location: `crates/patchhive-product-core/`

Every product backend that repeats PatchHive's API-key auth or typed startup checks should use `patchhive-product-core` instead of carrying its own copy.

Rules:
- If a Rust backend seam already exists in 2 or more products, extract it into `patchhive-product-core` before a third product repeats it.
- Keep the crate focused on backend primitives, not product behavior.
- Product backends should use `listen_addr()` so `PATCHHIVE_BIND_ADDR` can force loopback-only local runs when Docker-style `0.0.0.0` binding is not desired.
- Product backends should use `SqlitePool` from `patchhive-product-core` instead of a single global `Mutex<Connection>` or ad hoc connection opens. Tune globally with `PATCHHIVE_DB_POOL_SIZE` or with a product-specific `<PRODUCT>_DB_POOL_SIZE`.
- Product backends should define their `crate::auth` module with `define_api_key_auth_module!` in `main.rs` instead of carrying one-file delegation wrappers.
- Good candidates: auth middleware, SQLite pooling, startup/health helpers, generic ID or envelope helpers, generic named preset storage interfaces.
- Shared `TokenProtector` encryption keys must contain at least 32 characters of machine-random material; generate them with `openssl rand -hex 32` and keep them stable across restarts.
- Future Squad candidates: shared AI agent config types, encrypted active-squad and preset storage, redacted browser views, provider/model readiness checks, and HiveCore-facing Squad capability metadata once at least two products need AI roles.
- Bad candidates until proven generic: GitHub search logic, scoring heuristics, pipelines, route behavior, and product-specific SQLite schemas.

## Shared GitHub PR Crate

Location: `crates/patchhive-github-pr/`

Every product backend that needs GitHub PR diff fetch, signed webhook verification, check/status publishing, or maintained PR comments should use `patchhive-github-pr` instead of carrying a private copy.

Rules:
- Keep the crate focused on GitHub PR transport and lifecycle plumbing.
- Good candidates: token/env helpers, webhook signature verification, PR metadata fetch, diff fetch, check/status publishing, managed PR comments.
- Keep product-owned report text, policy decisions, scoring, and escalation logic outside the crate.

## Shared GitHub Data Crate

Location: `crates/patchhive-github-data/`

Every product backend that needs GitHub repository search, issue history, merged PR history, historical review feedback, or Actions workflow reads should use `patchhive-github-data` instead of carrying a private copy.

Rules:
- Keep the crate focused on GitHub read paths and typed response shapes.
- Good candidates: token/env helpers, repo fetch/search, issue history, PR history, review/comment/file reads, code search counts, Actions workflow runs/jobs.
- Keep PR webhook verification, PR comment/check publishing, and other PR lifecycle mechanics in `patchhive-github-pr`.
- Keep product-owned filtering, heuristics, scoring, and routing outside the crate.

## Shared GitHub Security Crate

Location: `crates/patchhive-github-security/`

Every product backend that needs GitHub code scanning alerts, Dependabot alerts, or advisory metadata should use `patchhive-github-security` instead of carrying a private copy.

Rules:
- Keep the crate focused on typed GitHub security reads.
- Good candidates: token/env helpers, code scanning alerts, Dependabot alerts, advisory fields, CWEs, references, EPSS metadata.
- Keep generic repository/issue/history reads in `patchhive-github-data`.
- Keep product-owned ranking, severity interpretation, prioritization, and routing outside the crate.

## Product Starter Template

Location: `templates/product-starter/`

PatchHive should use the shared starter when creating new products instead of copying an existing product directory manually.

Rules:
- The starter repo root is documentation and wrapper context; the actual copied scaffold lives under `templates/product-starter/scaffold/`.
- The starter should hold only the repeated shell: auth wiring, health/startup checks, Docker, CI, frontend shell, and placeholder overview route.
- Product-specific logic should replace starter copy early. Do not let placeholder starter routes linger once a product loop is real.
- Use `./scripts/new-product.sh <product-slug>` to create new products from the starter.
- If a new product backend uses shared git crates, refresh its standalone `Cargo.lock` with `./scripts/refresh-product-lockfile.sh <product-slug>` before the first standalone export.

Product accent keys live in `packages/ui/src/theme.js`:
- `repo-reaper`
- `signal-hive`
- `review-bee`
- `trust-gate`
- `repo-memory`
- `merge-keeper`
- `flake-sting`
- `dep-triage`
- `vuln-triage`
- `refactor-scout`
- `release-sentry`
- `hive-core`

## Frontend Convention

Each product frontend should follow:

```text
products/<name>/frontend/
  src/
    App.jsx
    config.js
    main.jsx
    panels/
    components/
  index.html
  package.json
  vite.config.js
  Dockerfile
  nginx.conf
```

`config.js` convention:

```js
export const API = import.meta.env.VITE_API_URL || "http://localhost:8000";
```

`App.jsx` convention:
- Call `applyTheme("<product-key>")` in a `useEffect`
- Use `ProductSessionGate` and `ProductAppFrame` from `@patchhivehq/product-shell` for auth, layout, tab chrome, footer, and panel error isolation
- Keep tab panels under `./panels/`

## Backend Convention

Each product backend should roughly follow:

```text
products/<name>/backend/
  src/
    main.rs
    state.rs
    db.rs
    agents.rs
    github.rs
    git_ops.rs
    startup.rs
    pipeline.rs
    fix_worker.rs
    routes/
      mod.rs
      config.rs
      history.rs
      webhook.rs
  Cargo.toml
  Dockerfile
```

Auth modules are generated in `main.rs` with `patchhive_product_core::define_api_key_auth_module!`.

For AI-enabled/GitHub-enabled products, keep multi-provider and GitHub helper modules separate rather than collapsing them into `main.rs`.

## Current Product: RepoReaper

Location: `products/repo-reaper/`

Pitch:
- Resolve selected repository issues automatically and open validated pull requests.

What it does:
- Hunts GitHub repos for open bug issues
- Scores them for fixability
- Generates patches with AI agents
- Reviews/refines them
- Runs tests
- Opens PRs

RepoReaper agent roles:
- Scout `◎`: hunts repos and scores issue fixability
- Judge `⚖`: selects relevant files
- Reaper `⚔`: generates the patch
- Smith `⬢`: reviews/refines and can reject low-confidence work
- Gatekeeper `🔒`: runs tests and opens the PR

Key features to preserve:
- Multi-provider AI support
- Confidence scoring surfaced in UI
- Rejected patches log with Smith feedback
- Self-healing patch apply retry
- Configurable test retry count
- Watch Mode via webhook-triggered hunts
- Dry Stalk mode
- Team presets
- Per-run and lifetime cost tracking
- PR monitor
- PatchHive branding in footer and PR bodies

RepoReaper v2 temporary scope:
- `products/repo-reaper/frontend-v2/` has a lightweight agent-team setup so Mission Deck and Dry Stalk can be tested honestly through gateway mode.
- That v2 setup is intentionally not the full old frontend team builder. It can recruit a starter team, edit the active backend team, apply provider defaults, pull and filter provider model lists, and test selected models. Richer per-agent controls and the full preset-management UX remain deferred.
- RepoReaper persists the active team and team presets in SQLite. Per-agent API keys and bot token overrides are encrypted at rest through `patchhive_product_core::secrets::TokenProtector` when `REAPER_ENCRYPTION_KEY` or `PATCHHIVE_ENCRYPTION_KEY` is set; without one of those keys, those secret fields stay memory-only and are not written to SQLite. Adding an encryption key later migrates existing plaintext active-team and preset secrets on boot.
- Dry Stalk is still a no-write mode, but it needs at least a Scout agent because issue scoring and dry-run analysis use the AI agent pipeline.
- Do not remove the old RepoReaper team/preset UI until the v2 replacement and unified-backend/HiveCore setup path cover those workflows.
- When RepoReaper moves from gateway proxying into the unified backend, revisit credential ownership, preset migration/export behavior, approval gates, and HiveCore-driven setup.

RepoReaper defaults:
- Backend: `VITE_API_URL` or the current browser origin
- Frontend: `http://localhost:5173`
- DB: `repo-reaper.db`
- Work dir: `/tmp/repo-reaper`

Important env vars:
- `BOT_GITHUB_TOKEN`
- `BOT_GITHUB_USER`
- `BOT_GITHUB_EMAIL`
- `PROVIDER_API_KEY`
- `PATCHHIVE_AI_URL`
- `OLLAMA_BASE_URL`
- `COST_BUDGET_USD`
- `MIN_REVIEW_CONFIDENCE`
- `RETRY_COUNT`
- `REAPER_MAX_ACTIVE_WORKERS`
- `REAPER_ENABLE_UNTRUSTED_TESTS`
- `REAPER_TEST_SANDBOX`
- `REAPER_ALLOW_HOST_TESTS`
- `REAPER_TEST_TIMEOUT_SECONDS`
- `WEBHOOK_SECRET`
- `REAPER_DB_PATH`
- `REAPER_WORK_DIR`

## Product Lineup

- RepoReaper: autonomous patch-and-PR execution
- SignalHive: maintenance signal and backlog risk detection
- ReviewBee: turn PR review threads into actionable follow-up tasks
- TrustGate: evaluate risk in AI-generated diffs
- RepoMemory: durable repo memory for coding agents
- MergeKeeper: keep PRs mergeable
- FlakeSting: detect and explain flaky tests
- DepTriage: dependency update prioritization
- VulnTriage: rank security findings into engineering work
- RefactorScout: surface safe high-value refactors
- ReleaseSentry: release readiness and ship/no-ship evidence
- HiveCore: suite control plane for visibility, shared defaults, and launch control

## SignalHive Notes

- SignalHive should stay visibility-first and read-only.
- Its job is to surface stale backlog risk, duplicate issues, recurring bug patterns, TODO/FIXME hotspots, and hidden maintenance drag before PatchHive starts changing code.
- SignalHive is the trust-building reconnaissance layer that should make the later autonomous products feel earned rather than abrupt.
- The MVP should stay simple: GitHub issue sync, stale and duplicate heuristics, recurring bug clustering, marker scanning, priority scoring, trend comparison, timeline visuals, scheduled re-scans, and exportable reports/dashboard snapshots.
- Scan presets and schedules are worth supporting early because they make repeated operator workflows sticky without changing SignalHive's read-only posture.
- SignalHive should respect allowlist, denylist, and opt-out controls early so autonomous repo discovery never feels invasive.
- The intended early audience is engineering leads and CTOs at small startups who need maintenance visibility before they are ready for autonomous repo changes.

## ReviewBee Notes

- ReviewBee should stay review-first and merge-speed-first.
- Its job is to turn PR review threads into a concrete, lower-noise follow-up checklist instead of forcing engineers to reread long thread histories.
- The MVP should work without live AI providers by clustering actionable review comments, grouping similar asks, and surfacing which feedback appears resolved versus still active.
- ReviewBee should reuse `patchhive-github-pr` for PR review fetch and thread-state plumbing instead of growing a separate GitHub client.
- ReviewBee should make teams faster at closing PRs before PatchHive asks them to trust broader autonomous write behavior.

## TrustGate Notes

- TrustGate should stay trust-first and review-first.
- Its job is to review AI-generated diffs against repo-specific risk rules and return a simple recommendation: `safe`, `warn`, or `block`.
- The MVP should work without live AI providers or GitHub webhooks by accepting pasted unified diffs and locally stored repo rule sets.
- Repo-specific rules are TrustGate's first memory layer: blocked paths, sensitive paths, suspicious terms, blocked terms, scope caps, and testing expectations.
- TrustGate should complement other coding agents instead of competing with them. It should plug into the rest of PatchHive as a safety gate.
- Early future integrations worth keeping in mind: GitHub status checks, PR diff ingestion, shared policy packs, and incident-informed rule tuning.

## RepoMemory Notes

- RepoMemory should stay context-first and durable-memory-first.
- Its job is to turn merged PRs, reviewer feedback, recurring bug signals, and hotspot history into reusable repo-specific knowledge.
- The MVP should work without live AI providers by extracting useful memory heuristics directly from GitHub data.
- Prompt-pack generation matters early because it is the bridge between remembered repo context and later agent behavior.
- RepoMemory should make both TrustGate and RepoReaper smarter, not compete with them as a separate actor.

## MergeKeeper Notes

- MergeKeeper should stay merge-readiness-first and orchestration-adjacent.
- Its job is to tell a human or another PatchHive product whether a PR is actually ready to merge, on hold, or blocked.
- The MVP should work without live AI providers by reading GitHub PR state, reviewer state, unresolved review pressure, and commit/check health.
- MergeKeeper should become the convergence point for ReviewBee, TrustGate, RepoMemory, and CI signals over time, but it should not wait for all of them before being useful.
- The early UX should stay simple: one PR in, one readiness decision out, with visible reasons.

## FlakeSting Notes

- FlakeSting should stay CI-trust-first and signal-first.
- Its job is to detect flaky tests and unstable workflow behavior before teams normalize unreliable checks.
- The MVP should work without live AI providers by reading GitHub Actions history and looking for fail/pass swings, rerun pressure, runner-specific weirdness, and repeated test instability.
- FlakeSting should explain why a job or step looks flaky, not just assign a scary score.
- The early UX should stay narrow and credible: one repo in, one ranked flaky queue out, with direct evidence back to GitHub runs.
- FlakeSting should make MergeKeeper and broader PatchHive automation safer over time by helping teams trust their CI signal again.

## DepTriage Notes

- DepTriage should stay triage-first and read-only.
- Its job is to turn dependency update noise into a ranked queue of `update now`, `watch`, and `ignore for now` calls.
- The MVP should work without live AI providers by reading open dependency PRs plus optional Dependabot alerts, then scoring urgency with deterministic heuristics.
- DepTriage should help teams spend attention on the dependency work that actually matters instead of making PatchHive look like “another update bot.”

## VulnTriage Notes

- VulnTriage should stay triage-first and read-only.
- Its job is to turn GitHub code scanning and dependency alerts into a ranked queue of `fix now`, `plan next`, and `watch`.
- The MVP should work without live AI providers by scoring severity, reachability proxy, owner hints, and practical next steps with deterministic heuristics.
- Current live GitHub security-feed scans are strongest for repositories where the operator has security-read access; third-party public repositories may return `403` even when the token is valid.
- Outbound/random public repo discovery needs a future public-intelligence fallback mode using OSV/GHSA advisories, manifest and lockfile parsing, public dependency inference, and lightweight code-pattern heuristics. Treat missing GitHub alert access as a product boundary, not a scanner bug.
- VulnTriage should help small teams behave like they have an AppSec triage layer without forcing them to stare at raw GitHub alert noise.
- VulnTriage should reuse `patchhive-github-security` for typed code scanning and Dependabot reads instead of growing another private GitHub security client.

## RefactorScout Notes

- RefactorScout should stay refactor-first, read-only, and conservative.
- Its job is to surface cleanup work with a strong safety-to-value ratio before that structural debt turns into feature drag or bug-prone code paths.
- The MVP should work without live AI providers by scanning local repository paths and ranking explainable heuristics such as oversized files, oversized functions, and repeated string literals.
- RefactorScout should prefer explicit filesystem allowlists and localhost-only scanning by default so repo analysis does not quietly become arbitrary server file access.
- The early UX should stay narrow and credible: one local repo path in, one ranked refactor queue out, with clear evidence and a suggested first move for each lead.

## ReleaseSentry Notes

- ReleaseSentry should stay release-readiness-first and evidence-first.
- Its job is to answer whether a repo, product, or release candidate is actually safe to ship.
- The MVP should work without live AI providers by reading tags, changelog/version drift, branch health, CI status, unresolved blockers, dependency/security pressure, and recent release notes.
- ReleaseSentry should produce a simple decision such as `ready`, `watch`, or `hold`, with the exact blockers and evidence that led to it.
- It should complement MergeKeeper instead of overlapping it: MergeKeeper decides if a PR can merge, while ReleaseSentry decides if the resulting release should go out.
- Early future integrations worth keeping in mind: generated release notes, release checklist presets, package publish guards, GHCR image alignment, and HiveCore suite release verification.

## HiveCore Notes

- HiveCore should stay control-plane-first before it becomes orchestration-heavy.
- Its first job is to make the PatchHive suite legible in one place: product health, launch links, shared defaults, and operational checks.
- The control-plane v1 surface polls health, startup checks, capabilities, product-owned `/runs` history, and server-side `/runs/:id` detail; stored product service tokens unlock protected run reads and capability-driven action dispatch without exposing machine credentials to the browser. Service-token records are now scoped and rotatable, HiveCore can encrypt saved downstream service tokens at rest with `HIVECORE_ENCRYPTION_KEY`, and legacy operator API keys remain only a temporary fallback.
- HiveCore should push the suite toward shared contracts instead of hiding differences forever. It should reveal where products drift and help standardize them.
- HiveCore now reports per-product contract drift for health, startup checks, capabilities, run lists, and run detail support.
- HiveCore's Setup tab should adapt to already-running products first, then use `patchhive-launcher` only for missing local stack pieces. Browser UX stays in HiveCore; Docker and `.env` mutation belong in the launcher service.
- HiveCore-enabled mode means HiveCore owns suite lifecycle coordination, but each product must remain standalone and expose product-owned APIs for that coordination.
- Early future integrations worth keeping in mind: shared run history, suite-wide schedules, global allowlist and denylist propagation, and cross-product handoffs like SignalHive -> TrustGate -> RepoReaper.

## FailGuard Notes

- FailGuard is a cross-cutting capability, not a standalone product.
- Its job is to turn bugs, outages, painful reviews, reverted PRs, and other bad outcomes into reusable future knowledge.
- On the RepoMemory side, that means capturing and storing lessons so humans and agents can reuse them later.
- On the TrustGate side, that means converting those lessons into future warnings, checks, or blocking guardrails.
- The intended flow is: incident or painful failure -> captured lesson -> durable memory -> future policy.
- FailGuard v1 is complete in RepoMemory: `POST /failguard/candidates` queues reviewable bad-outcome lessons, candidates can be promoted or dismissed, and `POST /failguard/lessons` still creates pinned `failure_pattern` policy memories directly.
- TrustGate automatically submits FailGuard candidates for `warn` and `block` reviews when `PATCHHIVE_REPO_MEMORY_URL` is configured.
- RepoReaper automatically submits FailGuard candidates when Smith rejects a generated patch below `MIN_REVIEW_CONFIDENCE`.

## Key Decisions

- Rust backend and React frontend are deliberate and should stay consistent across products.
- Multi-provider AI support in RepoReaper is non-negotiable.
- No AI provider SDKs by default; prefer raw HTTP.
- SQLite only.
- HiveCore should become the orchestration and global-settings layer for the specialist products.
- Products should be buildable independently, but their APIs should converge toward shared contracts so HiveCore can coordinate them.
- Long-term suite direction: one shared `patchhive-backend` runtime with many product frontends. HiveCore should connect to that backend as the control-plane frontend, while standalone product repos eventually launch the shared backend Docker image with only their product enabled. Product identities and workflows remain distinct, and the backend owns shared auth, product registry, credentials/config, routing, run history, and cross-product orchestration. See `docs/suite-backend-direction.md`.
- Product boundaries should be decided early. If a capability clearly strengthens an existing product, build it there; if it needs its own operator workflow, data contract, trust boundary, or repeated lifecycle, create it as a standalone product from the start instead of treating extraction as inevitable cleanup.
- Long-term suite runs should be HiveCore-owned orchestration runs: every product can scan, some products can fix, and any product that naturally owns a fix type should eventually expose an explicit product-owned fix action. Scan actions stay read-only by default; fix actions are separate mutating capabilities with approval metadata, scopes, quality gates, and run history. See `docs/suite-runs-and-fix-capabilities.md`.
- Watch Mode is a UI toggle backed by SQLite settings.
- PatchHive should contribute under its own GitHub identity with explicit autonomous attribution.
- Allowlist, denylist, and opt-out controls should exist early anywhere PatchHive discovers work autonomously.
- Hard quality and rate limits should gate outbound PR creation so PatchHive's reputation compounds in the right direction.

## Git Conventions

- Branch names: `reaper/issue-{number}` for RepoReaper, similar pattern for other products
- Every GitHub-facing PR body, issue/PR comment, and maintained report should
  include explicit attribution and end with
  `*ProductName by [PatchHive](https://github.com/patchhive)*`. Rust products
  should use `patchhive_product_core::branding::append_product_signature` for
  generated Markdown.
- Commit messages should use `fix: {issue title} (closes #{number})` where applicable

## Local Development

```bash
# Local AI gateway
npm install
npm run dev:ai-local

# RepoReaper backend
cd products/repo-reaper/backend
cargo run

# RepoReaper frontend
cd products/repo-reaper/frontend
npm install
npm run dev

# Docker
cd products/repo-reaper
docker-compose up --build
```

## New Product Checklist

1. Create `products/<name>/`.
2. Copy `products/repo-reaper/backend/` as the backend starting point and strip unused pieces.
3. Create `products/<name>/frontend/` with `package.json`, `src/config.js`, `src/App.jsx`, and panels.
4. Wire in `applyTheme("<product-key>")`, `PatchHiveHeader`, `TabBar`, and `PatchHiveFooter`.
5. Add `docker-compose.yml`, `backend/Dockerfile`, and `frontend/Dockerfile`.
6. Update this file and `README.md` when the product becomes real.
