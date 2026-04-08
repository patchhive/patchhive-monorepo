# CLAUDE.md — PatchHive Project Context

This file is read by Claude automatically when working in this repo.
It captures all architectural decisions, conventions, and build history so Claude has full context without needing re-explanation.

---

## What PatchHive Is

PatchHive is a software maintenance platform — a family of focused tools that help engineering teams find, prioritize, and automate the maintenance work that slows delivery.

**Three core principles:**
1. Maintenance work should be continuously visible
2. Automation should be constrained and reviewable
3. Trust should be earned through signal quality, not hype

**Builder:** Jeremy Coe (@coe0718) — independent developer, building PatchHive for personal use. If others use it, great. Not optimizing for growth or monetization.

---

## Monorepo Structure

```
patchhive/
  packages/
    ui/                     ← @patchhivehq/ui — shared React component library
    product-shell/          ← @patchhivehq/product-shell — shared frontend auth/bootstrap helpers
  products/
    repo-reaper/            ← ✅ BUILT (v0.1.0)
    signal-hive/            ← next
    review-bee/
    trust-gate/
    repo-memory/
    merge-keeper/
    flake-sting/
    dep-triage/
    vuln-triage/
    refactor-scout/
  package.json              ← npm workspaces root
  README.md
  CLAUDE.md                 ← you are here
```

---

## Tech Stack — Non-Negotiable

**Backend:** Rust — always. axum (HTTP), rusqlite (SQLite), reqwest (HTTP client), tokio (async), serde/serde_json, chrono, uuid, anyhow, tracing.

**Frontend:** React + Vite — always. No TypeScript (not currently used). All styling is inline CSS using CSS variables from `@patchhivehq/ui`. No CSS frameworks.

**AI providers:** Direct HTTP via reqwest — no SDK dependencies. Supports Anthropic, OpenAI, Gemini, Groq, Ollama. Multi-provider is intentional and must be preserved in every product that calls AI.

**No separate ORM.** rusqlite used directly with raw SQL.

**Docker:** Every product gets a `docker-compose.yml`, `backend/Dockerfile`, `frontend/Dockerfile`.

---

## Shared UI Package — @patchhivehq/ui

Located at `packages/ui/`. Every product frontend imports from here.

```js
import {
  // Theme
  applyTheme,        // call with product key e.g. applyTheme("signal-hive")
  PRODUCT_THEMES,    // accent color map for all 10 products
  PROVIDERS,         // AI provider metadata (label, color, icon, keyHint)
  ROLE_META,         // agent role metadata (for RepoReaper — may not apply to all products)

  // Primitives
  S,                 // style constants (S.panel, S.field, S.label, S.input, S.select)
  Btn, Input, Sel,   // base form elements
  Divider, EmptyState,
  ScoreBadge, ConfidenceBar, StatusDot, Tag,
  timeAgo,

  // Layout — use these in every product for consistency
  PatchHiveHeader,   // sticky header with phase indicator, running dot
  PatchHiveFooter,   // "ProductName by PatchHive" footer
  TabBar,            // tab navigation

  // Components
  AgentCard,         // agent status card — RepoReaper specific but reusable
  DiffViewer,        // modal diff viewer
  IssueRow,          // GitHub issue row with score/confidence
  LoginPage,         // auth page — accepts icon, title, storageKey, apiBase props
} from "@patchhivehq/ui";
```

### Adding to the shared package

If a component will appear in 2+ products, it goes in `packages/ui/src/components/` and gets re-exported from `packages/ui/src/index.js`. If it's product-specific, it stays in the product's `src/components/` or `src/panels/`.

### Product accent colors

Each product has a pre-defined accent color in `packages/ui/src/theme.js`:

| Product         | Key              | Accent  |
|-----------------|------------------|---------|
| RepoReaper      | `repo-reaper`    | Crimson `#c41e3a` |
| SignalHive      | `signal-hive`    | Blue `#2a6aaa` |
| ReviewBee       | `review-bee`     | Amber `#c8922a` |
| TrustGate       | `trust-gate`     | Purple `#7b2d8b` |
| RepoMemory      | `repo-memory`    | Green `#2a8a4a` |
| MergeKeeper     | `merge-keeper`   | Blue `#2a6aaa` |
| FlakeSting      | `flake-sting`    | Orange `#c87020` |
| DepTriage       | `dep-triage`     | Amber `#c8922a` |
| VulnTriage      | `vuln-triage`    | Crimson `#c41e3a` |
| RefactorScout   | `refactor-scout` | Green `#2a8a4a` |

Apply in `App.jsx`: `useEffect(() => { applyTheme("signal-hive"); }, []);`

---

## Shared Product Shell Package — @patchhivehq/product-shell

Located at `packages/product-shell/`. Product frontends that use the shared PatchHive API-key auth flow should import from here instead of rebuilding the same bootstrap logic.

```js
import {
  createApiFetcher,
  useApiKeyAuth,
} from "@patchhivehq/product-shell";
```

Use it for:
- `/auth/status` and `/auth/login` bootstrap
- authenticated backend `fetch` helpers
- any repeated frontend shell behavior that appears in 2+ PatchHive products and is not purely visual

---

## Product Frontend Convention

Every product frontend follows this pattern:

```
products/<name>/frontend/
  src/
    App.jsx          ← main app, uses PatchHiveHeader/Footer/TabBar from @patchhivehq/ui
    config.js        ← exports API url (VITE_API_URL or localhost default)
    main.jsx         ← ReactDOM.createRoot boilerplate
    panels/          ← one file per tab panel
    components/      ← product-specific components (if any)
  index.html
  package.json       ← depends on @patchhivehq/ui as a published package
  vite.config.js
  Dockerfile
  nginx.conf
```

`config.js` always looks like:
```js
export const API = import.meta.env.VITE_API_URL || "http://localhost:8000";
```

`App.jsx` always:
1. Calls `applyTheme("product-key")` in a `useEffect`
2. Uses `LoginPage` from `@patchhivehq/ui` with product-specific props
3. Uses `PatchHiveHeader`, `TabBar`, `PatchHiveFooter` for layout shell
4. Imports panels from `./panels/`
5. Imports shared components from `@patchhivehq/ui`

---

## Product Backend Convention

Every product backend follows this structure:

```
products/<name>/backend/
  src/
    main.rs          ← axum app setup, startup, background tasks
    state.rs         ← AppState struct (shared via Arc/RwLock)
    db.rs            ← rusqlite init, all DB operations
    agents.rs        ← multi-provider AI calls (if product uses AI)
    github.rs        ← GitHub API calls (if product uses GitHub)
    git_ops.rs       ← git clone/branch/push/patch (if product touches code)
    auth.rs          ← API key auth middleware
    startup.rs       ← config validation, background loops
    pipeline.rs      ← main SSE endpoint(s)
    fix_worker.rs    ← per-item async worker (if applicable)
    routes/
      mod.rs
      config.rs      ← /config, /agents, /presets, /repo-lists, etc.
      history.rs     ← /history, /leaderboard, /pr-tracking, etc.
      webhook.rs     ← /webhook/github, /schedules
  Cargo.toml
  Dockerfile
```

Standard `Cargo.toml` dependencies:
```toml
axum = { version = "0.7", features = ["macros"] }
tokio = { version = "1", features = ["full"] }
tokio-stream = { version = "0.1", features = ["sync"] }
tower-http = { version = "0.5", features = ["cors"] }
rusqlite = { version = "0.31", features = ["bundled"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
dotenvy = "0.15"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
hmac = "0.12"
sha2 = "0.10"
hex = "0.4"
futures = "0.3"
async-stream = "0.3"
once_cell = "1"
```

---

## HiveCore

HiveCore is NOT a backend service. It is the unified frontend dashboard that will connect all PatchHive products into one UI. It will be built last, after all individual products exist. It is `products/hive-core/` when that time comes.

---

## Products Built

### ✅ RepoReaper (`products/repo-reaper/`) — v0.1.0

**Pitch:** Resolve selected repository issues automatically and open validated pull requests.

**What it does:** Hunts GitHub repos for open bug issues, scores them for fixability, generates patches via AI agents, reviews and refines them, runs tests, opens PRs.

**Agent roles (RepoReaper-specific):**
- Scout `◎` — hunts repos and scores issue fixability
- Judge `⚖` — selects relevant files to target
- Reaper `⚔` — generates the patch
- Smith `⬢` — reviews and refines the patch, can reject low-confidence work
- Gatekeeper `🔒` — runs tests and opens the PR

**Key features:**
- Multi-provider AI (Anthropic, OpenAI, Gemini, Groq, Ollama)
- Confidence scoring per patch (0-100), surfaced prominently in UI
- Rejected patches log with Smith feedback and reasoning
- Self-healing patch apply (retries on git apply failure)
- Configurable test retry count per run
- Watch Mode — webhook triggers auto-hunt on new "bug" labeled issues
- Dry Stalk mode — preview without making any changes
- Team presets (top-level tab, prominent)
- Cost tracking — per-run and lifetime across all providers
- PR Monitor — tracks all opened PRs, auto-cleans merged branches
- "by PatchHive" in footer and all PR bodies

**Backend:** `localhost:8000`
**Frontend:** `localhost:5173`
**DB:** SQLite (`repo-reaper.db`)
**Work dir:** `/tmp/repo-reaper` (temp clones, auto-cleaned)

**Key env vars:**
```
BOT_GITHUB_TOKEN    GitHub PAT (repo + PR scopes)
BOT_GITHUB_USER     Bot GitHub username
BOT_GITHUB_EMAIL    Bot commit email
PROVIDER_API_KEY    Global AI key (agents can override per-agent)
OLLAMA_BASE_URL     Ollama endpoint (default: http://localhost:11434)
COST_BUDGET_USD     Run budget cap (0 = unlimited)
MIN_REVIEW_CONFIDENCE  Smith confidence threshold (default: 40)
RETRY_COUNT         Test retry attempts (default: 3)
WEBHOOK_SECRET      GitHub webhook HMAC secret
REAPER_DB_PATH      SQLite path (default: repo-reaper.db)
REAPER_WORK_DIR     Temp clone dir (default: /tmp/repo-reaper)
```

---

## Products Remaining (planned build order — but Jeremy builds in whatever order sounds fun)

### 2. SignalHive
**Pitch:** Find the maintenance work your team is missing before it turns into drag or downtime.
**What it does:** Scans repo, issue history, and code comments to surface stale issues, duplicate reports, TODO/FIXME clusters, recurring bugs, and hidden backlog risk. Produces a ranked maintenance queue.
**Target:** Engineering leads at startups with 3-15 engineers.
**MVP scope:** GitHub issue sync, stale/duplicate heuristics, TODO/FIXME scanner, priority score, web view.
**Accent:** Blue `#2a6aaa`
**Note:** Read-only — no write access, no code changes. Pure signal.

### 3. ReviewBee
**Pitch:** Close PR review threads faster by turning reviewer comments into concrete follow-up tasks.
**What it does:** Reads PR review comments, clusters actionable feedback, generates a checklist for the author.
**MVP scope:** Parse PR review comments, cluster actionable feedback, generate checklist, mark resolved items.
**Accent:** Amber `#c8922a`

### 4. TrustGate
**Pitch:** Decide whether AI-generated code changes should be blocked, warned on, or approved.
**What it does:** Reviews AI-generated diffs against repo-specific risk rules. Returns `safe/warn/block`.
**MVP scope:** PR diff ingestion, risk scoring, policy rules, GitHub status check.
**Accent:** Purple `#7b2d8b`

### 5. RepoMemory
**Pitch:** Give coding agents memory of how your repo actually works.
**What it does:** Builds a durable knowledge layer from merged PRs, past bugs, reviewer feedback, conventions. Generates prompt-packs for agents.
**MVP scope:** Ingest merged PRs, extract conventions/failures, searchable memory store, prompt-pack generation.
**Accent:** Green `#2a8a4a`

### 6. MergeKeeper
**Pitch:** Keep pull requests mergeable by catching stale branches, conflicts, and failing checks early.
**MVP scope:** Detect stale branches, failed checks, merge conflicts, rerun/rebase suggestions.
**Accent:** Blue `#2a6aaa`

### 7. FlakeSting
**Pitch:** Detect, isolate, and explain flaky tests before they destroy trust in CI.
**MVP scope:** Parse CI results, identify inconsistent failures across runs, rank flaky tests, emit suspect/quarantine report.
**Accent:** Orange `#c87020`

### 8. DepTriage
**Pitch:** Tell teams which dependency updates matter now and which can wait.
**MVP scope:** Ingest dependency PRs and advisories, score by risk/impact, collapse duplicates, weekly triage report.
**Accent:** Amber `#c8922a`

### 9. VulnTriage
**Pitch:** Turn security findings into ranked, actionable engineering work.
**MVP scope:** Pull GitHub/code scanning alerts, map to owners/files, rank by exploitability, create issues or reports.
**Accent:** Crimson `#c41e3a`

### 10. RefactorScout
**Pitch:** Surface the safest high-value refactors your team can make this week.
**MVP scope:** Complexity hotspots, dead-code hints, duplicate-code detection, safe refactor recommendations.
**Accent:** Green `#2a8a4a`

### 11. HiveCore (LAST)
**Pitch:** One command center for all PatchHive products.
**What it is:** Unified frontend dashboard connecting all products. Not a backend service.
**Build:** After all individual products exist.

---

## Key Decisions & Rationale

- **Rust backend, React frontend:** Jeremy wants majority of dev under one language. Rust for all backends, React only where JS is unavoidable (frontend).
- **No Codex:** All code is written by Claude. No other AI coding tools in the loop for PatchHive.
- **Build for self first:** PatchHive is built for Jeremy's own use. Community adoption is a bonus, not the goal.
- **No SDK dependencies for AI:** All provider calls are raw reqwest HTTP. Avoids SDK drift, gives full control.
- **SQLite only:** No Postgres, no Redis. Single-binary simplicity.
- **HiveCore is a frontend, not a backend:** It connects products, it doesn't run them.
- **Multi-provider is non-negotiable in RepoReaper:** Jeremy added it intentionally. Preserve it.
- **Watch Mode is toggle-based:** Enabled/disabled via UI toggle in Webhooks panel. Stored in SQLite settings table.

---

## Git Conventions (expected)

- Branch names: `reaper/issue-{number}` (RepoReaper), follow same pattern per product
- PR bodies always end with `*ProductName by PatchHive*`
- Commit messages: `fix: {issue title} (closes #{number})`

---

## Running Locally

```bash
# RepoReaper backend
cd products/repo-reaper/backend
cargo run

# RepoReaper frontend
cd products/repo-reaper/frontend
npm install
npm run dev

# Docker (all-in-one)
cd products/repo-reaper
docker-compose up --build
```

---

## When Starting a New Product

1. Create `products/<name>/` directory
2. Copy `products/repo-reaper/backend/` as a starting point, strip what's not needed
3. Create `products/<name>/frontend/` with:
   - `package.json` pointing to `@patchhivehq/ui`
   - `src/config.js` with the API url
   - `src/App.jsx` calling `applyTheme("<product-key>")`
   - `PatchHiveHeader`, `PatchHiveFooter`, `TabBar` for layout shell
4. Add `docker-compose.yml`, `backend/Dockerfile`, `frontend/Dockerfile`
5. Add product to this file under "Products Built"
6. Update status in the product list above

---

*Last updated: April 2026*
*Built by Jeremy Coe | github.com/coe0718*
