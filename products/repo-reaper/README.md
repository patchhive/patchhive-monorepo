# RepoReaper by PatchHive

RepoReaper autonomously fixes selected repository issues and opens validated pull requests.

It is PatchHive's outbound contribution product: a multi-agent system that finds promising issues, selects likely code targets, generates patches, reviews and refines those patches, runs validation, and then opens a pull request when the result clears its gates.

## Product Documentation

- GitHub-facing product doc: [docs/products/repo-reaper.md](../../docs/products/repo-reaper.md)
- Product docs index: [docs/products/README.md](../../docs/products/README.md)

## Core Workflow

- hunt open GitHub issues and rank them for fixability
- choose relevant files and code paths before patch generation
- generate, review, and refine a proposed fix through product-owned agents
- run validation inside configured safety limits
- open an attributed PatchHive pull request only when the result clears the gates
- enrich patch attempts with RepoMemory context when configured
- queue FailGuard candidates in RepoMemory when Smith rejects generated work

## Operating Model

| Role | Responsibility |
| --- | --- |
| Scout | Finds candidate issues and scores them for fixability. |
| Judge | Narrows the patch to the most relevant files and code paths. |
| Reaper | Generates the initial fix. |
| Smith | Reviews and improves the patch before it moves forward. |
| Gatekeeper | Runs validation and handles pull request delivery. |

## Run Locally

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5173`
Backend: `http://localhost:8000`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` | GitHub token used for repo discovery, clone, push, and pull request creation. |
| `BOT_GITHUB_USER` / `BOT_GITHUB_EMAIL` | Git identity for PatchHive commits and pull requests. |
| `PROVIDER_API_KEY` | Direct AI provider API key when not using a local OpenAI-compatible gateway. |
| `PATCHHIVE_AI_URL` | Optional OpenAI-compatible local gateway such as `@patchhive/ai-local`. |
| `OLLAMA_BASE_URL` | Optional Ollama endpoint. |
| `COST_BUDGET_USD` | Run budget cap. |
| `MIN_REVIEW_CONFIDENCE` | Minimum Smith confidence before validation and PR delivery. |
| `RETRY_COUNT` | Patch or validation retry count. |
| `REAPER_ENABLE_UNTRUSTED_TESTS` | Enables validation commands for untrusted repos. Default is disabled. |
| `REAPER_TEST_SANDBOX` | Test sandbox mode, usually `docker`. |
| `REAPER_ALLOW_HOST_TESTS` | Allows host test execution when explicitly enabled. |
| `REAPER_TEST_TIMEOUT_SECONDS` | Validation timeout, defaulting to `600`. |
| `WEBHOOK_SECRET` | Optional webhook secret for watch-mode triggers. |
| `PATCHHIVE_REPO_MEMORY_URL` / `PATCHHIVE_REPO_MEMORY_API_KEY` | Optional RepoMemory context and FailGuard candidate destination. |
| `REAPER_API_KEY_HASH` | Optional pre-seeded app auth hash. Otherwise generate the first local key from the UI. |
| `REAPER_SERVICE_TOKEN_HASH` | Optional service-token hash for HiveCore or other PatchHive service callers. |
| `REAPER_DB_PATH` | SQLite path for runs, costs, and PR tracking. |
| `REAPER_WORK_DIR` | Local workspace used for cloned repositories and patch attempts. |
| `REAPER_PORT` | Backend port for split local runs. |

To reuse the same password across SignalHive, TrustGate, RepoReaper, and HiveCore, run `./scripts/set-suite-api-key.sh --stack first` from the monorepo root before starting the stack. For every PatchHive product, run `./scripts/set-suite-api-key.sh`. Once the hash is pre-seeded, RepoReaper can be used through a subdomain without remote bootstrap.

To give HiveCore a dedicated machine credential instead of reusing the operator login secret, generate a service token from `POST /auth/generate-service-token` and save that token in HiveCore Settings.

If you only want to work on public repositories, keep your GitHub token public-only. If you want RepoReaper to clone, push, and open pull requests against specific repositories, grant only the write permissions those repositories actually need.

## AI and Platform Integrations

RepoReaper can run through direct provider APIs or through `@patchhive/ai-local`.

```bash
PATCHHIVE_AI_URL=http://127.0.0.1:8787/v1
```

Optional integrations:

- `PATCHHIVE_REPO_MEMORY_URL` to load remembered conventions, hotspots, and failure patterns, and to queue FailGuard candidates from Smith rejections
- future TrustGate and MergeKeeper flows to gate outbound changes more tightly

## Safety Boundary

- first-time API-key bootstrap is localhost-first
- untrusted repo test execution is disabled by default
- if tests are enabled, Docker sandboxing is the default
- host test execution requires both `REAPER_ENABLE_UNTRUSTED_TESTS=true` and `REAPER_ALLOW_HOST_TESTS=true`
- validation commands time out after `REAPER_TEST_TIMEOUT_SECONDS` seconds, defaulting to `600`
- validation and pull request publication are treated as explicit gates, not incidental side effects
- FailGuard is cross-cutting: RepoReaper can suggest candidates from Smith rejections, but RepoMemory owns review and promotion

RepoReaper is the only current PatchHive product that writes code and opens pull requests. It should be the last step in the early suite loop, after signal and trust layers have made the candidate work visible and reviewable.

## HiveCore Fit

HiveCore should treat RepoReaper as a product-owned autonomous action surface. It can show health, capabilities, run history, dispatchable actions, and PR outcomes, but RepoReaper keeps ownership of patch generation, validation, attribution, and pull request delivery.

## Standalone Repository

The PatchHive monorepo is the source of truth for RepoReaper development. The standalone [`patchhive/reporeaper`](https://github.com/patchhive/reporeaper) repository is an exported mirror of this directory.
