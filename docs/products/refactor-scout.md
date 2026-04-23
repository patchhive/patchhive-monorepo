# RefactorScout

<p align="center">
  <img src="../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

RefactorScout surfaces safe, high-value refactor opportunities before structural
code drift turns expensive. It scans local repository paths and ranks conservative
cleanup leads such as oversized files, oversized functions, and repeated string
literals.

Standalone repo: [patchhive/refactorscout](https://github.com/patchhive/refactorscout)

## Product Role

RefactorScout is refactor-first, read-only, and conservative. Its job is to help
teams schedule cleanup work with a strong safety-to-value ratio.

## Core Workflow

1. Point RefactorScout at a local repository path inside an allowed root.
2. Walk the repository without mutating anything.
3. Rank refactor leads with explicit evidence.
4. Save scan history.
5. Reload or copy the ranked queue when planning cleanup work.

## Inputs

- Local repository path.
- Explicit filesystem allowlist roots.
- Optional scan settings.

## Outputs

- Ranked refactor queue.
- Evidence for each lead.
- Suggested first move.
- Saved scan history.

## Safety Boundary

RefactorScout is read-only. It does not rewrite code, apply codemods, or open
pull requests. Filesystem access should remain explicitly constrained.

Important safety rules:

- Set `REFACTOR_SCOUT_ALLOWED_ROOTS` before scanning broad checkout directories.
- Remote filesystem scans are disabled by default.
- Set `REFACTOR_SCOUT_ALLOW_REMOTE_FS=true` only when authenticated remote
  clients should intentionally trigger scans.

## Local Development

```bash
cd products/refactor-scout
cp .env.example .env
docker compose up --build
```

Defaults:

- Frontend: `http://localhost:5182`
- Backend: `http://localhost:8090`
- Database: `REFACTOR_SCOUT_DB_PATH`

Split local workflow:

```bash
cd products/refactor-scout/backend
cargo run

cd ../frontend
npm install
npm run dev
```

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `REFACTOR_SCOUT_API_KEY_HASH` | Optional preconfigured API-key hash. |
| `REFACTOR_SCOUT_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `REFACTOR_SCOUT_DB_PATH` | SQLite database path. |
| `REFACTOR_SCOUT_PORT` | Backend port. |
| `REFACTOR_SCOUT_ALLOWED_ROOTS` | Colon-separated filesystem roots allowed for scans. |
| `REFACTOR_SCOUT_ALLOW_REMOTE_FS` | Explicit opt-in for authenticated remote filesystem scans. |
| `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP` | Explicit opt-in for remote first-run bootstrap. |

## HiveCore Fit

HiveCore can surface RefactorScout as the suite's conservative cleanup discovery
view. Future handoffs should stay explicit: RefactorScout identifies work,
TrustGate evaluates risk, and write-capable products act only with approval.

