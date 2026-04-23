# VulnTriage

<p align="center">
  <img src="../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

VulnTriage turns security alert noise into a ranked engineering queue. It reads
GitHub code scanning alerts and dependency alerts, then ranks findings by
severity, likely impact, ownership hints, and the next practical action.

Standalone repo: [patchhive/vulntriage](https://github.com/patchhive/vulntriage)

## Product Role

VulnTriage is security-triage-first. It helps small teams behave like they have
an AppSec triage layer without forcing them to stare at raw alert queues.

## Core Workflow

1. Read code scanning alerts and dependency alerts for a repository.
2. Normalize findings into a practical queue.
3. Rank findings into `fix now`, `plan next`, or `watch`.
4. Highlight ownership hints and the most useful next step.
5. Save scan history for reload and comparison.

## Inputs

- GitHub repository reference.
- Code scanning alerts.
- Dependabot alerts.
- Optional ownership and historical context in future flows.

## Outputs

- Ranked vulnerability queue.
- Action bucket per finding.
- Ownership hints and next-step guidance.
- Saved scan history.
- Future time-to-patch metrics.

## Safety Boundary

VulnTriage is read-only in the MVP. It does not dismiss alerts, patch code,
open issues, or publish security-sensitive findings by default.

## Local Development

```bash
cd products/vuln-triage
cp .env.example .env
docker compose up --build
```

Defaults:

- Frontend: `http://localhost:5181`
- Backend: `http://localhost:8080`
- Database: `VULN_TRIAGE_DB_PATH`

Split local workflow:

```bash
cd products/vuln-triage/backend
cargo run

cd ../frontend
npm install
npm run dev
```

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` | GitHub token for security alert reads. |
| `GITHUB_TOKEN` | Optional fallback GitHub token. |
| `VULN_TRIAGE_API_KEY_HASH` | Optional preconfigured API-key hash. |
| `VULN_TRIAGE_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `VULN_TRIAGE_DB_PATH` | SQLite database path. |
| `VULN_TRIAGE_PORT` | Backend port. |

## HiveCore Fit

HiveCore can surface VulnTriage as the security pressure view for the suite.
Future CVE response flows should route through explicit approval, TrustGate
review, and RepoReaper execution rather than letting triage imply automatic
patching.

