# PatchHive Configuration Standards

This document records the **de facto configuration conventions** used across PatchHive
products. It is descriptive, not prescriptive: it captures what the products already do so
new products stay consistent and existing drift is visible.

The canonical monorepo source is the ignored root `.env`, seeded from [`.env.example`](../.env.example). Product `.env` paths may point to that file for standalone and Compose compatibility. The authoritative reference for any variable is the root example, the product's `.env.example`, and its
[detailed product doc](products/README.md). The README `## Configuration` table carries the
operator-facing essentials; the detailed doc carries the complete reference.

## Canonical Variable Suffixes

Every product prefixes its variables with its product slug and reuses a fixed set of suffixes:

| Suffix | Meaning | Example |
| --- | --- | --- |
| `_API_KEY_HASH` | Pre-seeded app auth hash (BCrypt/Argon). Generate the first key from the UI if unset. | `DEP_TRIAGE_API_KEY_HASH` |
| `_SERVICE_TOKEN_HASH` | Pre-seeded service-token hash for HiveCore / inter-product dispatch. | `DEP_TRIAGE_SERVICE_TOKEN_HASH` |
| `_PORT` | Backend listen port for split local runs. | `DEP_TRIAGE_PORT` |
| `_DB_PATH` | SQLite database path for runs / memory. | `DEP_TRIAGE_DB_PATH` |
| `_DB_POOL_SIZE` | SQLite connection pool size. | `DEP_TRIAGE_DB_POOL_SIZE` |
| `_PUBLIC_URL` | Public/base URL used for links back to saved runs (optional). | `MERGE_KEEPER_PUBLIC_URL` |
| `_GITHUB_WEBHOOK_SECRET` | HMAC secret for signed webhook verification (optional). | `TRUST_GITHUB_WEBHOOK_SECRET` |

## Shared / Suite-Level Variables

These are not product-prefixed. They appear across multiple products:

| Variable | Purpose |
| --- | --- |
| `PATCHHIVE_ENV_FILE` | Optional explicit path to the canonical suite environment file. |
| `PATCHHIVE_GITHUB_TOKEN_RO` | Suite-wide classic PAT used only by PatchHive GitHub read clients. Prefer `public_repo`; use `repo` only for intentional private-repository access. |
| `MERGE_KEEPER_GITHUB_TOKEN_RW` | MergeKeeper-only classic PAT for explicit status/comment publishing. |
| `REVIEW_BEE_GITHUB_TOKEN_RW` | ReviewBee-only classic PAT for explicit maintained-comment publishing. |
| `TRUST_GATE_GITHUB_TOKEN_RW` | TrustGate-only classic PAT for explicit status/comment publishing. |
| `REPO_REAPER_GITHUB_TOKEN_RW` | RepoReaper-only classic PAT for branches, commits, and pull requests. |
| `RUST_LOG` | Rust logging level. |
| `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP` | Set `true` to allow API-key generation from non-localhost. Defaults to localhost-only. |
| `PATCHHIVE_<OTHER>_URL` / `PATCHHIVE_<OTHER>_API_KEY` | Cross-product wiring (e.g. `PATCHHIVE_REPO_MEMORY_URL`, `PATCHHIVE_TRUST_GATE_URL`). Lets one product call another. |

## Per-Product Prefix Map

| Product | Prefix | Notes |
| --- | --- | --- |
| DepTriage | `DEP_TRIAGE_` | Full canonical set. |
| FlakeSting | `FLAKE_STING_` | Full canonical set. |
| RefactorScout | `REFACTOR_SCOUT_` | Adds `ALLOWED_ROOTS`, `ALLOW_REMOTE_FS`, `CLONE_TIMEOUT_SECS` (filesystem sandbox controls). |
| ReleaseSentry | `RELEASE_SENTRY_` | Full canonical set. |
| RepoMemory | `REPO_MEMORY_` | **Missing `_DB_POOL_SIZE`** — has `_DB_PATH` only. |
| RepoReaper | `REAPER_` | Broadest set (AI provider keys, sandbox caps, encryption). |
| ReviewBee | `REVIEW_BEE_` | Full canonical set. |
| SignalHive | `SIGNAL_` | Adds `SIGNAL_MARKER_REPO_LIMIT`. |
| TrustGate | `TRUST_` / `TRUSTGATE_` | **Mixed prefix — see deviations.** |
| VulnTriage | `VULN_TRIAGE_` | Full canonical set. |
| MergeKeeper | `MERGE_KEEPER_` | Adds `REQUIRE_APPROVAL`, webhook secret, public URL. |
| HiveCore | `HIVE_CORE_` | Adds `HIVECORE_ENCRYPTION_KEY`, `PATCHHIVE_LAUNCHER_URL`, `PATCHHIVE_SUITE_BOOTSTRAP_SECRET`. |

## Known Deviations

These are real inconsistencies in the current codebase. They are documented here so they can
be reconciled deliberately rather than copied forward:

1. **TrustGate prefix split.** TrustGate uses `TRUST_API_KEY_HASH`, `TRUST_DB_PATH`,
   `TRUST_GITHUB_WEBHOOK_SECRET`, `TRUST_SERVICE_TOKEN_HASH` but `TRUSTGATE_PORT`,
   `TRUSTGATE_PUBLIC_URL`, `TRUSTGATE_DB_POOL_SIZE`. New code should pick one (`TRUSTGATE_` is
   the more complete set); old `TRUST_` vars should be aliased or migrated.
2. **RepoMemory missing `_DB_POOL_SIZE`.** It defines `REPO_MEMORY_DB_PATH` but not the pool-size
   companion that every other DB-backed product has. Add `REPO_MEMORY_DB_POOL_SIZE` for parity.
3. **HiveCore dual spelling.** `HIVE_CORE_API_KEY_HASH` / `HIVE_CORE_PORT` coexist with
   `HIVECORE_ENCRYPTION_KEY`. Prefer `HIVE_CORE_` for product-scoped vars.

## Rules for Adding a New Variable

1. Use the product slug prefix + a suffix from the canonical list above.
2. Add it to `.env.example` **and** the detailed doc's `## Configuration` section.
3. Document the scope/permission it implies (e.g. GitHub token scopes) in the same place.
4. Keep secrets as `*_HASH` or `*_SECRET` — never store raw tokens in the repo.
5. If the variable wires two products together, use the `PATCHHIVE_<OTHER>_URL` / `_API_KEY`
   convention so the dependency is discoverable.

## Source of Truth

- Per-variable intent and defaults: `products/<slug>/.env.example`
- Complete reference + scope notes: `docs/products/<slug>.md` → `## Configuration`
- GitHub token scope guidance: [github-token-scopes.md](github-token-scopes.md)
