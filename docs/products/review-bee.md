# ReviewBee

<p align="center">
  <img src="../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

ReviewBee turns pull request review churn into a concrete follow-up checklist.
It reads review comments and review threads, separates actionable feedback from
noise, groups similar asks, and keeps the current state visible.

Standalone repo: [patchhive/reviewbee](https://github.com/patchhive/reviewbee)

## Product Role

ReviewBee is review-first and merge-speed-first. Its job is to help authors and
maintainers understand what still matters in a pull request without rereading a
long review history.

## Core Workflow

1. Fetch reviews and review threads for a target pull request.
2. Filter for actionable feedback.
3. Group repeated or related comments into checklist items.
4. Estimate what appears resolved versus still active.
5. Save review history.
6. Optionally maintain one GitHub comment with the current checklist.
7. Refresh from signed webhooks when review activity changes.

## Inputs

- GitHub pull request reference.
- Review comments, review threads, reviewer state, and file context.
- Optional RepoMemory reviewer-preference context in future flows.

## Outputs

- Actionable follow-up checklist.
- Grouped review asks.
- Saved review history.
- Optional maintained GitHub comment.
- Future task-list export to GitHub, Linear, or Jira.

## Safety Boundary

ReviewBee does not edit code and does not decide whether a pull request should
merge. It reduces review noise and makes remaining work easier to clear.

## Local Development

```bash
cd products/review-bee
cp .env.example .env
docker compose up --build
```

Defaults:

- Frontend: `http://localhost:5177`
- Backend: `http://localhost:8040`
- Database: `REVIEW_BEE_DB_PATH`

Split local workflow:

```bash
cd products/review-bee/backend
cargo run

cd ../frontend
npm install
npm run dev
```

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` | GitHub token for pull request and review reads. |
| `GITHUB_TOKEN` | Optional fallback GitHub token. |
| `REVIEW_BEE_API_KEY_HASH` | Optional preconfigured API-key hash. |
| `REVIEW_BEE_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `REVIEW_BEE_DB_PATH` | SQLite database path. |
| `REVIEW_BEE_PORT` | Backend port. |
| `REVIEW_BEE_GITHUB_WEBHOOK_SECRET` | Signed webhook secret. |
| `REVIEW_BEE_PUBLIC_URL` | Public URL used in maintained comments. |

## HiveCore Fit

HiveCore can surface ReviewBee health, run history, and capability support.
Later, ReviewBee output can feed MergeKeeper readiness calls and RepoMemory
reviewer-preference memories.

