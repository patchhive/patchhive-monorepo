# PatchHive Monorepo

The maintenance layer for modern codebases.

## Source Of Truth

This repository is the canonical PatchHive development monorepo.

- Shared packages are developed here first.
- Products are developed here first.
- Individual product repositories can be exported from this monorepo later for standalone distribution and visibility.
- Until that split happens, this repo is the source of truth for code, docs, and history.

The intended long-term flow is:

1. Build and evolve products in this monorepo.
2. Publish shared packages such as `@patchhivehq/ui`.
3. Export products such as `products/repo-reaper` into their own GitHub repositories when they are ready.

The export workflow is documented in [Product And Package Export Workflow](/home/coemedia/Documents/code/patchhive/docs/product-export-workflow.md).

## Structure

```
patchhive/
  packages/
    ui/                     ← @patchhivehq/ui — shared component library
      src/
        theme.js            ← base dark theme + per-product accent colors
        primitives.jsx      ← Btn, Input, Sel, ScoreBadge, ConfidenceBar,
                               PatchHiveHeader, PatchHiveFooter, TabBar, etc.
        components/
          AgentCard.jsx
          DiffViewer.jsx
          IssueRow.jsx
          LoginPage.jsx     ← accepts icon/title/storageKey/apiBase props
        index.js            ← re-exports everything
    product-shell/          ← @patchhivehq/product-shell — shared frontend auth/bootstrap helpers
      src/
        index.js            ← API-key auth hook + authenticated fetch helpers
    ai-local/               ← @patchhive/ai-local — localhost AI gateway
      src/
        index.js            ← gateway server + provider routing
        cli.js              ← local entrypoint
  crates/
    patchhive-product-core/ shared Rust auth + startup primitives
  products/
    repo-reaper/            ← RepoReaper v0.1.0
      backend/              ← Rust (axum, rusqlite, reqwest, tokio)
      frontend/             ← React (imports from @patchhivehq/ui)
    signal-hive/            ← SignalHive v0.1.0
      backend/              ← Rust read-only maintenance signal API
      frontend/             ← React signal dashboard
    trust-gate/             ← TrustGate v0.1.0
      backend/              ← Rust diff review and policy engine
      frontend/             ← React rule editor and review dashboard
```

## Adding a New Product

1. Copy `products/repo-reaper` as a template
2. Replace the backend with your product's logic
3. In `frontend/src/App.jsx`:
   - Call `applyTheme("signal-hive")` (or your product key)
   - Pass your product's `icon` and `title` to `LoginPage`, `PatchHiveHeader`, `PatchHiveFooter`
4. All shared UI — buttons, inputs, panels, colors — just work

## Shared UI Package (`@patchhivehq/ui`)

```js
import {
  // Theme
  applyTheme, PRODUCT_THEMES, PROVIDERS,

  // Primitives
  S, Btn, Input, Sel, Divider, EmptyState,
  ScoreBadge, ConfidenceBar, StatusDot, Tag, timeAgo,

  // Layout
  PatchHiveHeader, PatchHiveFooter, TabBar,

  // Components
  AgentCard, DiffViewer, IssueRow, LoginPage,
} from "@patchhivehq/ui";
```

## Shared Product Shell (`@patchhivehq/product-shell`)

```js
import {
  createApiFetcher,
  useApiKeyAuth,
} from "@patchhivehq/product-shell";
```

This package holds repeated frontend shell behaviors that already show up across multiple PatchHive products:

- API-key login bootstrap against `/auth/status` and `/auth/login`
- authenticated `fetch` helpers for product backends

## Shared Rust Product Core (`patchhive-product-core`)

`patchhive-product-core` is the first shared Rust crate for backend overlap that is already real across multiple PatchHive products.

It currently holds:

- API-key auth hashing, verification, persistence, and axum middleware
- typed startup checks and shared startup logging helpers

Standalone Rust product repositories should consume the shared crate from its own repo, while the monorepo uses `.cargo/config.toml` to patch that git dependency back to the local crate path during development.

Because that monorepo patch intentionally overrides a git dependency with a local path, strict `cargo check --locked` validation for product backends should live in the standalone repos. Inside the monorepo, use plain `cargo check` while iterating on shared Rust crate changes.

## Product Accent Colors

| Product        | Accent  |
|----------------|---------|
| repo-reaper    | Crimson |
| signal-hive    | Blue    |
| review-bee     | Amber   |
| trust-gate     | Purple  |
| repo-memory    | Green   |
| merge-keeper   | Blue    |
| flake-sting    | Orange  |
| dep-triage     | Amber   |
| vuln-triage    | Crimson |
| refactor-scout | Green   |

## Quick Start — RepoReaper

```bash
cd products/repo-reaper
cp .env.example .env
# fill in BOT_GITHUB_TOKEN, BOT_GITHUB_USER, PROVIDER_API_KEY

# Dev
cd backend && cargo run
cd frontend && npm install && npm run dev

# Docker
docker-compose up --build
```

## Quick Start — SignalHive

```bash
cd products/signal-hive
cp .env.example .env
# fill in BOT_GITHUB_TOKEN

# Dev
cd backend && cargo run
cd ../frontend && npm install && npm run dev

# Docker
docker-compose up --build
```

## Quick Start — TrustGate

```bash
cd products/trust-gate
cp .env.example .env

# Dev
cd backend && cargo run
cd ../frontend && npm install && npm run dev

# Docker
docker-compose up --build
```

## Local AI Gateway

PatchHive includes `@patchhive/ai-local`, a localhost AI gateway for products that should route through a user's own Codex or Copilot session.

```bash
npm install
npm run dev:ai-local
```

Point products at:

```bash
PATCHHIVE_AI_URL=http://127.0.0.1:8787/v1
```

RepoReaper uses `PATCHHIVE_AI_URL` first and falls back to `OPENAI_BASE_URL` for compatibility.

## Platform Docs

- [Platform Guardrails](/home/coemedia/Documents/code/patchhive/docs/platform-guardrails.md)
- [Product API Contract v1](/home/coemedia/Documents/code/patchhive/docs/product-api-contract-v1.md)

---

*PatchHive — visibility first, autonomy second.*
