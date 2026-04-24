# HiveCore First-Stack Readiness Audit

<p align="center">
  <img src="../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

Updated: April 23, 2026

This audit covers the first local stack for testing HiveCore with the products that matter most right now: SignalHive, TrustGate, RepoReaper, and HiveCore itself.

The goal is not full orchestration yet. The goal is to make the first loop legible:

1. SignalHive finds maintenance pressure.
2. TrustGate reviews risky diffs or pull requests.
3. RepoReaper acts only when its own gates clear.
4. HiveCore shows health, startup checks, capabilities, run history, and contract drift from one place.

Each product remains standalone. HiveCore consumes product-owned APIs and saved product service tokens.

## Target Stack

| Product | Role | Frontend | API | Required first-pass config |
| --- | --- | --- | --- | --- |
| SignalHive | Read-only maintenance discovery | `http://localhost:5174` | `http://localhost:8010` | `BOT_GITHUB_TOKEN`, optional `SIGNAL_API_KEY_HASH`, optional `SIGNAL_SERVICE_TOKEN_HASH` |
| TrustGate | Diff and PR risk review | `http://localhost:5175` | `http://localhost:8020` | Optional `BOT_GITHUB_TOKEN`, optional `PATCHHIVE_REPO_MEMORY_URL`, optional `TRUST_SERVICE_TOKEN_HASH` |
| RepoReaper | Autonomous patch and PR execution | `http://localhost:5173` | `http://localhost:8000` | `BOT_GITHUB_TOKEN`, `BOT_GITHUB_USER`, `BOT_GITHUB_EMAIL`, AI credentials or `PATCHHIVE_AI_URL`, optional `REAPER_SERVICE_TOKEN_HASH` |
| HiveCore | Suite control plane | `http://localhost:5183` | `http://localhost:8100` | Optional `HIVE_CORE_API_KEY_HASH`, optional `HIVE_CORE_SERVICE_TOKEN_HASH`, saved product service tokens for protected reads |

Docker Compose maps every backend container to port `8000` internally and exposes the public API ports above. Those exposed ports match HiveCore's built-in product registry defaults.

## Bring-Up Order

Run each product from its own directory:

```bash
cd products/signal-hive
cp .env.example .env
docker compose up --build
```

Repeat for:

- `products/trust-gate`
- `products/repo-reaper`
- `products/hive-core`

For split local development, each product also supports:

```bash
cp .env.example .env

cd backend
cargo run

cd ../frontend
npm install
npm run dev
```

## Health Smoke Checks

After the services are up, check the public control endpoints:

```bash
curl -s http://localhost:8010/health
curl -s http://localhost:8020/health
curl -s http://localhost:8000/health
curl -s http://localhost:8100/health
```

HiveCore expects products to expose:

- `/health`
- `/startup/checks`
- `/capabilities`
- `/runs`
- `/runs/{id}` when run detail support is advertised

SignalHive, TrustGate, RepoReaper, and HiveCore all expose the shared health, startup, capability, and run list contracts. HiveCore reports contract drift if any endpoint is missing, locked, or malformed.

## Auth Notes

First-time operator API key generation is localhost-first. Open each product through its local frontend and generate the first key from the login screen, or pre-seed the matching hash in `.env`:

| Product | Operator hash | Service-token hash |
| --- | --- | --- |
| SignalHive | `SIGNAL_API_KEY_HASH` | `SIGNAL_SERVICE_TOKEN_HASH` |
| TrustGate | `TRUST_API_KEY_HASH` | `TRUST_SERVICE_TOKEN_HASH` |
| RepoReaper | `REAPER_API_KEY_HASH` | `REAPER_SERVICE_TOKEN_HASH` |
| HiveCore | `HIVE_CORE_API_KEY_HASH` | `HIVE_CORE_SERVICE_TOKEN_HASH` |

If you want the same password across the first stack and plan to use subdomains or other remote hosts, run this from the monorepo root before starting the products:

```bash
./scripts/set-suite-api-key.sh --stack first
```

That writes the same SHA-256 hash into the first-stack `.env` files. After restart, use the same raw password in SignalHive, TrustGate, RepoReaper, and HiveCore.

For product-to-product auth, prefer HiveCore Settings: paste a one-time operator API key there and let HiveCore provision or rotate a dedicated service token through each product's `POST /auth/generate-service-token` or `POST /auth/rotate-service-token` route. If you need to do it manually first, use:

```bash
curl -s -X POST http://localhost:8010/auth/generate-service-token -H "X-API-Key: <signal-operator-key>"
curl -s -X POST http://localhost:8020/auth/generate-service-token -H "X-API-Key: <trust-operator-key>"
curl -s -X POST http://localhost:8000/auth/generate-service-token -H "X-API-Key: <reaper-operator-key>"
```

After generating or rotating product service tokens, save the SignalHive, TrustGate, and RepoReaper service tokens in HiveCore Settings. HiveCore keeps those tokens server-side and uses them for protected `/runs` reads and advertised action dispatch.

## First Test Path

1. Start SignalHive and run a low-risk scan against public repositories or a tightly scoped allowlist.
2. Start TrustGate and review a pasted diff first. Move to GitHub PR diff review after token permissions are confirmed.
3. Start RepoReaper in the safest mode available for the target: low budget, high confidence threshold, untrusted tests disabled unless Docker sandboxing is configured, and dry-run targeting before real PR delivery.
4. Start HiveCore and confirm each product shows health, startup checks, capabilities, runs, and run detail support without contract errors.
5. Save product service tokens in HiveCore Settings and confirm protected run history appears.

## FailGuard Readiness

FailGuard is cross-cutting, not a standalone product. To test the failure-learning loop, also bring up RepoMemory at:

- Frontend: `http://localhost:5176`
- API: `http://localhost:8030`

Then configure:

- `PATCHHIVE_REPO_MEMORY_URL=http://localhost:8030` in TrustGate for `warn` and `block` candidates
- `PATCHHIVE_REPO_MEMORY_URL=http://localhost:8030` in RepoReaper for Smith rejection candidates
- matching `PATCHHIVE_REPO_MEMORY_API_KEY` values when RepoMemory auth is enabled

TrustGate and RepoReaper submit candidates best-effort. RepoMemory owns review, promotion, dismissal, and durable policy storage.

## Current Readiness Notes

- The first-stack Docker ports line up with HiveCore's default registry.
- The first-stack products expose the shared contract endpoints HiveCore needs for visibility.
- RepoReaper's `.env.example` now documents `REAPER_API_KEY_HASH`, matching the backend auth module.
- The first-stack path is ready for health, startup, capabilities, and run-history testing.
- Automatic SignalHive -> TrustGate -> RepoReaper handoff is still future orchestration work. Tomorrow's test should prove visibility and contract health first.
- Real RepoReaper PR delivery should stay behind strict GitHub permissions, dry-run checks, cost caps, confidence thresholds, and validation gates.
