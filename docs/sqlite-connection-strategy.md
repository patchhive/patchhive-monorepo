# SQLite Connection Strategy

PatchHive currently opens one process-wide `rusqlite::Connection` per backend and
guards it behind `OnceCell<Mutex<Connection>>`. That is simple and safe for the
MVP, but it serializes every DB operation in each product.

## Current Tradeoff

The current pattern is acceptable while products mostly serve low-frequency UI
requests and short background jobs:

- one SQLite connection is easy to initialize and migrate
- all writes are naturally serialized
- poisoning is usually handled by returning an error instead of panicking

The downside is that long reads, run-history writes, or background job updates
can block unrelated requests behind the same mutex. RepoReaper and HiveCore are
the first likely products to feel this because they write run/action evidence
while the UI is also polling state.

## Target Shape

Move products to a small SQLite connection pool with product-owned helpers:

- use WAL mode for products with concurrent read/write pressure
- keep migrations serialized during startup
- keep write transactions explicit and short
- expose a `with_conn` / `with_tx` style helper instead of raw global locks
- keep SQLite as the database; this is not a move to a server DB

Recommended pool options:

- `r2d2` + `r2d2_sqlite` for synchronous `rusqlite` code
- `deadpool-sqlite` only if a product moves more DB work behind async-friendly
  boundaries

## Migration Order

1. HiveCore
   HiveCore now records launcher jobs, action events, service-token metadata,
   and smoke evidence while continuously polling product state. It is the best
   pilot because failures are visible but the schema is smaller than RepoReaper.

2. RepoReaper
   RepoReaper has the heaviest write path. Move it after the HiveCore helper is
   proven so autonomous run tracking does not compete with dashboard polling.

3. SignalHive
   SignalHive has the cleanest modular DB layout already, so it can adopt the
   helper with less churn after the pilot.

4. Starter-derived products
   ReviewBee, MergeKeeper, FlakeSting, DepTriage, VulnTriage, RefactorScout,
   TrustGate, and RepoMemory can migrate once the helper API is stable.

## Guardrails

- Do not mix pooled and global-lock access inside the same product.
- Keep schema migration tests passing before and after each product migration.
- Preserve the current local-file env vars such as `HIVE_CORE_DB_PATH` and
  `REAPER_DB_PATH`.
- Add a lightweight concurrent read/write smoke test to the first pooled product.
- Keep pool size small by default, for example 4 connections, because SQLite
  still has one writer at a time.

## Success Criteria

- UI health polling stays responsive while smoke/run evidence is being written.
- DB initialization and migration remain deterministic.
- Tests cover schema initialization, one concurrent read/write path, and the
  poison/error behavior that the mutex currently protected.
