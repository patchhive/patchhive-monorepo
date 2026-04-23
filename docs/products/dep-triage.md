# DepTriage

<p align="center">
  <img src="../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

DepTriage turns dependency update noise into a ranked engineering queue. It reads
open dependency pull requests, optionally folds in Dependabot alerts, groups work
by package, and recommends `update now`, `watch`, or `ignore for now`.

Standalone repo: [patchhive/deptriage](https://github.com/patchhive/deptriage)

## Product Role

DepTriage is dependency-triage-first. It helps teams spend attention on updates
that matter instead of treating every dependency pull request as equally urgent.

## Core Workflow

1. Read open dependency pull requests for a target repository.
2. Optionally read matching Dependabot alerts.
3. Group related update activity by package.
4. Score urgency, risk, and practical impact.
5. Save scan history for later comparison.

## Inputs

- GitHub repository reference.
- Open dependency pull requests.
- Optional Dependabot alerts.
- Optional ownership or workspace context in future flows.

## Outputs

- Ranked dependency queue.
- Action bucket per package.
- Evidence for why each update matters or can wait.
- Saved scan history.

## Safety Boundary

DepTriage is read-only in the MVP. It does not merge dependency pull requests,
change dependency files, or rewrite update configuration. Future execution should
flow through RepoReaper and TrustGate.

## Local Development

```bash
cd products/dep-triage
cp .env.example .env
docker compose up --build
```

Defaults:

- Frontend: `http://localhost:5180`
- Backend: `http://localhost:8070`
- Database: `DEP_TRIAGE_DB_PATH`

Split local workflow:

```bash
cd products/dep-triage/backend
cargo run

cd ../frontend
npm install
npm run dev
```

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` | GitHub token for pull request and optional alert reads. |
| `GITHUB_TOKEN` | Optional fallback GitHub token. |
| `DEP_TRIAGE_API_KEY_HASH` | Optional preconfigured API-key hash. |
| `DEP_TRIAGE_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `DEP_TRIAGE_DB_PATH` | SQLite database path. |
| `DEP_TRIAGE_PORT` | Backend port. |

## HiveCore Fit

HiveCore can surface DepTriage health, run history, and ranked update pressure.
The future cross-product path is DepTriage identifying an update, TrustGate
checking risk, and RepoReaper executing the dependency migration only after the
operator allows it.

