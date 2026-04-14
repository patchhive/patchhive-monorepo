# 🛡 VulnTriage by PatchHive

> Turn vulnerability noise into a ranked engineering queue teams can actually act on.

VulnTriage ingests GitHub code scanning alerts and dependency alerts, then ranks what matters most by severity, reachability proxy, owner hint, and the practical next action teams should take.

## What It Does

- reads GitHub code scanning alerts and Dependabot alerts for one repo at a time
- turns open security findings into `fix now`, `plan next`, or `watch`
- highlights where a finding lives, who should probably own it, and what the next engineering move should be
- stores scan history in SQLite so earlier snapshots can be reloaded
- stays read-only in the MVP: it does not dismiss alerts, patch repos, or open issues for you

## Quick Start

```bash
cp .env.example .env

# Backend
cd backend && cargo run

# Frontend
cd ../frontend && npm install && npm run dev
```

Backend: `http://localhost:8080`
Frontend: `http://localhost:5181`

## Local Run Notes

- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- The backend stores scan history in SQLite at `VULN_TRIAGE_DB_PATH`.
- Prefer a fine-grained personal access token over a classic PAT whenever your setup allows it.
- If you only want VulnTriage on public repos, keep repository access public-only and avoid private repo access.
- `BOT_GITHUB_TOKEN` or `GITHUB_TOKEN` is strongly recommended. Code scanning may work with weaker/public access in some repos, but Dependabot alert reads require authenticated GitHub access.
- VulnTriage mainly needs GitHub security read permissions such as code scanning alerts and Dependabot alerts.
- VulnTriage uses `patchhive-github-security` for typed GitHub security reads and keeps the ranking logic product-owned.

## Standalone Repo Notes

VulnTriage is developed in the PatchHive monorepo first, and `patchhive/vulntriage` is the exported standalone mirror for this product directory.

*VulnTriage by PatchHive — Turn vulnerability noise into a ranked engineering queue teams can actually act on.*
