# SQLite Connection Strategy

PatchHive product backends use `patchhive_product_core::sqlite::SqlitePool`, a
small synchronous connection pool around `rusqlite::Connection`. The pool keeps
SQLite as the local single-file database while avoiding one process-wide
`Mutex<Connection>` becoming the bottleneck for unrelated reads and writes.

## Current Shape

The shared pool:

- defaults to 4 connections
- enables WAL mode and foreign keys by default
- applies a 5 second SQLite busy timeout
- returns connections to the pool on drop
- can be tuned globally with `PATCHHIVE_DB_POOL_SIZE`
- can be tuned per product with `<PRODUCT>_DB_POOL_SIZE`
- exposes operator-facing helpers for DB path wording, backup guidance,
  migration guidance, and SQLite error classification

Products still keep schema creation and migrations in their own `db.rs` modules.
That keeps product data models local while making the connection behavior suite
wide.

## Design Goals

- avoid serializing all DB access behind a single global mutex
- keep migrations serialized during startup
- keep write transactions explicit and short
- expose product-owned helpers instead of raw global locks
- keep SQLite as the database; this is not a move to a server DB

## Guardrails

- Do not mix pooled and global-lock access inside the same product.
- Keep schema migration tests passing before and after each product migration.
- Preserve the current local-file env vars such as `HIVE_CORE_DB_PATH` and
  `REAPER_DB_PATH`.
- Keep pool size small by default, for example 4 connections, because SQLite
  still has one writer at a time.
- Startup checks should use `patchhive_product_core::sqlite::db_path_message`
  so every product tells operators to back up the database plus any matching
  `-wal` and `-shm` files before manual cleanup.
- When surfacing SQLite failures to users, use the shared operator helpers
  instead of raw `rusqlite` strings. Busy and locked errors should tell the
  operator to let active runs finish, close duplicate dev servers, or restart
  stale local processes. Cannot-open, read-only, disk, corrupt, and schema
  errors should keep their action wording consistent across products.

## Success Criteria

- UI health polling stays responsive while smoke/run evidence is being written.
- DB initialization and migration remain deterministic.
- Tests cover schema initialization and shared pool connection reuse.
- Browser-visible Checks tabs describe database paths, backup expectations, and
  common lock/permission/disk recovery steps in the same language everywhere.
