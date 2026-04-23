# FlakeSting

<p align="center">
  <img src="../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

FlakeSting detects flaky CI behavior before teams normalize unreliable checks.
It reads GitHub Actions history, looks for pass/fail swings and rerun pressure,
and ranks likely flaky jobs or steps with evidence.

Standalone repo: [patchhive/flakesting](https://github.com/patchhive/flakesting)

## Product Role

FlakeSting is CI-trust-first. It helps teams understand when a failing signal is
really a flaky system problem rather than a straightforward product regression.

## Core Workflow

1. Read recent GitHub Actions workflow runs and jobs.
2. Detect fail/pass swings in test-like jobs and steps.
3. Score unstable jobs and steps into a ranked flaky queue.
4. Surface runner hints, rerun pressure, and evidence links.
5. Compare against previous scans to show whether flake pressure is rising or
   improving.

## Inputs

- GitHub repository reference.
- GitHub Actions workflow runs and job history.
- Optional scan window and history settings.

## Outputs

- Ranked flaky queue.
- Runner, OS, job, and step evidence.
- Saved scan history.
- Trend comparisons across scans.

## Safety Boundary

FlakeSting is read-only in the MVP. It does not rerun workflows, edit CI
configuration, quarantine tests, or open cleanup issues by default.

## Local Development

```bash
cd products/flake-sting
cp .env.example .env
docker compose up --build
```

Defaults:

- Frontend: `http://localhost:5179`
- Backend: `http://localhost:8060`
- Database: `FLAKE_STING_DB_PATH`

Split local workflow:

```bash
cd products/flake-sting/backend
cargo run

cd ../frontend
npm install
npm run dev
```

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` | GitHub token for Actions reads. |
| `GITHUB_TOKEN` | Optional fallback GitHub token. |
| `FLAKE_STING_API_KEY_HASH` | Optional preconfigured API-key hash. |
| `FLAKE_STING_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `FLAKE_STING_DB_PATH` | SQLite database path. |
| `FLAKE_STING_PORT` | Backend port. |

## HiveCore Fit

HiveCore can surface FlakeSting as a CI trust signal. Longer term, FlakeSting
should help MergeKeeper and RepoReaper understand whether validation failures
represent real breakage or unstable infrastructure.

