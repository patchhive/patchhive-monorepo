# FlakeSting by PatchHive

FlakeSting spots flaky CI patterns before unreliable checks erode team trust.

It reads recent GitHub Actions history, looks for fail or pass swings in test-like jobs and steps, and turns that churn into a ranked queue of likely flaky problems so teams can focus on the unstable parts of their delivery pipeline.

## Product Documentation

- GitHub-facing product doc: [docs/products/flake-sting.md](../../docs/products/flake-sting.md)
- Product docs index: [docs/products/README.md](../../docs/products/README.md)

## Core Workflow

- read recent workflow runs and jobs for a target repository
- detect fail or pass swings instead of treating every red build the same
- score unstable jobs and steps into a practical flaky queue
- surface runner hints, rerun pressure, and direct evidence links
- compare each scan to the previous comparable run so teams can see whether flake pressure is rising or improving

## Run Locally

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5179`
Backend: `http://localhost:8060`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` or `GITHUB_TOKEN` | Optional fine-grained PAT for workflow run and job reads. Recommended scopes: Metadata (read), Actions (read). |
| `FLAKE_STING_API_KEY_HASH` | Optional pre-seeded app auth hash. Otherwise generate the first local key from the UI. |
| `FLAKE_STING_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `FLAKE_STING_DB_PATH` | SQLite path for flaky scan history. |
| `FLAKE_STING_PORT` | Backend port for split local runs. |
| `RUST_LOG` | Rust logging level. |

FlakeSting works best with a fine-grained GitHub token. GitHub Actions read access is the main requirement for the MVP.

## Safety Boundary

FlakeSting is intentionally read-only. It does not rerun workflows, edit CI configuration, mark checks, or suppress failures. It explains where CI signal looks unstable so humans and downstream PatchHive products can treat that signal with the right level of trust.

## HiveCore Fit

HiveCore can surface FlakeSting health, capabilities, run history, and CI-trust pressure. Longer term, MergeKeeper and other products can use FlakeSting output to avoid over-trusting flaky checks.

## Standalone Repository

The PatchHive monorepo is the source of truth for FlakeSting development. The standalone [`patchhive/flakesting`](https://github.com/patchhive/flakesting) repository is an exported mirror of this directory.
