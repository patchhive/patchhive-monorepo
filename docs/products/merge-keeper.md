# MergeKeeper

<p align="center">
  <img src="../../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

MergeKeeper turns pull request state into a clear merge-readiness decision. It reads reviewer state, unresolved review pressure, commit and check health, and optional PatchHive context, then returns `ready`, `hold`, or `blocked`.

## Product Role

MergeKeeper is merge-readiness-first. It is the convergence point for GitHub state, ReviewBee review pressure, TrustGate safety decisions, RepoMemory repo expectations, and CI health. Its job is to make the path to merge obvious and explainable — it does not merge code, bypass review, approve pull requests, or force checks green in the MVP.

## Core Workflow

1. Fetch pull request metadata, reviewer decisions, review-thread pressure, and check health.
2. Convert that into a single merge-readiness state: `ready`, `hold`, or `blocked`.
3. Show blockers and hold-level warnings clearly with evidence.
4. Optionally fold in ReviewBee, TrustGate, and RepoMemory context.
5. Optionally publish a maintained PR comment and check/status signal back to GitHub.
6. Persist every assessment to a SQLite history table.

## Inputs

- **GitHub pull request** — repository (`owner/name`) and PR number.
- **Reviewer decisions** — approvals, changes-requested, comment-only reviews.
- **Review threads** — resolved, unresolved, and actionable open threads.
- **Commit/check health** — success, failure, pending status contexts and check runs.
- **Diff data** — changed files, additions, deletions, and per-file paths.
- **Optional: ReviewBee context** — open checklist items, actionable threads.
- **Optional: TrustGate context** — recommendation, risk score, blocked/warning findings.
- **Optional: RepoMemory context** — policy entries, pinned entries, repo-specific merge expectations.
- **Configuration** — `require_approval` flag (defaults to `MERGE_KEEPER_REQUIRE_APPROVAL` env var or `true`).

## Outputs

- **Readiness state**: `ready`, `hold`, or `blocked`.
- **Blockers**: hard signals that block merge (e.g. merge conflict, failing checks, changes requested, closed/draft PR).
- **Warnings**: hold-level signals (e.g. no approval, pending checks, open review threads, wide diff, integration findings).
- **Mergeability interpretation**: GitHub `mergeable=false` or state `dirty`
  indicates a hard merge conflict. State `blocked` with `mergeable=true` is a
  branch-protection or repository-policy hold, not a conflict, and therefore
  contributes a warning unless another hard blocker exists.
- **Evidence strings**: per-signal supporting detail (e.g. check names, reviewer names, thread excerpts).
- **Merge metrics**: approvals, changes-requested, comment-reviews, reviewer count, review threads (total/open/actionable), successful/pending/failing checks, changed files, additions, deletions.
- **Cross-product context**: embedded ReviewBee, TrustGate, RepoMemory previews when available.
- **GitHub report** (optional): attempt to publish a maintained PR comment + check run (falls back to commit status).
- **Persisted history**: full `MergeAssessment` JSON stored in SQLite with indexed lookups.

## Safety Boundary

MergeKeeper does not merge code, bypass review, approve pull requests, or force checks green in the MVP. Its job is to make the path to merge obvious and explainable.

## Local Development

```bash
cd products/merge-keeper
cp .env.example .env
docker compose up --build
```

Defaults:
- Frontend: `http://localhost:5178`
- Legacy frontend reference: `http://localhost:5197` when started with the
  `legacy-ui` Docker Compose profile
- Backend: `http://localhost:8050`
- Suite backend route: `http://localhost:8100/api/products/merge-keeper`
- Database: `merge-keeper.db` (configurable via `MERGE_KEEPER_DB_PATH`)

Split local workflow:
```bash
cd products/merge-keeper/backend
cargo run

cd ../frontend-v2
npm install && npm run dev
```

### Unified Backend Mode

MergeKeeper is the first product engine mounted in-process inside
`services/patchhive-backend`. In suite mode, the v2 frontend should talk to the
unified backend route instead of a separate MergeKeeper backend service:

```bash
PATCHHIVE_PRODUCTS=merge-keeper \
PATCHHIVE_BIND_ADDR=127.0.0.1:8100 \
cargo run --manifest-path services/patchhive-backend/Cargo.toml

npm --prefix products/merge-keeper/frontend-v2 run dev
```

The standalone backend at `products/merge-keeper/backend` remains as a
compatibility wrapper around the same product module while the migration is
tested. Once product-mode packaging runs the shared backend image with only
MergeKeeper enabled, the old separate backend service can be moved to legacy or
removed.

### UI v1 to v2 Parity Audit

Audited on 2026-07-03 against:

- `products/merge-keeper/frontend-legacy/src/App.jsx`
- `products/merge-keeper/frontend-legacy/src/panels/KeeperPanel.jsx`
- `products/merge-keeper/frontend-legacy/src/panels/HistoryPanel.jsx`
- `products/merge-keeper/frontend-legacy/src/panels/ChecksPanel.jsx`
- `products/merge-keeper/frontend-v2/src/App.jsx`

The v2 MergeKeeper surface must keep these v1 workflows before the old UI can
be removed:

- API-key login and first-key generation.
- Directed PR assessment by `owner/repo` and PR number.
- Optional GitHub report publishing toggle.
- Readiness outcome with blockers, warnings, summary, and evidence.
- Merge metrics for approvals, requested changes, failing checks, pending
  checks, actionable/open review threads, changed files, additions, and
  deletions.
- PR identity and direct open-link behavior.
- GitHub artifact posture, including maintained comment link, check/report link,
  report state/details, and copyable report markdown.
- ReviewBee, TrustGate, and RepoMemory context when configured, with clear local
  fallback when they are not configured.
- Overview/history counts for stored runs, repos seen, ready calls, hold calls,
  and blocked calls.
- History list, selected-run loading, and full selected-run detail.
- Health/startup checks for backend status, DB path, auth, GitHub readiness,
  webhook configuration, report publishing, startup warnings, and integrations.

Current v2 parity status:

- **Covered**: readiness assessment, local/publish mode, approval policy toggle,
  history list, selected-run detail, radar visualization, blockers/warnings,
  suite input posture, health/startup checks, and clear selected-assessment
  behavior.
- **Legacy status**: after the final 2026-07-05 recheck, the old UI was moved
  to `products/merge-keeper/frontend-legacy/`. `frontend-v2/` is the active
  local and Docker frontend.
- **Improved from v1**: loading a history row stays in the history context and
  renders the selected radar/detail below it instead of always kicking the user
  back to the main page.
- **Intentional v2 change**: `publish_report` defaults off in the v2 form during
  local gateway/unified-backend testing. Operators can still enable it per run.
  The API default remains documented separately.
- **Replaced surface**: the old dedicated Setup tab is covered by v2 auth,
  readiness, and Checks surfaces rather than a standalone setup wizard.
- **Deferred polish**: v1 had a dedicated latest-reviewer-state badge strip and
  an inline rendered report preview. V2 currently summarizes reviewer pressure
  in metrics and exposes report links/copy actions. Bring back a compact
  reviewer-state strip or expandable report preview only if live use shows they
  save meaningful time.

Before deleting the old MergeKeeper UI, run one final browser pass that covers:

1. A ready PR with `require_approval=false`.
2. A blocked PR with failing checks.
3. A saved history load.
4. A `publish_report=true` run with enough token scope to create/update the
   maintained PR comment and check/status artifact.
5. The Checks tab with GitHub, webhook, report publish, DB, and integration
   states visible.

### UI v1 and v2 to v3 Parity Audit

Re-audited on 2026-07-11 using v1 and v2 together as the behavioral source for
`products/merge-keeper/frontend-v3/`. The old frontends remain in place until
the v3 surface receives operator sign-off.

The audit found that the initial generic v3 workspace omitted several important
surfaces that v1 or v2 already exposed. The parity implementation now includes:

- API-key login, first-key generation, sign-out, persistent suite theme, and
  mobile navigation.
- Directed `owner/repo` and positive PR-number validation, local/publish mode,
  and the per-run active-approval policy.
- Readiness summary, PR identity/link, mergeability, base/head branches, manual
  or webhook trigger context, and the complete merge metric set.
- Searchable and filterable blocker/warning evidence with decision/evidence
  filters, sorting, saved dashboard views, and dedicated detail views.
- Latest reviewer identities and states.
- ReviewBee, TrustGate, and RepoMemory summaries, counts, and returned top
  evidence, plus a clear GitHub-only fallback.
- GitHub artifact state, delivery details, comment/check links, copyable report
  Markdown, and an expandable Markdown preview.
- Saved-run history with decision, summary, blocker/hold, approval, and failing
  check context; repository/decision filters, sorting, search, saved views,
  refresh, short run identity, and six-row progressive disclosure; loaded runs
  update the shareable `?run=<id>` URL.
- Startup messages using the backend's actual `msg` field, backend/DB state,
  GitHub token/webhook/public-URL/publish posture, integration state, and
  GitHub permission guidance.
- The v2 footer identity wording through the shared v3 shell.

Local verification on 2026-07-11 used the newest MergeKeeper product database
and its saved hold assessment. The authenticated workspace, history-backed run
loading, deep-link URL, full hold evidence, report controls, Checks surface,
login surface, and responsive navigation were rendered successfully. The
MergeKeeper v3 production build and all nine backend tests passed. Shared-shell
regression builds also passed for ReleaseSentry, FlakeSting, and VulnTriage.

The final promotion gate remains intentionally explicit: verify one live ready
decision, one live blocked decision, and an operator-authorized
`publish_report=true` delivery before deleting v1/v2 or changing Docker's active
frontend. The audit did not perform a GitHub write merely to satisfy UI parity.

## Configuration

| Variable | Purpose |
|----------|---------|
| `BOT_GITHUB_TOKEN` | Fine-grained PAT for pull request, review, and check reads. Add Checks (write), Commit statuses (write), and Issues (write) for GitHub publishing. |
| `GITHUB_TOKEN` | Fallback GitHub token. |
| `MERGE_KEEPER_GITHUB_WEBHOOK_SECRET` | Optional signed webhook secret for auto-refresh on supported PR events. |
| `MERGE_KEEPER_PUBLIC_URL` | Optional public URL for deep-links from GitHub artifacts back to saved runs. |
| `MERGE_KEEPER_REQUIRE_APPROVAL` | Default approval policy (`true` = require one active approval before calling a PR ready). Manual API runs can override per assessment. Defaults to `true`. |
| `MERGE_KEEPER_API_KEY_HASH` | Optional pre-seeded API-key hash. Otherwise generate the first local key from the API (`POST /auth/generate-key`). |
| `MERGE_KEEPER_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `MERGE_KEEPER_DB_PATH` | SQLite database path (default: `merge-keeper.db`). |
| `MERGE_KEEPER_DB_POOL_SIZE` | SQLite connection pool size (default: product-core default). |
| `MERGE_KEEPER_PORT` | Backend port (default: `8050`). |
| `PATCHHIVE_REVIEW_BEE_URL` | Optional ReviewBee base URL. |
| `PATCHHIVE_REVIEW_BEE_API_KEY` | Optional ReviewBee API key. |
| `PATCHHIVE_TRUST_GATE_URL` | Optional TrustGate base URL. |
| `PATCHHIVE_TRUST_GATE_API_KEY` | Optional TrustGate API key. |
| `PATCHHIVE_REPO_MEMORY_URL` | Optional RepoMemory base URL. |
| `PATCHHIVE_REPO_MEMORY_API_KEY` | Optional RepoMemory API key. |
| `RUST_LOG` | Rust logging level (default: `info`). |

## Technical Architecture

### Module Tree

```
backend/src/
├── lib.rs               # Product module mounted by the standalone and suite backends
├── main.rs              # Standalone compatibility server bootstrap
├── models.rs            # Request/response types (AssessmentRequest, MergeAssessment, etc.)
├── db.rs                # SQLite persistence (merge_runs table)
├── github.rs            # GitHub API client, PR context fetching, report publishing
├── integrations.rs      # ReviewBee, TrustGate, RepoMemory client wrappers
├── startup.rs           # Configuration validation checks
├── state.rs             # AppState (reqwest Client)
├── pipeline/
│   ├── mod.rs
│   ├── routes.rs        # Axum route handlers for all endpoints
│   ├── assessment.rs    # Readiness assessment logic (blockers, warnings, summary)
│   └── utils.rs         # Shared utilities (API error helpers, text analysis, diff parsing)
└── auth/                # Generated by patchhive_product_core::define_api_key_auth_module!
```

### Key Dependencies

- **axum** — HTTP server framework
- **patchhive_product_core** — shared library providing: auth module, rate limiting, CORS, SQLite pool, startup check framework, capability contract, RepoMemory client
- **patchhive_github_pr** — GitHub PR client for fetching PR details, reviews, review threads, commit health, diffs, and publishing check runs / commit statuses / managed comments
- **reqwest** — HTTP client for integrations
- **rusqlite** — SQLite driver
- **serde / serde_json** — serialization
- **chrono** — timestamps
- **uuid** — assessment IDs
- **tokio** — async runtime
- **tracing / tracing-subscriber** — structured logging

### Data Flow

1. **Request received** — `POST /assess/github/pr` (manual) or `POST /webhooks/github` (webhook).
2. **GitHub fetch** — `github::fetch_merge_context()` retrieves PR detail, reviews, review threads, commit health, and diff in parallel via `GitHubPrClient`. Mergeability state is refreshed with up to 2 retries (900ms delay) if GitHub returns `unknown`.
3. **Assessment build** — `build_assessment()` in `pipeline/assessment.rs`:
   a. Counts approvals, changes-requested, comment-only reviews from latest-per-reviewer state.
   b. Analyzes review threads for unresolved / actionable open threads.
   c. Collects commit check health (success, failure, pending counts with evidence).
   d. Applies hard blockers: closed PR, already merged, draft, merge conflict, failing checks, changes-requested.
   e. Applies warnings: uncertain merge-state, pending checks, no approval, open review threads, wide diff.
   f. Fetches ReviewBee, TrustGate, and RepoMemory context concurrently via `tokio::join!`.
   g. Applies cross-product signals: ReviewBee pressure, TrustGate block/warn, RepoMemory policy expectations.
   h. Readiness is determined: any blocker → `blocked`, any warning (but no blockers) → `hold`, otherwise → `ready`.
4. **GitHub context attached** — trigger metadata (webhook event/action or manual) saved in assessment.
5. **Report publish** — optionally publishes a maintained PR comment + check run (falls back to commit status) via `github::publish_assessment_outcome()`.
6. **Persistence** — full `MergeAssessment` JSON stored in SQLite `merge_runs` table with indexed columns for history queries.

## API Endpoints

All non-public endpoints require `X-API-Key` or `X-PatchHive-Service-Token` header. Public paths: `/health`, `/auth/login`, `/auth/status`, `/auth/generate-key`, `/auth/generate-service-token`, `/auth/rotate-service-token`, `/startup/checks`, `/capabilities`, `/webhooks/github`.

### Health & Status

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Health check. Returns service status, DB health, GitHub token readiness, integration configuration, assessment counts, and version. |
| `GET` | `/startup/checks` | Detailed startup verification results (warnings, errors, info). |
| `GET` | `/capabilities` | Advertised product capabilities following the PatchHive contract format (actions + links). |

**`GET /health` response shape** (JSON):
```json
{
  "status": "ok" | "degraded",
  "version": "0.1.0",
  "product": "MergeKeeper by PatchHive",
  "auth_enabled": true,
  "config_errors": 0,
  "db_ok": true,
  "db_path": "merge-keeper.db",
  "github_ready": true,
  "assessment_count": 42,
  "repo_count": 5,
  "ready_count": 30,
  "hold_count": 8,
  "blocked_count": 4,
  "mode": "github-merge-readiness",
  "policy": { "approval_required_default": true },
  "github": {
    "token_configured": true,
    "token_verified": true,
    "webhook_secret_configured": true,
    "public_url_configured": false,
    "report_publish_configured": true,
    "report_publish_scope_verified": false,
    "report_publish_ready": false
  },
  "integrations": {
    "review_bee_configured": true,
    "trust_gate_configured": true,
    "repo_memory_configured": false
  }
}
```

Error states:
- DB failure or config errors > 0 → `status: "degraded"`.
- Missing GitHub token → `github_ready: false`, `report_publish_ready: false`.

`github_ready` and `github.token_verified` mean GitHub accepted the token through
an authenticated identity request. They do not imply access to every target
repository. `report_publish_configured` means a token is present; MergeKeeper
does not infer write permission from token presence or identity verification,
so `report_publish_scope_verified` and `report_publish_ready` remain `false`
until the publish path performs a real target-specific write. Every publish
attempt returns its concrete GitHub result.

### Authentication

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/auth/status` | Returns current auth configuration state. Public. |
| `POST` | `/auth/login` | Verify an API key. Body: `{"api_key": "..."}`. Returns `{"ok": true, "auth_enabled": true, "auth_configured": true}`. Errors: `503 SERVICE_UNAVAILABLE` if auth is not enabled, `401 UNAUTHORIZED` if key is invalid. Public. |
| `POST` | `/auth/generate-key` | Generate the first API key (only when auth is not yet configured; localhost-only). Returns `{"api_key": "...", "message": "Store this — it won't be shown again"}`. Public. |
| `POST` | `/auth/generate-service-token` | Generate the first service token for HiveCore or other PatchHive product callers (only when service auth is not yet configured). Returns `{"service_token": "...", "message": "..."}`. Public. |
| `POST` | `/auth/rotate-service-token` | Rotate an existing service token. Returns `{"service_token": "...", "message": "..."}`. Public. |

### Readiness Assessment

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/assess/github/pr` | Assess a GitHub pull request's merge readiness. **Requires auth.** |
| `POST` | `/webhooks/github` | Process a signed GitHub webhook for automatic readiness refresh on supported PR events. Public. |

**`POST /assess/github/pr` request body**:
```json
{
  "repo": "owner/repo-name",
  "pr_number": 123,
  "publish_report": true,
  "require_approval": true
}
```

- `repo`: required, must be in `owner/name` format.
- `pr_number`: required, must be > 0.
- `publish_report`: optional, defaults to `true`.
- `require_approval`: optional (`null` = use `MERGE_KEEPER_REQUIRE_APPROVAL` env default).

**Response**: a full `MergeAssessment` object (see Outputs section for structure).

Error codes:
- `400 BAD_REQUEST` — invalid repo format or PR number.
- `502 BAD_GATEWAY` — GitHub API error during fetch.
- `500 INTERNAL_SERVER_ERROR` — persistence failure.

**`POST /webhooks/github`**:
- Requires `MERGE_KEEPER_GITHUB_WEBHOOK_SECRET` to be configured.
- Verifies `X-Hub-Signature-256` against the webhook secret.
- Supported events: `pull_request` (opened, reopened, synchronize, ready_for_review, edited, closed), `pull_request_review` (submitted, edited, dismissed), `pull_request_review_comment` (created, edited, deleted), `pull_request_review_thread` (resolved, unresolved), `check_run` (created, completed, rerequested), `check_suite` (completed, rerequested).
- Returns `{"triggered": true, "event": "...", "action": "...", "readiness": "ready|hold|blocked", "assessment": {...}}`.
- Unsupported events return `{"triggered": false, ...}`.

### History & Overview

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/overview` | Product overview with aggregate counts (total runs, repos, ready/blocked/hold breakdown) and 6 most recent runs. Public. |
| `GET` | `/history` | List last 30 assessment history items. Public. |
| `GET` | `/history/:id` | Get a single full assessment by ID. Public. |
| `GET` | `/runs` | HiveCore contract-compatible runs list (delegates to history). Public. |

**`GET /overview` response shape**:
```json
{
  "product": "MergeKeeper by PatchHive",
  "tagline": "Read GitHub merge pressure and turn it into a clean readiness call...",
  "counts": {
    "runs": 42,
    "repos": 5,
    "ready_runs": 30,
    "blocked_runs": 4,
    "hold_runs": 8
  },
  "recent_runs": [ ... ]
}
```

**`GET /history` response**: array of `HistoryItem` objects:
```json
{
  "id": "uuid",
  "repo": "owner/repo",
  "pr_number": 123,
  "pr_title": "Fix the thing",
  "readiness": "ready",
  "summary": "This PR looks merge-ready...",
  "blockers_count": 0,
  "warnings_count": 1,
  "approvals_count": 1,
  "failing_checks_count": 0,
  "pending_checks_count": 0,
  "created_at": "2025-01-01T00:00:00Z"
}
```

**`GET /history/:id`** returns the full `MergeAssessment` JSON, or `404 {"error": "MergeKeeper run not found"}`.

## Monitoring

### Health Endpoint (`GET /health`)

The health endpoint provides a single diagnostic snapshot including:

- **Service status**: `"ok"` or `"degraded"` based on config errors and DB health.
- **Database**: connection check via `SELECT 1`, reports path.
- **GitHub readiness**: whether a token is configured and accepted by GitHub, plus separate webhook, public-URL, and unverified publishing posture.
- **Integration status**: whether ReviewBee, TrustGate, and RepoMemory are each configured.
- **Assessment counts**: total runs, unique repos, and breakdown by readiness state.
- **Policy**: current `approval_required_default` value.

Designed for container orchestration health checks (Docker HEALTHCHECK, K8s probes if applicable).

### Startup Checks (`GET /startup/checks`)

Returns detailed info/warn/error-level checks from `startup::validate_config()` including DB path, auth status, authenticated GitHub token verification, approval policy, integration URLs, webhook secret, and public URL configuration. The GitHub check also carries the machine-readable `github_token` code and `verified`, `failed`, or `missing` status.

### Logging

Controlled by `RUST_LOG` environment variable (default: `info`). Structured logging via `tracing` / `tracing-subscriber`.

## Deployment

### Docker

```bash
cd products/merge-keeper
cp .env.example .env
# Edit .env with your GitHub token and optional configuration
docker compose up --build
```

The Docker Compose setup starts the backend (port 8050) and active v2 frontend
(port 5178). The old v1 frontend is available only with the `legacy-ui` profile
on port 5197 while it remains useful as parity reference material.

### Minimal Standalone

```bash
cd products/merge-keeper/backend
cp ../.env.example .env
# Set BOT_GITHUB_TOKEN at minimum
cargo run --release
```

The backend runs on `MERGE_KEEPER_PORT` (default 8050). No external database server is required — SQLite stores history in a local file at `MERGE_KEEPER_DB_PATH`.

## Troubleshooting

| Symptom | Cause | Check |
|---------|-------|-------|
| `POST /assess/github/pr` returns `502` | GitHub API error — missing or invalid token, network issue, or PR doesn't exist | Verify `BOT_GITHUB_TOKEN` or `GITHUB_TOKEN` is set and has Metadata (read) + Pull requests (read) scopes. Verify the repo and PR number are correct. |
| `POST /assess/github/pr` returns `400 "Repository must be in owner/name format"` | `repo` field is not in `owner/repo-name` format | Ensure the `repo` value contains exactly one `/` with non-empty owner and name segments. |
| GitHub report is not published (`github_report.delivered: false`) | Token missing or lacks write scopes | Check `github_report.details` for the specific failure. Ensure token has Checks (write), Commit statuses (write), and Issues (write). |
| Webhook returns `401` | Signature mismatch — `MERGE_KEEPER_GITHUB_WEBHOOK_SECRET` doesn't match the webhook secret configured in GitHub | Compare the webhook secret in GitHub repo settings with the `.env` value. |
| Webhook returns `503` | `MERGE_KEEPER_GITHUB_WEBHOOK_SECRET` is not configured | Set `MERGE_KEEPER_GITHUB_WEBHOOK_SECRET` in `.env`. |
| Webhook returns `{"triggered": false}` | The webhook event/action is not in the supported list | Supported events: `pull_request` (opened/reopened/synchronize/ready_for_review/edited/closed), `pull_request_review` (submitted/edited/dismissed), `pull_request_review_comment` (created/edited/deleted), `pull_request_review_thread` (resolved/unresolved), `check_run` (created/completed/rerequested), `check_suite` (completed/rerequested). |
| Assessment always returns `blocked` with `pr-closed` | The PR is closed (merged or closed without merge) | Verify the PR is in open state. |
| Assessment always returns `hold` with `no-approval` | `MERGE_KEEPER_REQUIRE_APPROVAL` defaults to `true` and no reviewer has approved | Set `require_approval: false` in the request body, or set `MERGE_KEEPER_REQUIRE_APPROVAL=false` in `.env`. |
| DB connection error on startup | SQLite path is not writable or directory doesn't exist | Ensure the directory for `MERGE_KEEPER_DB_PATH` exists and is writable. Default path is the current working directory. |
| Health shows `"status": "degraded"` | Config errors or DB health check failed | Check `GET /startup/checks` for specific errors. Verify DB path and permissions. Ensure a GitHub token is configured. |

## Integrations

MergeKeeper is stronger when the rest of PatchHive is available, but it still works on its own.

- **ReviewBee** (`PATCHHIVE_REVIEW_BEE_URL`): Adds review churn context. Calls `{url}/review/github/pr` with repo, pr_number, and publish_comment=false. Contributes `review-bee-pressure` (blocker) or `review-bee-follow-up` (warning) signals based on open checklist items and actionable threads.
- **TrustGate** (`PATCHHIVE_TRUST_GATE_URL`): Adds safety and policy pressure via diff-risk analysis. Calls `{url}/review/github/pr` with repo, pr_number, ai_source="mergekeeper", and publish_status=false. Contributes `trust-gate-block` (blocker) or `trust-gate-warn` (warning) signals based on recommendation.
- **RepoMemory** (`PATCHHIVE_REPO_MEMORY_URL`): Adds repo-specific merge expectations. Uses `patchhive_product_core::repo_memory::fetch_repo_memory_context()` with changed paths and diff summary. Contributes `repo-memory-policy` (warning) signal when policy or pinned entries exist.

If those services are not configured, MergeKeeper falls back to GitHub-only readiness logic.

## Related Products

- **ReviewBee** — tracks active review checklist state per PR.
- **TrustGate** — evaluates diff risk and policy violations.
- **RepoMemory** — stores repo-specific merge expectations and conventions.
- **HiveCore** — can use MergeKeeper as a suite-level readiness signal once product handoffs mature.

## Current Status

- **MVP features**: GitHub PR assessment with reviewer state, review threads, commit/check health, diff analysis, optional integration context, GitHub report publishing (PR comment + check run / commit status), webhook auto-refresh, SQLite history, auth (API key + service token), startup validation, health endpoint.
- **Not yet in scope**: auto-merge, PR approval, forced check override, external issue tracker integration, Slack/Teams notifications, ML-based decision models.

## Standalone Repository

The PatchHive monorepo is the source of truth for MergeKeeper development. The standalone [`patchhive/mergekeeper`](https://github.com/patchhive/mergekeeper) repository is an exported mirror of this directory.
