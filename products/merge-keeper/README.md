# MergeKeeper by PatchHive

MergeKeeper turns pull request merge pressure into a clear readiness call.

It reads a pull request the way a strong reviewer or engineering lead would: what is still blocked, what is risky, what is merely waiting, and what is actually ready? Then it returns a simple state such as `ready`, `hold`, or `blocked` with the reasons behind that decision.

## Product Documentation

- GitHub-facing product doc: [docs/products/merge-keeper.md](../../docs/products/merge-keeper.md)
- Product docs index: [docs/products/README.md](../../docs/products/README.md)

## Core Workflow

- fetch pull request metadata, reviewer decisions, review-thread pressure, and check health
- convert that into a single merge-readiness state
- show blockers and hold-level warnings clearly
- optionally fold in ReviewBee, TrustGate, and RepoMemory context
- keep the result visible inside GitHub through comments and check-style output

## Run Locally

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5178`
Backend: `http://localhost:8050`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` or `GITHUB_TOKEN` | Optional GitHub token for pull request, review, and check reads. |
| `MERGE_KEEPER_GITHUB_WEBHOOK_SECRET` | Optional signed webhook secret for refreshes. |
| `MERGE_KEEPER_PUBLIC_URL` | Optional public URL for links from GitHub artifacts back to saved runs. |
| `PATCHHIVE_REVIEW_BEE_URL` / `PATCHHIVE_REVIEW_BEE_API_KEY` | Optional ReviewBee context. |
| `PATCHHIVE_TRUST_GATE_URL` / `PATCHHIVE_TRUST_GATE_API_KEY` | Optional TrustGate context. |
| `PATCHHIVE_REPO_MEMORY_URL` / `PATCHHIVE_REPO_MEMORY_API_KEY` | Optional RepoMemory merge expectations. |
| `MERGE_KEEPER_API_KEY_HASH` | Optional pre-seeded app auth hash. Otherwise generate the first local key from the UI. |
| `MERGE_KEEPER_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `MERGE_KEEPER_DB_PATH` | SQLite path for readiness history. |
| `MERGE_KEEPER_PORT` | Backend port for split local runs. |
| `RUST_LOG` | Rust logging level. |

MergeKeeper works best with a fine-grained GitHub token. Reading pull requests, reviews, and commit health is enough for the base product loop. Maintained comments or check-style output may need extra write permissions.

## Integrations

MergeKeeper is stronger when the rest of PatchHive is available, but it still works on its own.

- `PATCHHIVE_REVIEW_BEE_URL` adds review churn context.
- `PATCHHIVE_TRUST_GATE_URL` adds safety and policy pressure.
- `PATCHHIVE_REPO_MEMORY_URL` adds repo-specific merge expectations.

If those services are not configured, MergeKeeper falls back to GitHub-only readiness logic.

## Safety Boundary

MergeKeeper is merge-readiness-first. It does not merge code, bypass review, approve pull requests, or force checks green in the MVP. Its job is to make the path to merge obvious and explainable.

## HiveCore Fit

HiveCore can use MergeKeeper as a suite-level readiness signal once product handoffs mature. MergeKeeper should keep owning its GitHub reads and readiness decision while HiveCore surfaces the result next to ReviewBee, TrustGate, RepoMemory, and CI context.

## Standalone Repository

The PatchHive monorepo is the source of truth for MergeKeeper development. The standalone [`patchhive/mergekeeper`](https://github.com/patchhive/mergekeeper) repository is an exported mirror of this directory.
