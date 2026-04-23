# HiveCore

<p align="center">
  <img src="../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

HiveCore is the PatchHive control plane. It brings standalone PatchHive products
into one operational interface for health, launch links, shared defaults, run
history, capability visibility, and product handoffs.

Standalone repo: [patchhive/hivecore](https://github.com/patchhive/hivecore)

## Product Role

HiveCore is not a replacement for standalone products. Its first job is to make
the suite legible: what is running, what is healthy, what capabilities exist,
what work has happened, and where product contracts have drifted.

## What HiveCore Covers Today

- Product registry with launch links.
- Health polling across the product catalog.
- Startup check visibility.
- Capability and contract drift reporting.
- Product-owned run history.
- Server-side run detail proxying so product access tokens stay off the browser.
- Capability-driven action dispatch through advertised product actions.
- Global defaults for topics, languages, repo guardrails, and operator notes.
- Per-product frontend and API overrides.
- Server-side storage for per-product access tokens.

## Product Registry Defaults

| Product | Frontend | API |
| --- | --- | --- |
| RepoReaper | `http://localhost:5173` | `http://localhost:8000` |
| SignalHive | `http://localhost:5174` | `http://localhost:8010` |
| TrustGate | `http://localhost:5175` | `http://localhost:8020` |
| RepoMemory | `http://localhost:5176` | `http://localhost:8030` |
| ReviewBee | `http://localhost:5177` | `http://localhost:8040` |
| MergeKeeper | `http://localhost:5178` | `http://localhost:8050` |
| FlakeSting | `http://localhost:5179` | `http://localhost:8060` |
| DepTriage | `http://localhost:5180` | `http://localhost:8070` |
| VulnTriage | `http://localhost:5181` | `http://localhost:8080` |
| RefactorScout | `http://localhost:5182` | `http://localhost:8090` |
| HiveCore | `http://localhost:5183` | `http://localhost:8100` |

## Local Development

```bash
cd products/hive-core
cp .env.example .env
docker compose up --build
```

Defaults:

- Frontend: `http://localhost:5183`
- Backend: `http://localhost:8100`
- Database: `HIVE_CORE_DB_PATH`

Split local workflow:

```bash
cd products/hive-core/backend
cargo run

cd ../frontend
npm install
npm run dev
```

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `HIVE_CORE_API_KEY_HASH` | Optional preconfigured API-key hash. |
| `HIVE_CORE_SERVICE_TOKEN_HASH` | Optional service-token hash for HiveCore as a machine caller. |
| `HIVE_CORE_DB_PATH` | SQLite database path. |
| `HIVE_CORE_PORT` | Backend port. |
| `BOT_GITHUB_TOKEN` | Optional GitHub token for future integrations. |

## Boundary

HiveCore should use product-owned APIs, not product databases. The shared
contract target is documented in [../product-api-contract-v1.md](../product-api-contract-v1.md).

Current required product endpoints:

- `GET /health`
- `GET /startup/checks`
- `GET /capabilities`
- `GET /runs`
- `GET /runs/:id`

Future orchestration should arrive through explicit contracts, approvals, and
product-advertised actions rather than hidden coupling.
