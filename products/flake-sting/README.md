# FlakeSting by PatchHive

FlakeSting spots flaky CI patterns before unreliable checks erode team trust.

It reads recent GitHub Actions history, looks for fail or pass swings in test-like jobs and steps, and turns that churn into a ranked queue of likely flaky problems so teams can focus on the unstable parts of their delivery pipeline.

## Core Workflow

- read recent workflow runs and jobs for a target repository
- detect fail or pass swings instead of treating every red build the same
- score unstable jobs and steps into a practical flaky queue
- surface runner hints, rerun pressure, and direct evidence links
- compare each scan to the previous comparable run so teams can see whether flake pressure is rising or improving

FlakeSting is intentionally read-only in the MVP. It does not rerun workflows or edit CI configuration.

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

## GitHub Access

FlakeSting works best with a fine-grained personal access token.

- If you only want public repositories, keep the token public-only.
- GitHub Actions read access is the main requirement for the MVP.
- Put the token in `BOT_GITHUB_TOKEN`.

## Local Notes

- The backend stores scan history in SQLite at `FLAKE_STING_DB_PATH`.
- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- The product currently focuses on one repository at a time so the evidence stays easy to trust.
- Generate the first local API key from `http://localhost:5179`.

## Repository Model

The PatchHive monorepo is the source of truth for FlakeSting development. The standalone `patchhive/flakesting` repository is an exported mirror of this directory.
