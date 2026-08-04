# Security, performance, and safety review — 2026-08-03

## Scope and baseline

This pass reviewed the monorepo at `f5c2d7c` after the canonical specialist UI
consolidation, plus the runtime, export, release, and documentation repairs in
the working tree. `main`, `HEAD`, and `origin/main` matched at the start of the
pass. The repairs described here are not an exact deployable revision until
they are reviewed and committed.

## Outcome

No known high-severity code vulnerability remains in the reviewed paths. The
unified runtime now authenticates every enabled mounted engine without copying
raw machine secrets into `.env` or HiveCore's database. Standalone products
remain independent and use explicit network configuration when they integrate.
Export and release tooling now fails closed when it cannot produce the artifact
it claims to publish.

The autonomous proof milestone is still gated on deployment evidence, not more
architecture work: commit/review the repairs, restart the managed backend from
that exact revision, prove the configured AI request and target-specific GitHub
write authority against the allowlisted sandbox, then run the no-write loop.

## Security findings and repairs

- **Unified peer authentication — repaired.** Mounted products previously had
  URLs but not a complete authenticated caller path. Every enabled engine now
  issues a random process-local scoped service credential at startup. Its target
  retains only a constant-time-verified hash; raw values live only in redacted
  runtime configuration and disappear on restart. Calls continue through normal
  HTTP middleware, scopes, rate limiting, and telemetry.
- **HiveCore fleet authentication — repaired.** HiveCore now consumes the same
  target-issued runtime credentials for snapshots, run reads, smoke, and
  dispatch. It does not require duplicate saved downstream tokens for mounted
  engines. An explicit external API URL still selects the durable standalone
  credential instead of sending an in-process credential to another host.
- **Export target validation — repaired.** Product, package, crate, template,
  branch, and remote selectors are validated before Git or filesystem use,
  preventing path-like selectors and ambiguous remote operands.
- **Release source ambiguity — repaired.** External release actions require a
  clean tracked and untracked worktree. `--allow-dirty` is limited to local
  smoke/pack work and cannot publish a different committed tree from the one
  validated.
- **Mirror overwrite risk — repaired.** Reset-style mirror sync uses an observed
  remote head plus `--force-with-lease`; blind force pushes were removed.
- **Dependency scan — clean.** `cargo audit` passed for all 21 checked Rust
  lockfiles. `npm audit --package-lock-only --audit-level=low` reported zero
  vulnerabilities for the root and all 12 standalone frontend lockfiles.

## Performance findings

- HiveCore refreshes product state concurrently and serves suite reads from
  materialized SQLite snapshots, avoiding request-time fleet fan-out.
- Runtime credentials add small in-memory hash-map and constant-time hash checks;
  they do not add database writes or a second network boundary.
- A clean debug-binary probe started the 12-engine runtime in about 5.5 seconds,
  used about 78 MiB RSS after startup, and returned suite health in about 2 ms.
  These are local diagnostic numbers, not production SLOs.
- Three five-second snapshot cycles completed successfully during the final
  probe. All 12 product snapshots existed and all 12 run observations were
  `observed`.

No performance regression requiring a code change was reproduced. Production
capacity still needs a release-build soak with realistic repositories, API
latency, database growth, and concurrent work before unattended operation.

## Safety findings and repairs

- The canonical autonomous cycle is documented in
  [autonomous-maintenance-loop.md](autonomous-maintenance-loop.md). Broad mandate
  discovery is planning evidence, not executable work; a concrete attributed
  finding must enter the durable ledger before execution.
- RepoReaper publication remains fail-closed on explicit TrustGate `safe`, test
  evidence, exact approval where required, pause authority, and the two-phase PR
  reservation protocol.
- FailGuard is documented as AI-first interpretation behind deterministic
  evidence, promotion, enforcement, audit, and rollback. Repository text is
  untrusted input and a closed-unmerged PR is not automatically a product
  failure.
- Export scripts now retain regenerated lockfiles in the actual subtree commit,
  use locked Rust builds and reproducible pinned container bases, and reject
  service exports whose monorepo path dependencies cannot stand alone.

## Remaining release and deployment gates

1. Review and commit this working tree; do not deploy an uncommitted build as if
   it were an exact reviewed SHA.
2. Replace the stale ad-hoc backend process with a managed restart from that
   committed revision and retain its startup/snapshot evidence.
3. Publish `@patchhivehq/ui@0.1.4`, then
   `@patchhivehq/product-shell@0.1.4`, before exporting specialist products that
   depend on those versions. No publish was triggered by this review.
4. Keep standalone service subtree export disabled until it has a portable
   dependency bundle; the script now rejects the currently invalid export.
5. Prove the selected AI provider/model with a bounded non-publishing request
   and prove each product-owned GitHub write identity on the allowlisted sandbox.
   Token presence and GitHub identity lookup are not target-specific authority.
6. Run the controlled no-write autonomous rehearsal before permitting exactly
   one attributed draft PR.

Optional webhook secrets, public URLs, and registry configuration remain
feature-specific. They must be reported honestly but are not blanket blockers
for a local sandbox proof.

## Verification evidence

- `cargo test --manifest-path crates/patchhive-product-core/Cargo.toml`: 121
  passed.
- Unified backend tests: 4 consolidation tests and 7 runtime tests passed.
- `cargo clippy --manifest-path services/patchhive-backend/Cargo.toml
  --all-targets -- -D warnings`: passed.
- `cargo build --locked --manifest-path
  services/patchhive-backend/Cargo.toml`: passed.
- `bash -n scripts/*.sh`, `git diff --check`, and
  `./scripts/check-suite-drift.sh`: passed.
- Crate and starter-template export rehearsals retained their refreshed locks
  in exported commits and left the source checkout clean.
- The suite release dry-run completed with external writes disabled.

`shellcheck` was not installed in the review environment; Bash syntax and the
repository's executable script rehearsals were used instead.
