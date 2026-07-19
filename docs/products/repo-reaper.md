# RepoReaper by PatchHive

> **Autonomous bug-fixing agent.** RepoReaper hunts open GitHub issues, generates patches, validates them, and opens pull requests — entirely by machine.

| Attribute | Value |
|-----------|-------|
| **Product Role** | Outbound autonomous contribution — find, fix, validate, and open PRs |
| **Status** | Active development |
| **Standalone Repository** | [`patchhive/reporeaper`](https://github.com/patchhive/reporeaper) (mirror of this directory) |
| **Local Port** | `8000` |

---

## Product Role

RepoReaper is PatchHive's outbound contribution product. It is a **multi-agent system** that:

1. **Discovers** open issues across GitHub repos, or hunts inside a supplied target repo (filtered by language, stars, labels, and custom allow/deny/opt-out lists)
2. **Scores** issues for fixability using an AI Scout agent
3. **Fork + clone** each target repo
4. **Selects** relevant code via a Judge agent
5. **Generates** a patch via a Reaper agent
6. **Reviews** the patch via a Smith agent
7. **Runs** tests in a sandboxed Docker environment
8. **Publishes** a PR back to the original repo via a Gatekeeper agent

RepoReaper is the **only current PatchHive product that writes code and opens pull requests**. It should be the last step in the early suite loop, after SignalHive and TrustGate have made candidate work visible.

RepoReaper's agent team is also the seed implementation of the shared PatchHive
Squad pattern. Future AI-capable products should reuse the shared Squad
substrate for provider/model setup, encrypted per-agent secrets, presets,
readiness, and HiveCore visibility instead of copying RepoReaper's team builder.
See [Shared Squad architecture](../shared-squad-architecture.md).

---

## Core Workflow

```
┌──────────────┐     ┌──────────┐     ┌───────────┐     ┌──────────┐     ┌────────────┐
│   Scout      │ ──► │  Judge   │ ──► │  Reaper   │ ──► │  Smith   │ ──► │ Gatekeeper │
│ (discover +  │     │ (select  │     │ (generate │     │ (review  │     │ (test + PR │
│  score)      │     │  files)  │     │  patch)   │     │ + refine)│     │  publish)  │
└──────────────┘     └──────────┘     └───────────┘     └──────────┘     └────────────┘
```

### Phase 1 — Scan (Scout)
- If `target_repo` is supplied, reads that exact `owner/repo` and collects matching open issues there
- If `target_repo` is blank, queries GitHub search API (`GET /search/repositories`) with language, star minimum, and optional search query
- Applies allowlist / denylist / opt-out filters from the `repo_lists` table
- For each repo, collects up to 5 open issues matching the configured labels
- AI-scores each issue (0–100) for fixability
- Emits SSE events: `phase: scan`, `repos`, `issues`

### Phase 2 — Triage
- Prioritises issues by score, up to `max_issues` per run
- Emits `phase: triage`

### Phase 3 — Fix (Reaper / Judge / Smith / Gatekeeper)
- For each issue, concurrently (limited by a semaphore of `concurrency`):
  1. **Fork** the upstream repo via GitHub API
  2. **Clone** (depth=10) using `GIT_ASKPASS` with the bot token and no ambient Git credential helper
  3. **Branch** (`reaper/issue-{N}`)
  4. **Comment** on the issue announcing the hunt
  5. **Judge** selects relevant files by analysing the repo structure
  6. **Reaper** generates a unified diff patch via AI
  7. **Self-heal**: if `git apply --check` fails, the Reaper retries with the error context
  8. **Smith** reviews the patch — if confidence < `MIN_REVIEW_CONFIDENCE`, the patch is rejected (and optionally queued as a FailGuard candidate in RepoMemory)
  9. **Gatekeeper** runs tests in a sandboxed Docker container (pytest, cargo, go, npm) with up to `retry_count` retries
  10. **Gatekeeper** commits + pushes the branch with the explicit bot token and opens a PR (draft if tests are not run or fail, full PR if they pass)

### Phase 4 — Finalize
- Records the run summary: total fixed, attempted, cost
- Emits `phase: done`
- Resets `runActive` flag

### Example Run Request

```json
POST /run
X-API-Key: rr-...

{
  "target_repo": "",
  "search_query": "",
  "language": "python",
  "min_stars": 100,
  "max_repos": 10,
  "max_issues": 5,
  "labels": ["bug"],
  "concurrency": 3,
  "retry_count": 2,
  "cost_budget_usd": 0.50
}
```

### Example Dry Run Request

```json
POST /dry-run
X-API-Key: rr-...

{
  "target_repo": "",
  "search_query": "topic:machine-learning language:python stars:>500 is:public",
  "language": "python",
  "min_stars": 500,
  "max_repos": 5,
  "max_issues": 3,
  "labels": ["bug", "help wanted"]
}
```

### Response (SSE Stream)

Run endpoints (`/run`, `/dry-run`) return `text/event-stream` with these event types:

| Event | Data | Phase |
|-------|------|-------|
| `phase` | `{"phase":"scan"}` | Start |
| `log` | `{"msg":"...","type":"info\|success\|warn\|error"}` | Any |
| `agent_status` | `{"agent_id":"...","status":"idle\|working","task":"..."}` | Any |
| `agent_log` | `{"agent_id":"...","agent":"...","role":"...","msg":"...","type":"...","ts":"..."}` | Any |
| `repos` | `{"repos":[{"id":...,"full_name":"...","stars":...,...}]}` | Scan |
| `issues` | `{"issues":[...]}` | Scan/Triage |
| `issue_assign` | `{"id":...,"score":...,"reaper":"...","judge":"...","smith":"...","gatekeeper":"..."}` | Fix |
| `issue_confidence` | `{"id":...,"confidence":...}` | Fix |
| `issue_result` | `{"id":...,"status":"fixed\|skipped\|rejected\|error","pr":{...}}` | Fix |
| `cost_update` | `{"run_cost":...,"lifetime_cost":...}` | Fix |
| `error` | `{"msg":"..."}` | Any |
| `done` | `{"total_fixed":...,"total_attempted":...,"run_id":"...","cost":...}` | Finalize |

---

## Inputs

### RunRequest Schema

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `target_repo` | string | `""` | Optional directed repo target in `owner/repo` format. When present, RepoReaper hunts issues only inside that repo. When blank, RepoReaper uses autonomous repo discovery. |
| `search_query` | string | `""` | GitHub search query override (defaults to `topic:machine-learning language:{lang} stars:>{min_stars} is:public`) |
| `language` | string | `"python"` | Programming language filter |
| `min_stars` | u32 | `100` | Minimum repo stars |
| `max_repos` | u32 | `10` | Max repos to scan |
| `max_issues` | u32 | `5` | Max issues to process |
| `labels` | string[] | `["bug"]` | GitHub issue labels to filter by |
| `concurrency` | u32 | `3` | Max concurrent fix jobs |
| `retry_count` | u32 | `2` | Patch/test retry attempts per issue |
| `cost_budget_usd` | f64 | `0.0` | Run cost cap (0 = uncapped) |

### Repo Filters (via API)

Three list types, stored in the `repo_lists` SQLite table:

- **allowlist** — only scan these repos (overrides search)
- **denylist** / **blocklist** — exclude specific repos
- **opt_out** / **opt-out** / **optout** — respect upstream non-consent

### Directed and Autonomous Modes

RepoReaper supports both operating modes described in [Product operating modes](../product-operating-modes.md):

- **Directed mode:** set `target_repo` to `owner/repo`. RepoReaper reads that repository, collects matching open issues, scores them, and then proceeds through Dry Stalk or the guarded write path.
- **Autonomous mode:** leave `target_repo` blank. RepoReaper discovers repositories from `search_query`, `language`, `min_stars`, `labels`, and `max_repos`.

Both modes honor allowlist, denylist, and opt-out controls. A malformed `target_repo` stops discovery for that run instead of falling back to broad autonomous search.

Run history keeps these modes visible:

- Dry Stalk creates a saved `dry_run=true` run with candidate and Scout-report evidence.
- The Dry Stalk tab shows recent no-write previews and can reload their saved candidate queue.
- The Run History tab is reserved for guarded, write-capable hunts.
- Saved runs expose `run_style` as `targeted` or `autonomous`.
- Targeted runs expose `target_repo`.

---

## Outputs

- **Pull requests** opened on target repos with full attribution:
  - Commit message: `fix: {title} (closes #{N})`
  - PR body includes: Reaper confidence, fixability score, files changed, test results, agent attribution, and `Closes #{N}`
  - Draft PR if tests fail, full PR if tests pass
- **Run history** persisted in SQLite
- **Rejected patches** stored with Smith feedback for later review
- **SSE stream** for real-time UI updates

---

## Safety Boundary

- **Auth-gated**: API key (`rr-...`) required for all endpoints except `/health`, `/auth/*`, `/startup/checks`, `/capabilities`, and `/webhook/github`
- **Bootstrap**: key generation is localhost-only
- **Tests opt-in**: untrusted test execution is disabled by default (`REAPER_ENABLE_UNTRUSTED_TESTS` must be `true`); a repository trusted by configured HiveCore policy may enter the Docker sandbox without that global opt-in
- **Sandboxed execution**: Docker sandbox with `--network none`, `--cap-drop ALL`, `--pids-limit 256`, `--memory 2g`, `--cpus 2`
- **Host execution**: requires two env vars (`REAPER_ENABLE_UNTRUSTED_TESTS=true` + `REAPER_ALLOW_HOST_TESTS=true`)
- **Test timeout**: configurable via `REAPER_TEST_TIMEOUT_SECONDS` (default: 600s)
- **Cost budget**: `COST_BUDGET_USD` caps AI spend per run (0 = uncapped, with a warning)
- **Concurrency limiting**: Semaphore-based; configurable per run
- **Only one run at a time**: `runActive` atomic flag prevents concurrent hunts
- **Suite policy and PR capacity**: when `PATCHHIVE_HIVECORE_URL` is configured, RepoReaper fails closed on unavailable repository policy, reserves both its product slot and suite-wide slot immediately before PR creation, and releases capacity on failed creation or observed PR closure/merge
- **Existing PR guard**: before starting patch generation, RepoReaper checks the
  GitHub issue timeline for open pull requests already linked to the issue. If
  one exists, RepoReaper records a held attempt and does not open a competing PR.
  If the linked-PR check cannot be verified, RepoReaper holds instead of risking
  duplicate maintainer noise.
- **File system confinement**: `collect_files_selective` validates that all paths stay within the repo root; any path that canonicalizes outside is silently skipped
- **No secrets in output**: API keys are masked in config responses
- **Temporary work directories**: cleaned up after each attempt

---

## Local Development

### Docker (full stack)

```bash
cd products/repo-reaper
cp .env.example .env
docker compose up --build
```

| Service | URL |
|---------|-----|
| Backend | `http://localhost:8000` |
| Frontend | `http://localhost:5173` |
| Frontend v2 | `http://localhost:5195` |

Backend: `http://localhost:8000`
Frontend: `http://localhost:5173`

### Frontend v2 Temporary Scope

The v2 prototype has a deliberately lightweight agent-team setup. It exists so
Mission Deck and Dry Stalk can be tested honestly through gateway mode without
pulling the entire old frontend team builder into the new shell.

Current v2 behavior:

- Checks can recruit a starter team and edit the backend's current in-memory
  agent team through `/agents`.
- Checks includes provider defaults for the active team, so custom
  OpenAI-compatible base URLs, provider keys, and optional bot overrides can be
  entered once and applied to every agent instead of pasted into each role.
- Provider defaults can pull the current model list through RepoReaper's
  `/models/:provider` endpoint while keeping manual model entry available for
  custom providers with incomplete model-list support. The shared picker filters
  obvious non-text models such as embeddings, rerankers, STT/TTS/audio,
  image/video, moderation, and provider utility entries from noisy catalogs.
  Free provider models remain visible by default, and RepoReaper v2 can narrow
  the picker to free-marked model IDs when the operator enables Free only.
- Provider defaults can also test the selected model through
  `/models/:provider/test`, which sends a tiny prompt through RepoReaper's real
  provider runtime and reports auth, rate limit, timeout, or provider errors.
- Mission Deck and Dry Stalk are gated until an agent team exists, so the UI no
  longer lets a run fail with a bare `No agents configured` backend error.
- RepoReaper persists the active team and saved team presets in SQLite.
  Per-agent API keys and bot token overrides are encrypted at rest when
  `REAPER_ENCRYPTION_KEY` or `PATCHHIVE_ENCRYPTION_KEY` is set. Without one of
  those keys, secret fields stay memory-only and are not written to SQLite.
  When an encryption key is added later, RepoReaper migrates existing plaintext
  active-team and preset secret fields on boot.
- Dry Stalk remains no-write, but it still needs at least a Scout agent because
  issue scoring and dry-run analysis use the AI agent pipeline.

Deferred until the RepoReaper unified-backend pass:

- full provider/model discovery parity with the old team builder
- full team preset save/load/delete management in v2
- richer per-agent controls, cooldown visibility, and live agent logs
- HiveCore-driven setup for RepoReaper agent teams, approvals, and write gates

RepoReaper polish backlog after the first live sandbox PR tests:

- Keep strengthening the History tab as the operator's first diagnostic surface.
  It should show where each attempt stopped: Judge/context selection, Reaper
  no-patch decision, patch apply/self-heal, Smith review, validation, GitHub
  branch/fork/PR delivery, or final PR tracking.
- Mirror the most important no-patch, patch-error, and no-changes reasons on
  the Mission Deck so the operator does not have to open History for the basic
  answer to "why did this not open a PR?"
- Add links from attempts to the GitHub issue status comments and PR comments
  RepoReaper posted. The current UI links to the issue and PR, but does not
  persist individual comment URLs.
- Persist a compact per-run log artifact instead of relying on stdout, live SSE
  messages, or UI summaries when debugging a failed run later. Use the shared
  `ProductRunEvent` / `ProductRunArtifact` contract so History can show Judge,
  Reaper, apply, Smith, validation, branch, fork, PR, and issue-comment events
  without inventing a RepoReaper-only format.
- Surface provider/runtime failures more clearly, including rejected API keys,
  empty model responses, rate limits, timeouts, and model output that cannot be
  parsed into a patch.
- Standardize the suite-wide write/test policy. RepoReaper already treats
  disabled untrusted tests as "not proven" rather than a failure worth retrying,
  but future write-capable products need the same distinction between
  untrusted-disabled, sandboxed-passed, sandboxed-failed, and host-approved test
  execution.
- Keep draft PR behavior conservative until HiveCore owns write approvals and
  MergeKeeper/TrustGate can participate in the final gate.

Do not remove the old RepoReaper team/preset UI until the v2 replacement and the
unified backend/HiveCore setup path cover those workflows.

### Split Backend + Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
cd ../frontend-v2 && npm install && npm run dev
```

### Environment

```bash
# Required
REPO_REAPER_GITHUB_TOKEN_RW=ghp_...           # Classic PAT: Metadata(ro), Contents(rw), Issues(rw), Pull requests(rw)
BOT_GITHUB_USER=patchhive-bot

# Git identity
BOT_GITHUB_EMAIL=bot@patchhive.dev

# AI — choose one of:
PROVIDER_API_KEY=sk-...             # Direct provider key (global fallback)
PATCHHIVE_AI_URL=http://...         # OpenAI-compatible local gateway (e.g. @patchhive/ai-local)
OLLAMA_BASE_URL=http://localhost:11434

# Optional
COST_BUDGET_USD=0.50                # Run cost cap
MIN_REVIEW_CONFIDENCE=40            # Minimum Smith confidence (0–100)
RETRY_COUNT=2                       # Patch retry attempts
WEBHOOK_SECRET=whsec_...            # GitHub webhook secret for watch-mode
REAPER_API_KEY_HASH=...             # Pre-seeded auth hash
REAPER_SERVICE_TOKEN_HASH=...       # Service-token hash for HiveCore
REAPER_ENCRYPTION_KEY=...           # Encrypts saved active-team secrets
REAPER_DB_PATH=/tmp/repo-reaper.db
REAPER_WORK_DIR=/tmp/repo-reaper
REAPER_PORT=8000
REAPER_ENABLE_UNTRUSTED_TESTS=false
REAPER_TEST_SANDBOX=docker          # docker | host
REAPER_ALLOW_HOST_TESTS=false
REAPER_TEST_TIMEOUT_SECONDS=600

# RepoMemory integration (optional)
PATCHHIVE_REPO_MEMORY_URL=http://...
PATCHHIVE_REPO_MEMORY_API_KEY=...
```

---

## Configuration (Environment Variables)

| Variable | Required | Default | Purpose |
|----------|----------|---------|---------|
| `REPO_REAPER_GITHUB_TOKEN_RW` | ✅ | — | PAT for repo discovery, clone, push, PR creation |
| `BOT_GITHUB_USER` | ✅ | — | Git commit author |
| `BOT_GITHUB_EMAIL` | ❌ | — | Git commit email |
| `PROVIDER_API_KEY` | ❌ | — | Fallback AI provider API key |
| `PATCHHIVE_AI_URL` | ❌ | — | OpenAI-compatible local AI gateway |
| `OLLAMA_BASE_URL` | ❌ | `http://localhost:11434` | Ollama endpoint |
| `ANTHROPIC_API_KEY` | ❌ | — | Provider-specific API key |
| `OPENAI_API_KEY` | ❌ | — | Provider-specific API key |
| `GEMINI_API_KEY` | ❌ | — | Provider-specific API key |
| `GROQ_API_KEY` | ❌ | — | Provider-specific API key |
| `CUSTOM_AI_API_KEY` | ❌ | — | Provider-specific API key |
| `CUSTOM_AI_BASE_URL` | ❌ | — | Provider-specific base URL |
| `COST_BUDGET_USD` | ❌ | `0` (uncapped) | Per-run AI cost cap |
| `MIN_REVIEW_CONFIDENCE` | ❌ | `40` | Min Smith confidence to proceed |
| `RETRY_COUNT` | ❌ | `2` | Patch/test retries per issue |
| `REAPER_MAX_ACTIVE_WORKERS` | ❌ | `3` | Process-wide cap shared by manual, scheduled, webhook, patch, and test workers |
| `WEBHOOK_SECRET` | ❌ | — | GitHub webhook secret |
| `REAPER_API_KEY_HASH` | ❌ | — | Pre-seeded auth hash (else UI generates) |
| `REAPER_SERVICE_TOKEN_HASH` | ❌ | — | Service-token hash for HiveCore |
| `REAPER_ENCRYPTION_KEY` | ❌ | — | Encrypt active-team API keys and bot token overrides in SQLite |
| `PATCHHIVE_ENCRYPTION_KEY` | ❌ | — | Suite-wide fallback encryption key used when `REAPER_ENCRYPTION_KEY` is unset |
| `REAPER_DB_PATH` | ❌ | `./reaper.db` | SQLite database path |
| `REAPER_WORK_DIR` | ❌ | `/tmp/repo-reaper` | Clone workspace |
| `REAPER_PORT` | ❌ | `8000` | HTTP listen port |
| `REAPER_ENABLE_UNTRUSTED_TESTS` | ❌ | `false` | Enable test execution |
| `REAPER_TEST_SANDBOX` | ❌ | `docker` | Test sandbox mode |
| `REAPER_ALLOW_HOST_TESTS` | ❌ | `false` | Allow host test execution |
| `REAPER_TEST_TIMEOUT_SECONDS` | ❌ | `600` | Test timeout |
| `PATCHHIVE_REPO_MEMORY_URL` | ❌ | — | RepoMemory API endpoint |
| `PATCHHIVE_REPO_MEMORY_API_KEY` | ❌ | — | RepoMemory API key |

Generate `REAPER_ENCRYPTION_KEY` or `PATCHHIVE_ENCRYPTION_KEY` with
`openssl rand -hex 32`. Startup checks reject short values and obvious
placeholders; retain the same key across restarts so persisted agent credentials
remain readable.

---

## Technical Architecture

### Module Tree

```
backend/src/
├── main.rs                  # Entry point, router, auth endpoints, health
├── pipeline.rs              # Run orchestration: discover, execute_run, run_fix_wave, dry_run
├── db.rs                    # SQLite persistence (runs, issue_attempts, pr_tracking, etc.)
├── github.rs                # GitHub API client (search, issues, forks, PRs, webhooks)
├── git_ops.rs               # Git operations (clone, branch, commit, push, apply_patch, run_tests)
├── agents.rs                # AI agent coordination (score, select_files, generate_patch, smith, retry)
├── ai_local.rs              # PatchHive AI local gateway client
├── startup.rs               # Config validation and health checks
├── state.rs                 # AppState, AgentConfig, AgentStats types
├── fix_worker.rs            # Fix worker module (re-exports)
├── fix_worker/
│   ├── orchestrate.rs       # fix_one — per-issue orchestrator
│   ├── context.rs           # Clone, code selection, enriched context loading
│   ├── memory.rs            # RepoMemory context building, FailGuard candidate submission
│   ├── patch.rs             # Patch self-heal, PR publishing
│   ├── sse.rs               # SSE event helpers (alog, astatus, sse_ev)
│   └── types.rs             # Shared types, agent selection, scope builders
└── routes/
    ├── mod.rs               # Route re-exports
    ├── config.rs            # /config, /agents, /presets, /repo-lists, /cooldowns, /watch-mode
    ├── history.rs           # /history, /runs, /diff, /leaderboard, /rejected, /pr-tracking
    └── webhook.rs           # /webhook/github, /schedules
```

### Dependencies

| Package | Purpose |
|---------|---------|
| `axum` | HTTP server & router |
| `tokio` | Async runtime, mpsc channels, semaphore |
| `rusqlite` | SQLite database |
| `reqwest` | HTTP client for GitHub API & AI providers |
| `serde` / `serde_json` | Serialization |
| `patchhive_product_core` | Shared PatchHive primitives (auth, startup, rate-limit, RepoMemory client) |
| `patchhive_github_pr` | GitHub token resolution, webhook signature verification |
| `chrono` / `uuid` | Timestamps, IDs |
| `tracing` / `tracing-subscriber` | Structured logging |
| `dotenvy` | `.env` file loading |
| `once_cell` | Static startup checks |
| `anyhow` | Error handling |

### Agent Model

| Role | Icon | Color | Responsibility |
|------|------|-------|----------------|
| Scout | ◎ | `#4a9af0` | Search repos, collect issues, score fixability |
| Judge | ⚖ | `#e0a030` | Analyse repo structure, select relevant files for the patch |
| Reaper | ⚔ | `#c41e3a` | Generate the initial unified-diff patch |
| Smith | ⬢ | `#7b2d8b` | Review the patch, provide feedback, optionally refine |
| Gatekeeper | 🔒 | `#2a8a4a` | Run tests, commit & push, open PR |

Agents are configured via the `/agents` API endpoint. Each agent has:
- `id` (auto-generated UUID8 if empty)
- `name` (human-readable)
- `role` (one of the five above)
- `provider` (anthropic, openai, gemini, groq, custom, ollama)
- `model` (specific model name)
- `base_url`, `api_key`, `bot_token`, `bot_user` (per-agent overrides)
- `status` (idle/working)
- `stats` (fixed, skipped, errors, cost counters)

### Supported AI Providers & Models

| Provider | Models (static list) | Dynamic Discovery |
|----------|---------------------|-------------------|
| **Anthropic** | `claude-opus-4-6`, `claude-sonnet-4-6`, `claude-haiku-4-5`, `claude-sonnet-4-20250514` | `GET https://api.anthropic.com/v1/models` |
| **OpenAI** | `gpt-5.4`, `gpt-5.4-mini`, `gpt-5.4-nano`, `gpt-5.3-codex`, `gpt-5.2-codex`, `gpt-5.1`, `gpt-5-mini`, `gpt-5-nano`, `gpt-5.1-codex`, `gpt-5.1-codex-mini`, `gpt-5.1-codex-max`, `gpt-5-codex`, `gpt-5`, `gpt-4.1`, `gpt-4.1-mini`, `gpt-4.1-nano`, `o3`, `o4-mini`, `o3-mini` | `GET {base}/models` |
| **Gemini** | `gemini-2.0-flash`, `gemini-2.0-flash-lite`, `gemini-1.5-pro`, `gemini-2.5-pro` | `GET https://generativelanguage.googleapis.com/v1beta/models` |
| **Groq** | `llama-3.3-70b-versatile`, `llama-3.1-8b-instant`, `mixtral-8x7b-32768` | `GET {base}/models` |
| **Custom** | `gpt-4.1-mini`, `qwen2.5-coder`, `llama3.2` | `GET {base}/models` |
| **Ollama** | `llama3.2`, `codellama`, `deepseek-coder`, `qwen2.5-coder` | `GET {base}/api/tags` |
| **PatchHive AI** (local gateway) | — | Via `PATCHHIVE_AI_URL/status` |

**Key fallback chain**: Provider-specific API key → global `PROVIDER_API_KEY` → error. The `PATCHHIVE_AI_URL` gateway is tried first for OpenAI-compatible requests when no custom `base_url` or `api_key` is supplied.

### SSE Streaming

All long-running operations (run, dry-run) use **Server-Sent Events**:

```
GET /run          → text/event-stream
GET /dry-run      → text/event-stream
```

The `axum::response::sse::Sse` response wraps a `tokio::sync::mpsc::Receiver` with a `KeepAlive` every 15s. The client receives real-time `agent_log`, `agent_status`, `repos`, `issues`, `issue_result`, `cost_update`, and `done` events.

### Database (SQLite)

| Table | Purpose |
|-------|---------|
| `runs` | Run metadata (id, started_at, finished_at, total_fixed, total_attempted, total_cost_usd, status, config_json) |
| `issue_attempts` | Per-issue attempt tracking (run_id, issue_number, status, skip_reason, pr_url, pr_number, agent assignments, cost, confidence, patch_diff) |
| `pr_tracking` | PR lifecycle (pr_number, repo, state, merged, review_state, last_checked) |
| `scheduled_runs` | Cron-based run schedules (cron_expr, config_json, enabled, last_run, next_run) |
| `team_presets` | Saved agent team configurations |
| `repo_lists` | Allowlist/denylist/opt-out entries |
| `perf` | Agent performance metrics (agent_name, provider, model, role, outcome, cost) |
| `settings` | Key-value settings (watch_mode) |

### Startup Checks

On boot, `startup::validate_config` runs:

1. **`REPO_REAPER_GITHUB_TOKEN_RW`** — must be non-empty
2. **`BOT_GITHUB_USER`** — must be non-empty
3. **`PROVIDER_API_KEY`** — must be non-empty (unless using gateway)
4. **GitHub API** — `GET https://api.github.com/user` to validate token
5. **AI Gateway** — `GET {PATCHHIVE_AI_URL}/status` to discover ready providers
6. **Orphan runs** — any `runs` with status `running` are recovered to `orphaned`

---

## API Endpoints

### Authentication

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/auth/status` | Returns `{"auth_enabled": bool, "auth_configured": bool}` |
| `POST` | `/auth/login` | `{"api_key": "rr-..."}` → `{"ok": true}` (localhost-only bootstrap) |
| `POST` | `/auth/generate-key` | Generate first API key (localhost-only) |
| `POST` | `/auth/generate-service-token` | Generate HiveCore service token |
| `POST` | `/auth/rotate-service-token` | Rotate existing service token |

### System

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Health check (returns version, agent count, run status, DB status, config errors) |
| `GET` | `/startup/checks` | Startup check results |
| `GET` | `/capabilities` | Product capabilities contract (for HiveCore) |

### Runs (authenticated)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/run` | Start a full autonomous hunt (SSE stream) |
| `POST` | `/dry-run` | Dry run — discover & score only, no patches (SSE stream) |

### Configuration (authenticated)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/config` | Read config + provider models + roles + AI gateway status |
| `POST` | `/config` | Save env vars (token, API key, URL, budget, confidence) |
| `GET` | `/agents` | List configured agents + cooldowns |
| `POST` | `/agents` | Replace entire agent team |
| `DELETE` | `/agents/:id` | Remove an agent |
| `GET` | `/models/:provider` | List available models (static + dynamic) |
| `POST` | `/models/:provider` | Force model refresh with optional API key / base URL |
| `GET` | `/ai-local/status` | PatchHive AI gateway status |
| `GET` | `/presets` | List team presets |
| `POST` | `/presets` | Save a team preset |
| `DELETE` | `/presets/:name` | Delete a preset |
| `GET` | `/repo-lists` | List allowlist/denylist/opt-out entries |
| `POST` | `/repo-lists` | Add a repo to a list |
| `DELETE` | `/repo-lists/*repo` | Remove a repo from all lists |
| `GET` | `/cooldowns` | List provider cooldowns |
| `DELETE` | `/cooldowns/:provider` | Clear a provider's cooldown |
| `GET` | `/watch-mode` | Get watch mode state |
| `POST` | `/watch-mode` | Set watch mode (auto-trigger on webhook) |
| `GET` | `/stats/lifetime-cost` | Lifetime AI cost |

### History (authenticated)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/history` | Run history |
| `GET` | `/runs` | Detailed run list |
| `GET` | `/diff/:run_id/:issue_number` | Patch diff for a specific attempt |
| `GET` | `/leaderboard` | Agent performance leaderboard |
| `GET` | `/rejected` | List rejected patches |
| `GET` | `/pr-tracking` | PR lifecycle status |
| `GET` | `/github/rate-limit` | GitHub API rate limit status |

### Webhook & Scheduling (authenticated except webhook)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/webhook/github` | **Public** — GitHub webhook receiver (signature-verified) |
| `GET` | `/schedules` | List scheduled runs |
| `POST` | `/schedules` | Create/update a schedule (cron, config) |
| `DELETE` | `/schedules/:id` | Delete a schedule |

---

## Webhook Integration

RepoReaper accepts GitHub webhooks for **watch mode**:

1. Configure a webhook in your GitHub repo pointing to `POST /webhook/github`
2. Set `WEBHOOK_SECRET` to match the GitHub secret
3. Enable watch mode via `POST /watch-mode {"enabled": true}`

When a webhook arrives, RepoReaper verifies the signature using `patchhive_github_pr::verify_github_webhook_signature`. If watch mode is enabled and a run is not already active, it automatically triggers a new hunt with the last-used configuration.

### Scheduled Runs

Schedules are stored in the `scheduled_runs` table with a cron expression and JSON config. The scheduler runs on a 60-second tick. Three preset cron expressions are available:

| Preset | Expression |
|--------|-----------|
| `hourly` | `0 * * * *` |
| `nightly` | `0 2 * * *` |
| `weekly` | `0 3 * * 0` |

---

## Monitoring

| Endpoint | What It Provides |
|----------|-----------------|
| `GET /health` | Status, version, agent count, run active flag, watch mode, lifetime cost, auth state, config errors, DB path |
| `GET /startup/checks` | Detailed startup check results |
| `GET /github/rate-limit` | GitHub API rate limit consumption |
| `GET /stats/lifetime-cost` | Total AI cost across all runs |
| `GET /capabilities` | Product contract for HiveCore integration |

---

## Deployment

### Docker

```bash
# Build and run
docker compose up --build

# Or build just the backend
cd backend
docker build -t repo-reaper .
docker run -p 8000:8000 \
  -e REPO_REAPER_GITHUB_TOKEN_RW=... \
  -e BOT_GITHUB_USER=... \
  -e PROVIDER_API_KEY=... \
  -e REAPER_DB_PATH=/data/reaper.db \
  -v reaper-data:/data \
  repo-reaper
```

### Native

```bash
cd backend
cp .env.example .env
# Edit .env with your configuration
cargo run --release
```

---

## Troubleshooting

| Symptom | Likely Cause | Check |
|---------|-------------|-------|
| `No agents configured` | Agent team not set up via API | `GET /agents` |
| `A hunt is already active` | Only one run allowed at a time | Wait or restart |
| No issues found | Filters too restrictive or no matching repos | Check `GET /repo-lists` and search query |
| GitHub 403 / rate limit | Token lacks permissions or rate-limited | `GET /github/rate-limit` |
| `Apply failed` | Patch doesn't match cloned code | Check repo fork is up to date |
| Smith rejection | Confidence below threshold | Raise `MIN_REVIEW_CONFIDENCE` or review patch quality |
| Test failures | Environment issues or legit test breakage | Check Docker availability, review test output in history |
| SSE connection drops | Network proxy or timeout | Ensure SSE support (no buffering proxy) |

---

## Related Products

| Product | Relationship |
|---------|-------------|
| **SignalHive** | Precedes RepoReaper — surfaces candidate issues and signals |
| **TrustGate** | Precedes RepoReaper — manages trust and review layers |
| **HiveCore** | Orchestrator — can dispatch runs and display results via the capabilities contract |
| **@patchhive/ai-local** | Local AI gateway — optional drop-in for direct provider calls |
| **RepoMemory** | Optional context store — loads remembered conventions and queues FailGuard candidates from Smith rejections |
| **MergeKeeper** | Future — could gate PR merging after RepoReaper opens the PR |

---

## Current Status

- **Active development** — core workflow (scan → fix → PR) is fully implemented
- Frontend v1 at `/frontend/`, v2 prototype at `/frontend-v2/`
- CI: GitHub Actions for Rust backend (`cargo build`, `cargo test`, `cargo clippy`)
- No Prometheus or Kubernetes integration — health checks are HTTP-based
- Configuration is via `.env` file or environment variables
- Auth: API key hashing + optional service tokens for HiveCore
- The standalone mirror repo [`patchhive/reporeaper`](https://github.com/patchhive/reporeaper) is an exported copy of this directory
