# TrustGate

<p align="center">
  <img src="../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

TrustGate reviews diffs before they move forward. It checks pasted unified diffs
or GitHub pull request diffs against repo-specific safety rules, then returns a
simple recommendation: `safe`, `warn`, or `block`.

Standalone repo: [patchhive/trustgate](https://github.com/patchhive/trustgate)

## Product Role

TrustGate is the safety layer in PatchHive. It does not compete with coding
agents. It evaluates their output and makes risk visible before maintainers or
automation advance a change.

## Core Workflow

1. Accept a pasted diff or fetch a pull request diff from GitHub.
2. Apply repo-specific rules and starter rule packs.
3. Flag risky paths, suspicious terms, missing tests, and oversized changes.
4. Return a clear decision with evidence.
5. Optionally publish a maintained PR comment, status, or check-style result.
6. Submit FailGuard candidates to RepoMemory for `warn` and `block` outcomes
   when configured.

## Inputs

- Unified diff text or GitHub pull request reference.
- Repo-specific safety rules.
- Optional report templates.
- Optional RepoMemory context for testing expectations, hotspots, and failure
  patterns.

## Outputs

- `safe`, `warn`, or `block` decision.
- Finding list with affected paths and evidence.
- Saved review history.
- Print-friendly or shareable review views.
- Optional GitHub comments, statuses, or check-style output.
- Optional FailGuard lesson candidates.

## Safety Boundary

TrustGate is review-first. It should not rewrite code, merge pull requests, or
hide product-specific policy decisions inside shared crates. Its value is the
clear risk call and the evidence behind it.

## Local Development

```bash
cd products/trust-gate
cp .env.example .env
docker compose up --build
```

Defaults:

- Frontend: `http://localhost:5175`
- Backend: `http://localhost:8020`
- Database: `TRUST_DB_PATH`

Split local workflow:

```bash
cd products/trust-gate/backend
cargo run

cd ../frontend
npm install
npm run dev
```

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` | Optional GitHub token for PR diff reads and publishing. |
| `TRUST_API_KEY_HASH` | Optional preconfigured API-key hash. |
| `TRUST_DB_PATH` | SQLite database path. |
| `TRUSTGATE_PORT` | Backend port. |
| `TRUST_GITHUB_WEBHOOK_SECRET` | Signed webhook secret. |
| `TRUSTGATE_PUBLIC_URL` | Public URL for GitHub-linked review artifacts. |
| `PATCHHIVE_REPO_MEMORY_URL` | Optional RepoMemory context and FailGuard destination. |

## HiveCore Fit

TrustGate is the gate before autonomous write behavior becomes comfortable.
HiveCore can surface TrustGate health, run history, and capabilities, then use
advertised actions when a product handoff needs a safety decision.

