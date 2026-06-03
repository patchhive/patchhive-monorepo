# Suite Backend Direction

PatchHive should move toward one shared backend runtime for the suite while keeping each product as a distinct specialist experience.

The preferred long-term shape is **Option A: one shared PatchHive backend Docker image**.

```text
SignalHive frontend      \
ReviewBee frontend       \
TrustGate frontend        \
RepoMemory frontend        > patchhive-backend
MergeKeeper frontend      /
...                      /
HiveCore frontend        /
```

HiveCore becomes the cockpit and control-plane frontend. The unified backend becomes the suite runtime that owns auth, credentials, product registry, run history, shared config, product APIs, and cross-product orchestration.

The canonical implementation lives in `services/patchhive-backend/` in the PatchHive monorepo. The standalone `patchhive/patchhive-unified-backend` repository is an exported mirror target, not a second source of truth. Update it from the monorepo with `./scripts/export-service.sh patchhive-backend ...` instead of manually editing or copying a second checkout.

## Decision

Standalone product repositories should eventually launch the shared PatchHive backend Docker image with only their product enabled, instead of carrying long-term independent backend implementations.

For example:

```yaml
services:
  backend:
    image: patchhive/patchhive-backend:latest
    environment:
      PATCHHIVE_PRODUCTS: signal-hive

  frontend:
    build: ./frontend-v2
    environment:
      VITE_API_URL: http://localhost:8010/api
```

For the full suite:

```yaml
services:
  backend:
    image: patchhive/patchhive-backend:latest
    environment:
      PATCHHIVE_PRODUCTS: all

  hivecore:
    build: ./products/hive-core/frontend-v2
    environment:
      VITE_API_URL: http://localhost:8100/api
```

This keeps product repos real while avoiding twelve permanent backend forks.

## Why

The product-per-backend shape was useful while products were becoming real, but it creates repeated local friction:

- separate API keys per product
- separate GitHub token and provider setup
- separate `.env` load paths
- many local backend ports to remember
- repeated auth/bootstrap UI
- repeated health, run, and capability glue
- repeated GitHub clients, rate limits, and error handling
- more ways for HiveCore and product frontends to disagree

One backend makes PatchHive easier to manage and lets shared improvements land once.

## Product Identity

One backend does not mean one product.

Each product should keep:

- its name, accent, UI surface, and workflow language
- product-owned decision logic
- product-specific safety boundaries
- product-specific run types and evidence
- product-specific storage where it still makes sense
- standalone repo packaging and launch docs

The backend should host specialist product engines. It should not flatten every product into a generic scanner.

## Runtime Modes

PatchHive should support three runtime modes.

### Suite Mode

Suite mode runs the shared backend with every enabled product.

```bash
PATCHHIVE_PRODUCTS=all patchhive-backend
```

HiveCore and product frontends all connect to the same API. HiveCore can show product health, trigger runs, manage shared defaults, rotate credentials, and route handoffs because it is talking to the same runtime the product frontends use.

### Product Mode

Product mode runs the same backend with one product enabled.

```bash
PATCHHIVE_PRODUCTS=signal-hive patchhive-backend
```

This is the normal standalone developer and product-repo experience. SignalHive still feels like SignalHive, but it uses the shared backend runtime.

### Gateway Mode

Gateway mode is temporary migration support.

The unified backend can proxy to existing product backends while product logic is being moved into the shared runtime. This keeps current products working while the suite backend grows product by product.

Gateway mode should be treated as a bridge, not the end state.

## HiveCore Role

HiveCore should connect to `patchhive-backend` like every other frontend.

HiveCore owns the operator cockpit:

- suite radar and product status
- launch links and local stack awareness
- shared defaults and product config
- product capability visibility
- suite-wide run history
- credential status and rotation workflows
- cross-product handoffs
- schedules and orchestration controls

The backend owns the authority:

- operator auth
- GitHub and AI provider credentials
- product registry
- product route mounting
- product run dispatch
- SQLite storage
- rate limits
- audit events
- action approval enforcement

HiveCore should not need private product database reads or browser-exposed service tokens to control the suite.

## First Frontend To Connect

HiveCore should be the first frontend wired to `patchhive-backend`.

That does not mean HiveCore owns the product engines first. It means the first useful backend skeleton should expose enough suite-level routes for HiveCore to prove the control-plane shape:

- operator auth/session
- product registry
- product health/status
- shared credential status
- run index
- event log
- gateway dispatch to existing product backends

Once HiveCore can see and control the suite through the unified backend, product frontends can follow the same API base one by one. SignalHive should still be the first product engine moved into the backend because it is read-only and central to the suite story.

## API Shape

Product APIs should be mounted under stable namespaces:

```text
/api/products/signal-hive/scans
/api/products/review-bee/review/github/pr
/api/products/trust-gate/review/github/pr
/api/products/repo-memory/prompt-pack
/api/products/merge-keeper/assess/github/pr
/api/products/flake-sting/scan/github/actions
```

Shared suite APIs should live outside product namespaces:

```text
/api/auth/session
/api/config/defaults
/api/credentials/github
/api/products
/api/runs
/api/events
/api/schedules
```

Product frontends should not need to know whether a request is handled by an in-process product module or by temporary gateway proxying.

## Product Engine Rule

Product logic should be portable inside the shared backend.

The long-term code shape should be:

```text
patchhive-backend
  shared platform services
  products/
    signal_hive/
    review_bee/
    trust_gate/
    repo_memory/
    merge_keeper/
```

Each product module should own its scoring, heuristics, reports, and product-specific evidence. Shared behavior such as auth, GitHub transport, rate limiting, SQLite pools, run envelopes, credential handling, and health contracts should live in shared backend/platform code or shared crates.

Standalone product repos should package the shared backend image, not copy product backend source as a permanent fork.

## Storage

The preferred long-term storage shape is one backend-owned SQLite database with namespaced product tables and shared suite tables.

Shared tables should cover:

- operator auth/session state
- credentials and credential metadata
- product registry
- run index and event log
- schedules
- shared defaults
- audit events

Product modules may keep product-owned tables for product-specific evidence and detail. Table names should be namespaced by product, such as `signal_hive_scans` or `trust_gate_reviews`.

During migration, the unified backend can attach or import existing product SQLite files where that is safer than a one-shot migration.

## Security Rules

- Browser code should never receive GitHub tokens, AI provider keys, product service tokens, or launcher secrets.
- The unified backend should enforce operator auth before routing product requests.
- Product actions should advertise capability metadata before HiveCore dispatches them.
- Destructive, mutating, or outbound contribution actions need explicit approval flows.
- Product-specific risk decisions should stay inside the product module.
- Shared rate limits should apply to auth, GitHub, provider, and mutating routes.
- The backend should log dispatches and preserve enough evidence to debug cross-product runs.

## Standalone Repositories

Existing product repositories remain useful.

Their future shape should be:

```text
patchhive/signalhive
  frontend-v2/
  docker-compose.yml
  README.md
  config examples
  uses patchhive/patchhive-backend with PATCHHIVE_PRODUCTS=signal-hive
```

The product repo still gives users a focused standalone product experience. It just does not own a permanent independent backend implementation.

Standalone product repos should pin a backend image version for releases and may use `latest` only for local development docs.

## Migration Path

This should be gradual.

1. Keep current product backends working.
2. Create the shared `patchhive-backend` runtime skeleton.
3. Add product enable flags such as `PATCHHIVE_PRODUCTS=signal-hive` and `PATCHHIVE_PRODUCTS=all`.
4. Wire HiveCore frontend to the unified backend first for suite auth, product registry, product health, run index, and gateway dispatch.
5. Add gateway routes so the unified backend can proxy existing product APIs during migration.
6. Point product v2 frontends at the unified backend after HiveCore proves the control-plane path.
7. Move SignalHive into the unified backend as the first in-process product engine because it is read-only and already central to the suite story.
8. Update SignalHive standalone repo packaging to use the shared backend image with only SignalHive enabled.
9. Repeat product by product, prioritizing read-only products before write-capable products.
10. Retire old individual product backends only after each product's logic is safely hosted by the unified backend and standalone repo packaging is updated.

## First Products To Move

Recommended order:

1. SignalHive: read-only reconnaissance, best first proof of the shared runtime.
2. ReviewBee: PR read path, already useful without write access.
3. TrustGate: local diff and PR review paths, important for suite safety.
4. RepoMemory: shared context layer for later handoffs.
5. MergeKeeper and FlakeSting: readiness and CI trust signals.
6. DepTriage, VulnTriage, RefactorScout, ReleaseSentry: portfolio signal products.
7. RepoReaper: write-capable execution should move later after auth, approvals, and safety contracts are solid.
8. HiveCore backend behavior: fold launcher/control-plane APIs into the shared runtime once the suite backend is stable.

## Open Questions

- What should the first `patchhive-backend` crate/package layout be?
- Should the shared backend image be published as `patchhive/patchhive-backend` or another image name?
- How should product-specific migrations be versioned inside one backend binary?
- How much existing product SQLite data needs migration versus fresh v2 storage?
- What route names should become permanent for the namespaced product API?
- What is the minimum approval contract before RepoReaper can run through the unified backend?

The direction is clear: one backend runtime, many product faces.
