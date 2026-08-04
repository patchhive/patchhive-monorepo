# Suite Backend Architecture

Tendwright's specialist products run in one shared backend runtime while keeping
distinct product identities, workflows, safety boundaries, frontends, exports,
and evidence models.

The canonical implementation is `services/patchhive-backend/`. The standalone
`patchhive/patchhive-unified-backend` repository is an exported mirror, never a
second source of truth.

## Current shape

```text
specialist frontends
        |
        v
patchhive-backend
  /api/products/<slug>  -> in-process product router
  /api/*                -> suite-owned auth, registry, policy, and orchestration
        |
        v
PATCHHIVE_DB_PATH
```

All eleven specialist engines are integrated. Each backend crate exposes the
same `init_runtime()` and `router()` used by both the suite runtime and its thin
standalone launcher. Product tables are namespaced in the shared SQLite file.

HiveCore remains a separate control-plane concern and is intentionally not
changed by specialist backend cleanup.

## Product registry

Declarative manifests in
`services/patchhive-backend/registry/products/*.toml` own product identity,
route claims, capabilities, safety posture, credential requirements, health
contracts, and module paths. Adding or changing a route or capability requires
updating the manifest; product knowledge does not belong in backend `main.rs`.

`PATCHHIVE_PRODUCTS` selects enabled products. `all` enables the complete
registered suite; a comma-separated list creates a focused product runtime.

## Runtime contracts

- Product APIs live under `/api/products/<slug>`.
- Product routers apply shared API-key/service-token auth and rate limiting.
- Browser code receives no GitHub token, provider secret, service token, or
  launcher credential.
- The root `.env` is canonical. Product-local `.env` paths may only be
  compatibility symlinks.
- `PATCHHIVE_DB_PATH` is canonical in suite mode. A product DB variable may be
  a standalone compatibility override, not a second suite datastore.
- Important run phases and artifacts are persisted; live SSE alone is not run
  history.
- Run trigger and target selection are separate typed values.
- Product policy, scoring, evidence meaning, and report language stay
  product-owned.

## Shared and standalone launches

Suite mode runs all enabled specialists:

```bash
PATCHHIVE_PRODUCTS=all cargo run \
  --manifest-path services/patchhive-backend/Cargo.toml
```

A focused suite runtime uses the same binary:

```bash
PATCHHIVE_PRODUCTS=signal-hive cargo run \
  --manifest-path services/patchhive-backend/Cargo.toml
```

Exported product repositories may keep their thin launcher and product-owned
packaging. They must call the same library engine; they must not carry a forked
implementation of product behavior.

## Scheduling and orchestration

Product schedules use `patchhive_product_core::scheduling` so records, claims,
results, and suite-facing views stay consistent. Products still own payload
validation, authorization, execution, approval requirements, and safety copy.
A schedule never widens the underlying action boundary.

Suite dispatch uses advertised capabilities and a peer service credential.
Read-only and write-capable actions remain distinguishable. A configured
external policy dependency fails closed when it is required for a write.

## Storage

Suite-owned tables stay backend-owned. Product evidence tables use stable
product namespaces such as `signal_hive_*` and `repo_reaper_*`. Schema changes
remain deterministic, idempotent, and covered by migration tests. Back up the
SQLite file with matching WAL/SHM files before manual intervention.

## No-Proxy Product Migration Ladder

The completed integration order remains useful architectural history because it
documents how the risk surface grew:

1. MergeKeeper
2. ReleaseSentry
3. DepTriage
4. VulnTriage
5. FlakeSting
6. ReviewBee
7. TrustGate
8. RepoMemory
9. SignalHive
10. RefactorScout
11. RepoReaper

This ladder is complete for the specialist suite. It is not an active migration
checklist and does not authorize changes to HiveCore.

## Enforcement

- `scripts/check-suite-drift.sh` validates canonical frontend and backend shape.
- `scripts/check-rust-packages.sh` verifies the registered Rust package set.
- Registry tests validate route ownership and product enablement.
- Shared frontend dependency smoke tests ensure specialist consumers resolve
  `ui` and product-shell consistently.
- Export checks prove standalone mirrors can be regenerated without hand edits.

The steady-state rule is simple: one specialist runtime, many honest product
faces, and no duplicate implementation paths.
