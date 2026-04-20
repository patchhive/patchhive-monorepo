# PatchHive

PatchHive is a product suite for software maintenance, review, and autonomous contribution.

This repository is the PatchHive source-of-truth monorepo. New products, shared packages, and shared Rust crates are built here first, then exported into standalone repositories under [`patchhive`](https://github.com/patchhive) when they are ready to stand on their own.

## Product Suite

| Product | Repo | Role |
| --- | --- | --- |
| RepoReaper | [`patchhive/reporeaper`](https://github.com/patchhive/reporeaper) | Autonomously fixes selected issues and opens validated pull requests. |
| SignalHive | [`patchhive/signalhive`](https://github.com/patchhive/signalhive) | Surfaces stale work, duplicate issues, recurring bugs, and maintenance drag. |
| ReviewBee | [`patchhive/reviewbee`](https://github.com/patchhive/reviewbee) | Turns review churn into an actionable pull request checklist. |
| TrustGate | [`patchhive/trustgate`](https://github.com/patchhive/trustgate) | Reviews diffs against repo-specific safety and policy rules. |
| RepoMemory | [`patchhive/repomemory`](https://github.com/patchhive/repomemory) | Builds durable repo memory from merged history, reviews, and recurring failures. |
| MergeKeeper | [`patchhive/mergekeeper`](https://github.com/patchhive/mergekeeper) | Decides whether a pull request is actually ready to merge. |
| FlakeSting | [`patchhive/flakesting`](https://github.com/patchhive/flakesting) | Detects flaky CI patterns from GitHub Actions history. |
| DepTriage | [`patchhive/deptriage`](https://github.com/patchhive/deptriage) | Prioritizes dependency updates by urgency and practical impact. |
| VulnTriage | [`patchhive/vulntriage`](https://github.com/patchhive/vulntriage) | Ranks code scanning and dependency alerts into a useful engineering queue. |
| RefactorScout | [`patchhive/refactorscout`](https://github.com/patchhive/refactorscout) | Surfaces safe, high-value refactor opportunities before code quality drift compounds. |
| HiveCore | [`patchhive/hivecore`](https://github.com/patchhive/hivecore) | Centralizes suite visibility, shared defaults, and launch control across PatchHive. |

## Shared Foundations

| Foundation | Repo | Purpose |
| --- | --- | --- |
| `@patchhivehq/ui` | [`patchhive/patchhive-ui`](https://github.com/patchhive/patchhive-ui) | Shared React components, layout primitives, and product themes. |
| `@patchhivehq/product-shell` | [`patchhive/product-shell`](https://github.com/patchhive/product-shell) | Shared frontend auth bootstrap, session handling, and product app framing. |
| `@patchhive/ai-local` | [`patchhive/patchhive-ai-local`](https://github.com/patchhive/patchhive-ai-local) | Local OpenAI-compatible gateway for Codex, Copilot, and future providers. |
| `patchhive-product-core` | [`patchhive/patchhive-product-core`](https://github.com/patchhive/patchhive-product-core) | Shared Rust auth, startup, and cross-product service primitives. |
| `patchhive-github-pr` | [`patchhive/patchhive-github-pr`](https://github.com/patchhive/patchhive-github-pr) | Shared Rust pull request, webhook, check, and comment plumbing. |
| `patchhive-github-data` | [`patchhive/patchhive-github-data`](https://github.com/patchhive/patchhive-github-data) | Shared Rust repo, issue, PR history, and Actions data client. |
| `patchhive-github-security` | [`patchhive/patchhive-github-security`](https://github.com/patchhive/patchhive-github-security) | Shared Rust security and advisory data client. |
| Product Starter | [`patchhive/patchhive-product-starter`](https://github.com/patchhive/patchhive-product-starter) | Monorepo-first starter for new PatchHive products. |

## Repository Layout

```text
patchhive/
  products/     standalone products
  packages/     shared frontend and gateway packages
  crates/       shared Rust libraries
  templates/    starter scaffolds and reusable repo templates
  scripts/      export, release, and maintenance workflows
  docs/         internal operating docs and release workflows
```

## Getting Started

### Prerequisites

- Rust and Cargo
- Node.js and npm
- Docker and Docker Compose

### Work on an Existing Product

```bash
git clone https://github.com/patchhive/patchhive2.git patchhive
cd patchhive

# Example: SignalHive
cd products/signal-hive
cp .env.example .env
docker compose up --build
```

Most products also support a split local workflow:

```bash
cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

### Create a New Product

```bash
./scripts/new-product.sh <product-slug>
```

The starter includes:

- shared Rust backend auth and startup wiring
- shared frontend auth and app shell wiring
- Docker and local-development setup
- API-key bootstrap flow
- standalone GitHub Actions CI

## Development Model

PatchHive is intentionally monorepo-first.

- Build features here first.
- Release shared packages from here first.
- Export products, crates, and packages into standalone repos when they are ready.
- Treat exported repositories as mirrors, not parallel sources of truth.

The export flow is documented in [docs/product-export-workflow.md](docs/product-export-workflow.md), and the starter workflow is documented in [docs/product-starter-workflow.md](docs/product-starter-workflow.md).

## Authentication Model

Every product ships with the same first-run API-key bootstrap pattern.

- Open the product from `http://localhost:<frontend-port>` for first-time bootstrap.
- Generate the first API key locally.
- Use session storage in the browser for subsequent authenticated requests.
- If remote bootstrap is truly intentional, opt in explicitly with `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP=true`.

GitHub-backed products are designed to work with fine-grained personal access tokens and can be run against public repositories only when that fits the use case.

## Current Status

PatchHive already has real standalone repositories, shared infrastructure, Docker support, exported mirrors, and CI across the suite. The focus now is deepening product quality while keeping shared seams stable enough for future orchestration through HiveCore.
