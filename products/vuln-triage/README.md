# VulnTriage by PatchHive

VulnTriage turns vulnerability noise into a ranked engineering queue.

It reads GitHub code scanning alerts and dependency alerts, then prioritizes those findings by severity, likely impact, owner hint, and next practical action so teams can stop treating every security finding like it deserves the same response.

## Product Documentation

- GitHub-facing product doc: [docs/products/vuln-triage.md](../../docs/products/vuln-triage.md)
- Product docs index: [docs/products/README.md](../../docs/products/README.md)

## Core Workflow

- read code scanning alerts and dependency alerts for a target repository
- group the findings into a practical triage queue
- rank each finding into action buckets such as `fix now`, `plan next`, or `watch`
- highlight likely ownership and the most useful next step
- save scan history so earlier snapshots can be reloaded and compared

## Run Locally

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5181`
Frontend v2 prototype: `http://localhost:5200`
Backend: `http://localhost:8110`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

The UI v2 prototype is isolated from the production frontend while the suite
direction is still being tested:

```bash
cd frontend-v2 && npm install && npm run dev
```

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` or `GITHUB_TOKEN` | Optional fine-grained PAT for code scanning and Dependabot alert reads. Recommended scopes: Metadata (read), Code scanning alerts (read), Dependabot alerts (read). |
| `VULN_TRIAGE_API_KEY_HASH` | Optional pre-seeded app auth hash. Otherwise generate the first local key from the UI. |
| `VULN_TRIAGE_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `VULN_TRIAGE_DB_PATH` | SQLite path for vulnerability triage history. |
| `VULN_TRIAGE_PORT` | Backend port for split local runs. |
| `RUST_LOG` | Rust logging level. |

VulnTriage works best with a fine-grained GitHub token that has the matching security read permissions for the repositories being scanned.

## Security Feed Access Boundary

VulnTriage's current MVP reads GitHub code scanning alerts and Dependabot security alerts. Those are protected repository feeds, not general public data. A valid token can still receive `403 Forbidden` on public repositories when the operator does not have security-read access or when the target repo has alerts disabled.

That means the current product is strongest for repos the operator owns, administers, or has been granted security access to. For outbound/random public repository discovery, VulnTriage needs a future public-intelligence fallback mode: OSV/GHSA advisory lookup, manifest and lockfile parsing, public dependency inference, and code-pattern heuristics. That fallback is a planned feature, not a current MVP bug.

## Safety Boundary

VulnTriage is intentionally read-only. It does not dismiss alerts, patch repositories, open issues, or publish security statuses. It uses `patchhive-github-security` for typed GitHub security reads and keeps ranking decisions visible and product-owned.

## HiveCore Fit

HiveCore can surface VulnTriage health, capabilities, run history, and ranked security pressure. It should treat VulnTriage as the suite's security triage view while leaving alert reads and ranking logic inside the product.

## Standalone Repository

The PatchHive monorepo is the source of truth for VulnTriage development. The standalone [`patchhive/vulntriage`](https://github.com/patchhive/vulntriage) repository is an exported mirror of this directory.
