# TrustGate by PatchHive

TrustGate reviews diffs before they move forward. It is PatchHive's trust and safety layer: a product that checks AI-generated or pull-request-backed diffs against repo-specific risk rules, then returns a simple recommendation such as `safe`, `warn`, or `block` with the reasons made explicit.

Every review returns one of those three recommendations plus the evidence behind it — matched paths, line excerpts, and a 0–100 risk score — so maintainers and downstream automation can decide before a change advances.

## Documentation

- Full product doc: [docs/products/trust-gate.md](../../docs/products/trust-gate.md)
- Product docs index: [docs/products/README.md](../../docs/products/README.md)

> This README is the getting-started entry point. The full product doc carries the API reference, technical architecture, complete configuration reference, monitoring, deployment, and troubleshooting.

### Where to find what

| If you need… | See in the full doc |
| --- | --- |
| API endpoints and request/response shapes | `#api-endpoints` |
| Report template variables | `#report-template-variables` |
| Service layout and dependencies | `#technical-architecture` |
| Every configuration variable | `#configuration` |
| Health checks and metrics | `#monitoring` |
| Production deployment steps | `#deployment` |
| Symptom → cause → fix | `#troubleshooting` |
| How it relates to other products | `#related-products` |
| What is / isn't built yet | `#current-status` |

## Core Workflow

- review pasted unified diffs or fetch pull request diffs directly from GitHub
- apply repo-specific rules, starter rule packs, and saved report templates
- flag risky paths, suspicious patterns, missing tests, and oversized change sets
- publish the result back into GitHub through a status signal and maintained pull request comment
- submit FailGuard candidates to RepoMemory for `warn` and `block` results when RepoMemory is configured
- save decision history and expose a print-friendly decision view

## Port Reference

| Mode | Frontend | Backend | Notes |
| --- | --- | --- | --- |
| Docker Compose | `http://localhost:5175` | `http://localhost:8020` | External host ports from `docker compose.yml` |
| Split local dev | `http://localhost:5175` | `http://localhost:8000` | `npm run dev` plus `cargo run` |
| Frontend preview | `http://localhost:4175` | `http://localhost:8000` | `npm run preview` plus backend running locally |
| Container internal | `http://frontend:8080` | `http://backend:8000` | Internal service ports inside Docker |

### Canonical UI v3

TrustGate's canonical Lovable-derived UI lives in `frontend/`. It preserves
pasted-diff review, GitHub PR review and publishing, decision evidence,
filterable history and saved views, rule packs, saved repo rules, report
templates, startup diagnostics, Markdown copy, and print-friendly HTML export.
The v1 and v2 frontends were removed after final parity acceptance on
2026-07-12.

```bash
cd frontend && npm install && npm run dev
```

## Quick Start

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5175`
Backend: `http://localhost:8020`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

For split local runs, the backend listens on `8000` by default and the frontend listens on `5175`.

## Configuration

| Variable | Purpose |
| --- | --- |
| `PATCHHIVE_GITHUB_TOKEN_RO` | Shared classic PAT for pull-request diff reads. |
| `TRUST_GATE_GITHUB_TOKEN_RW` | Dedicated classic PAT for explicit commit-status and maintained-comment publishing; native check runs require a GitHub App. |
| `TRUST_GITHUB_WEBHOOK_SECRET` | Optional signed webhook secret for pull request refreshes. |
| `TRUSTGATE_PUBLIC_URL` | Optional public URL for links from GitHub artifacts back to saved decisions. |
| `PATCHHIVE_REPO_MEMORY_URL` / `PATCHHIVE_REPO_MEMORY_API_KEY` | Optional RepoMemory context and FailGuard candidate destination. |
| `TRUST_API_KEY_HASH` | Optional pre-seeded app auth hash. Otherwise generate the first local key from the UI. |
| `TRUST_SERVICE_TOKEN_HASH` | Optional service-token hash for HiveCore or other PatchHive service callers. |
| `TRUST_DB_PATH` | SQLite path for rules, templates, and review history. |
| `TRUSTGATE_PORT` | Backend port for split local runs. |
| `RUST_LOG` | Rust logging level. |

To reuse the same password across SignalHive, TrustGate, RepoReaper, and HiveCore, run `./scripts/set-suite-api-key.sh --stack first` from the monorepo root before starting the stack. For every PatchHive product, run `./scripts/set-suite-api-key.sh`. Once the hash is pre-seeded, TrustGate can be used through a subdomain without remote bootstrap.

To give HiveCore a dedicated machine credential instead of reusing the operator login secret, generate a service token from `POST /auth/generate-service-token` and save that token in HiveCore Settings.

TrustGate works without GitHub for pasted diff review, but GitHub integration is what makes it operational inside real pull request flow. Direct pull request diff fetches need read access. PAT publishing uses commit-status and issue-comment writes; GitHub App installation tokens may publish native check runs instead.

## Safety Boundary

TrustGate is intentionally review-first. It does not rewrite code, approve code, merge pull requests, or turn every warning into hard policy. Its job is to make risk visible before downstream automation or maintainers move a change forward. FailGuard remains cross-cutting: TrustGate can suggest candidates, but RepoMemory owns the review and storage loop.

## HiveCore Fit

HiveCore can surface TrustGate health, capabilities, run history, and contract support, then eventually use TrustGate as the explicit safety gate before RepoReaper actions. TrustGate remains independently runnable and keeps its own rules, templates, decisions, and GitHub publishing behavior.

## Standalone Repository

The PatchHive monorepo is the source of truth for TrustGate development. The standalone [`patchhive/trustgate`](https://github.com/patchhive/trustgate) repository is an exported mirror of this directory.
