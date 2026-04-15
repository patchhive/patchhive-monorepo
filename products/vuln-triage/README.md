# VulnTriage by PatchHive

VulnTriage turns vulnerability noise into a ranked engineering queue.

It reads GitHub code scanning alerts and dependency alerts, then prioritizes those findings by severity, likely impact, owner hint, and next practical action so teams can stop treating every security finding like it deserves the same response.

## Core Workflow

- read code scanning alerts and dependency alerts for a target repository
- group the findings into a practical triage queue
- rank each finding into action buckets such as `fix now`, `plan next`, or `watch`
- highlight likely ownership and the most useful next step
- save scan history so earlier snapshots can be reloaded and compared

VulnTriage is intentionally read-only in the MVP. It does not dismiss alerts, patch repositories, or open issues for you.

## Run Locally

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5181`
Backend: `http://localhost:8080`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

## GitHub Access

VulnTriage works best with a fine-grained personal access token.

- If you only want public repositories, keep the token public-only.
- Dependabot and code scanning reads require the matching GitHub security permissions.
- Put the token in `BOT_GITHUB_TOKEN`.

## Local Notes

- The backend stores scan history in SQLite at `VULN_TRIAGE_DB_PATH`.
- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- VulnTriage uses `patchhive-github-security` for typed GitHub security reads and keeps the ranking logic product-owned.
- Generate the first local API key from `http://localhost:5181`.

## Repository Model

The PatchHive monorepo is the source of truth for VulnTriage development. The standalone `patchhive/vulntriage` repository is an exported mirror of this directory.
