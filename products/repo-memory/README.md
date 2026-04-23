# RepoMemory by PatchHive

RepoMemory turns merged history and review pain into durable repo memory.

It captures what a repository has already learned from merged pull requests, reviewer feedback, recurring bug themes, and repeated hotspots so humans and agents do not keep rediscovering the same architectural expectations over and over.

## Product Documentation

- GitHub-facing product doc: [docs/products/repo-memory.md](../../docs/products/repo-memory.md)
- FailGuard capability doc: [docs/products/failguard.md](../../docs/products/failguard.md)
- Product docs index: [docs/products/README.md](../../docs/products/README.md)

## Core Workflow

- ingest merged pull requests, review feedback, issues, and file hotspots
- extract memory entries with evidence and confidence
- build reviewer and maintainer profile memories from repeated patterns
- store curated memories as signals, policies, or suppressed items
- queue, review, dismiss, and promote FailGuard lessons from bugs, outages, rejected patches, and painful reviews
- expose prompt-pack and context endpoints for other PatchHive products
- compare each ingest to the previous one so memory drift is visible over time

## Run Locally

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5176`
Backend: `http://localhost:8030`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` | Optional GitHub token for merged PR, review, issue, and file hotspot reads. |
| `REPO_MEMORY_API_KEY_HASH` | Optional pre-seeded app auth hash. Otherwise generate the first local key from the UI. |
| `REPO_MEMORY_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `REPO_MEMORY_DB_PATH` | SQLite path for runs and memory entries. |
| `REPO_MEMORY_PORT` | Backend port for split local runs. |
| `RUST_LOG` | Rust logging level. |

RepoMemory works best with a fine-grained GitHub token. Reading merged pull requests, reviews, and issues is enough for the core MVP loop.

## Cross-Product Use

RepoMemory is already useful on its own, but it also acts as infrastructure for the rest of PatchHive.

- RepoReaper can use it before patch generation.
- TrustGate can use it before diff review.
- MergeKeeper can use it for repo-specific merge expectations.
- FailGuard uses it to turn reviewed bad outcomes into pinned failure-pattern policy memories.

When enabled, downstream products can call RepoMemory through `PATCHHIVE_REPO_MEMORY_URL`.

## FailGuard Lessons

RepoMemory owns the FailGuard review loop:

- `GET /failguard/candidates` lists suggested lessons by repo and status.
- `POST /failguard/candidates` queues a bad outcome from an operator, TrustGate, RepoReaper, or another product.
- `POST /failguard/candidates/:id/promote` turns a reviewed candidate into a curated `failure_pattern` memory.
- `POST /failguard/candidates/:id/dismiss` rejects noisy or unhelpful candidates.
- `POST /failguard/lessons` still captures an already-approved lesson directly.

Promoted lessons carry path evidence, a prevention rule, and policy/pinned curation by default. TrustGate already consumes these memories through the RepoMemory context endpoint, so approved FailGuard lessons can become future warnings or blocks without making FailGuard a separate product.

TrustGate now submits candidates automatically when it returns `warn` or `block`. RepoReaper submits candidates automatically when Smith rejects a generated patch below the configured confidence threshold. Both integrations are best-effort and are skipped when `PATCHHIVE_REPO_MEMORY_URL` is not configured.

## Safety Boundary

RepoMemory is intentionally context-first. It does not open pull requests, mutate repositories, or automatically promote every bad outcome into durable policy. FailGuard is a cross-cutting capability surfaced through RepoMemory, not a standalone product; humans can review, promote, or dismiss candidates before they become pinned failure-pattern memories.

## HiveCore Fit

HiveCore should surface RepoMemory health, capabilities, run history, context availability, and FailGuard review pressure. Other products can consume RepoMemory through product-owned APIs while HiveCore shows whether the suite has enough repo context to trust deeper automation.

## Local Notes

- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- The generated prompt pack is meant to be reused as context, not treated as infallible policy.
- Generate the first local API key from `http://localhost:5176`.

## Standalone Repository

The PatchHive monorepo is the source of truth for RepoMemory development. The standalone [`patchhive/repomemory`](https://github.com/patchhive/repomemory) repository is an exported mirror of this directory.
