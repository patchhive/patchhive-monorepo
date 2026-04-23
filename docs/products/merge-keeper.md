# MergeKeeper

<p align="center">
  <img src="../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

MergeKeeper turns pull request state into a clear merge-readiness decision. It
reads reviewer state, unresolved review pressure, commit and check health, and
optional PatchHive context, then returns `ready`, `hold`, or `blocked`.

Standalone repo: [patchhive/mergekeeper](https://github.com/patchhive/mergekeeper)

## Product Role

MergeKeeper is merge-readiness-first. It is the convergence point for GitHub
state, ReviewBee review pressure, TrustGate safety decisions, RepoMemory repo
expectations, and CI health.

## Core Workflow

1. Fetch pull request metadata and branch state.
2. Read review decisions and unresolved review pressure.
3. Inspect commit status and check health.
4. Optionally fold in ReviewBee, TrustGate, and RepoMemory context.
5. Produce a readiness state with clear reasons.
6. Optionally publish visible GitHub output.

## Inputs

- GitHub pull request reference.
- Reviews, review threads, commits, and checks.
- Optional ReviewBee checklist state.
- Optional TrustGate diff risk.
- Optional RepoMemory merge expectations.

## Outputs

- `ready`, `hold`, or `blocked` decision.
- Reason list and evidence.
- Saved readiness history.
- Optional maintained comment or check-style output.

## Safety Boundary

MergeKeeper does not merge code in the MVP. It tells a human or another product
whether the pull request appears ready, blocked, or still waiting.

## Local Development

```bash
cd products/merge-keeper
cp .env.example .env
docker compose up --build
```

Defaults:

- Frontend: `http://localhost:5178`
- Backend: `http://localhost:8050`
- Database: `MERGE_KEEPER_DB_PATH`

Split local workflow:

```bash
cd products/merge-keeper/backend
cargo run

cd ../frontend
npm install
npm run dev
```

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` | GitHub token for PR, review, and check reads. |
| `GITHUB_TOKEN` | Optional fallback GitHub token. |
| `MERGE_KEEPER_API_KEY_HASH` | Optional preconfigured API-key hash. |
| `MERGE_KEEPER_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `MERGE_KEEPER_DB_PATH` | SQLite database path. |
| `MERGE_KEEPER_PORT` | Backend port. |
| `MERGE_KEEPER_GITHUB_WEBHOOK_SECRET` | Signed webhook secret. |
| `MERGE_KEEPER_PUBLIC_URL` | Public URL for GitHub-linked artifacts. |
| `PATCHHIVE_REVIEW_BEE_URL` | Optional ReviewBee integration. |
| `PATCHHIVE_TRUST_GATE_URL` | Optional TrustGate integration. |
| `PATCHHIVE_REPO_MEMORY_URL` | Optional RepoMemory integration. |

## HiveCore Fit

HiveCore can use MergeKeeper as a suite-level readiness signal once product
handoffs become more common. MergeKeeper should still make its decision through
product-owned APIs and explicit integrations, not through private database reads.

