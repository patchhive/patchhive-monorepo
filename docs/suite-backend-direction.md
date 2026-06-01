# Suite Backend Direction

PatchHive should move toward one browser-facing backend for the suite while keeping the products as distinct specialist experiences.

The target shape is:

```text
SignalHive frontend      \
ReviewBee frontend       \
TrustGate frontend        \
RepoMemory frontend        > HiveCore / suite backend
MergeKeeper frontend      /
...                      /
HiveCore frontend        /
```

The products still exist. SignalHive remains reconnaissance, ReviewBee remains review-feedback analysis, TrustGate remains policy/risk review, MergeKeeper remains merge readiness, and so on. The change is that product frontends should eventually call one suite backend instead of each frontend owning a separate backend/auth/config path.

## Why

The current product-per-backend shape is useful while products are still becoming real, but it creates repeated local friction:

- separate API keys per product
- separate GitHub token and provider setup
- separate `.env` load paths
- many local backend ports to remember
- repeated auth/bootstrap UI
- repeated health, run, and capability glue
- more ways for HiveCore and product frontends to disagree

A suite backend makes PatchHive feel like one system without erasing product identity.

## Target Model

The preferred end state is:

- one suite backend process exposed to browsers
- one operator auth flow
- one place for GitHub, AI provider, allowlist, denylist, and suite defaults
- one product registry
- one shared run/event store for suite-level visibility
- product-specific APIs mounted under stable namespaces
- product-specific frontends that call the same backend

Example browser-facing routes:

```text
/api/products/signal-hive/scans
/api/products/review-bee/review/github/pr
/api/products/trust-gate/review/github/pr
/api/products/merge-keeper/assess/github/pr
/api/products/hive-core/products
```

## Product Identity

One backend does not mean one product.

Each product should keep:

- its name, accent, UI surface, and workflow language
- product-owned decision logic
- product-specific storage where it still makes sense
- product-specific safety boundaries
- standalone exportability until the suite backend is mature enough to replace that need

The suite backend should route, coordinate, and provide shared platform services. It should not turn every product into one blurry generic endpoint.

## Migration Path

This should be gradual.

1. Keep current product backends working.
2. Make HiveCore the one browser-facing backend for v2 frontends.
3. Add namespaced HiveCore proxy routes for existing product APIs.
4. Point product v2 frontends at HiveCore by default.
5. Move repeated auth, GitHub token, provider config, run history, scheduling, and guardrail logic into HiveCore or shared crates.
6. Extract product behavior into Rust modules/crates that can be mounted in the suite backend.
7. Retire individual product backends only after their product logic is safely available through the suite backend and standalone export needs are resolved.

This lets PatchHive get the UX benefits early without forcing a risky rewrite.

## Two Backend Modes

During migration, PatchHive can support two backend modes:

- **Gateway mode:** HiveCore proxies to existing product backends through product-owned contracts and service tokens.
- **Integrated mode:** HiveCore mounts product modules directly in one Rust process.

Gateway mode is the safer first step. Integrated mode can happen product by product when shared seams are stable.

## Frontend Rule

Product v2 frontends should eventually treat HiveCore as their default API base:

```text
VITE_API_URL=/api
```

They should not need to know whether the request is handled by a proxied product service or an in-process product module. That routing decision belongs to the suite backend.

## Safety Rules

- Browser code should never receive product service tokens.
- HiveCore should enforce operator auth before routing product requests.
- Product actions should still advertise capability metadata before HiveCore dispatches them.
- Destructive or write-capable actions need explicit approval flows.
- Product-specific risk decisions should stay inside the product module or product service.
- HiveCore should log dispatches and preserve enough evidence to debug cross-product runs.

## Open Questions

- Which products should be first to move from gateway mode to integrated mode?
- Should every product keep standalone export support after the suite backend is mature?
- How much product state should move into a shared suite database versus product-owned SQLite files?
- What route names should become permanent for the namespaced product API?
- How should HiveCore version and validate in-process product modules?

The direction is clear: one backend, many product faces.
