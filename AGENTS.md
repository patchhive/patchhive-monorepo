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
    product-shell/          @patchhivehq/product-shell shared frontend shell/auth helpers
    ai-local/               @patchhive/ai-local localhost AI gateway
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
  package.json              npm workspaces root
  README.md
  CLAUDE.md                 legacy Claude-oriented context file
  AGENTS.md                 Codex-oriented context file
```

## Tech Stack

Backend:
- Rust
- `axum`, `rusqlite`, `reqwest`, `tokio`, `serde`, `serde_json`, `chrono`, `uuid`, `anyhow`, `tracing`

Frontend:
- React + Vite
- No TypeScript in this repo currently
- Inline CSS only, using CSS variables from `@patchhivehq/ui`
- No CSS framework

AI provider integration:
- Direct HTTP via `reqwest`
- Preserve support for Anthropic, OpenAI, Gemini, Groq, and Ollama
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
- See [docs/platform-guardrails.md](/home/coemedia/Documents/code/patchhive/docs/platform-guardrails.md) and [docs/product-api-contract-v1.md](/home/coemedia/Documents/code/patchhive/docs/product-api-contract-v1.md).

## Shared UI Package

Location: `packages/ui/`

Every product frontend should import shared theme, primitives, layout shell, and reusable components from `@patchhivehq/ui`.

Rules:
- If a component will appear in 2 or more products, put it in `packages/ui/src/components/` and re-export it from `packages/ui/src/index.js`.
- If a component is product-specific, keep it inside that product.
- Product apps should call `applyTheme("<product-key>")` from `App.jsx`.

## Shared Product Shell Package

Location: `packages/product-shell/`

Every product frontend that uses PatchHive's API-key login flow should import shared auth/bootstrap behavior from `@patchhivehq/product-shell`.

Rules:
- If API-key login bootstrap is the same across 2 or more products, keep it in `product-shell`, not inside a product `App.jsx`.
- If authenticated backend `fetch` behavior is repeated across 2 or more products, keep it in `product-shell`.
- Avoid direct `localStorage` reads across individual panels when the app shell can pass the resolved API key down instead.

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
- Use `LoginPage` from `@patchhivehq/ui` with product-specific props
- Use `PatchHiveHeader`, `TabBar`, and `PatchHiveFooter`
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
    auth.rs
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

RepoReaper defaults:
- Backend: `http://localhost:8000`
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
- `WEBHOOK_SECRET`
- `REAPER_DB_PATH`
- `REAPER_WORK_DIR`

## Planned Products

- SignalHive: maintenance signal and backlog risk detection
- ReviewBee: turn PR review threads into actionable follow-up tasks
- TrustGate: evaluate risk in AI-generated diffs
- RepoMemory: durable repo memory for coding agents
- MergeKeeper: keep PRs mergeable
- FlakeSting: detect and explain flaky tests
- DepTriage: dependency update prioritization
- VulnTriage: rank security findings into engineering work
- RefactorScout: surface safe high-value refactors
- HiveCore: final unified PatchHive control plane / brain that connects the specialist products into one system

## SignalHive Notes

- SignalHive should stay visibility-first and read-only.
- Its job is to surface stale backlog risk, duplicate issues, recurring bug patterns, TODO/FIXME hotspots, and hidden maintenance drag before PatchHive starts changing code.
- SignalHive is the trust-building reconnaissance layer that should make the later autonomous products feel earned rather than abrupt.
- The MVP should stay simple: GitHub issue sync, stale and duplicate heuristics, recurring bug clustering, marker scanning, priority scoring, and a basic web view.
- Scan presets are worth supporting early because they make repeated operator workflows sticky without changing SignalHive's read-only posture.
- SignalHive should respect allowlist, denylist, and opt-out controls early so autonomous repo discovery never feels invasive.
- The intended early audience is engineering leads and CTOs at small startups who need maintenance visibility before they are ready for autonomous repo changes.

## Key Decisions

- Rust backend and React frontend are deliberate and should stay consistent across products.
- Multi-provider AI support in RepoReaper is non-negotiable.
- No AI provider SDKs by default; prefer raw HTTP.
- SQLite only.
- HiveCore should become the orchestration and global-settings layer for the specialist products.
- Products should be buildable independently, but their APIs should converge toward shared contracts so HiveCore can coordinate them.
- Watch Mode is a UI toggle backed by SQLite settings.
- PatchHive should contribute under its own GitHub identity with explicit autonomous attribution.
- Allowlist, denylist, and opt-out controls should exist early anywhere PatchHive discovers work autonomously.
- Hard quality and rate limits should gate outbound PR creation so PatchHive's reputation compounds in the right direction.

## Git Conventions

- Branch names: `reaper/issue-{number}` for RepoReaper, similar pattern for other products
- PR bodies should include explicit autonomous attribution and end with `*ProductName by PatchHive*`
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
